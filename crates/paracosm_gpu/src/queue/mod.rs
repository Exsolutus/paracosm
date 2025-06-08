pub mod commands;

use commands::Commands;
use crate::device::LogicalDevice;

use anyhow::Result;
use bevy_ecs::{
    schedule::{ExecutorKind, IntoSystemConfigs, IntoSystemSetConfigs, Schedule, ScheduleLabel, SystemSet}, 
    system::ResMut, 
    world::World
};


#[derive(ScheduleLabel, Hash, Eq, PartialEq, Clone, Copy, Debug, Default)]
pub enum Queue {
    #[default] Graphics,
    Compute,
}

#[derive(SystemSet, PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub(crate) struct SubmitSet(pub u32);

#[derive(Default)]
pub struct SubmitInfo {
    pub wait: Box<[(crate::queue::Queue, u32)]>,
    pub signal: Box<[u32]>
}

#[derive(Default)]
struct Submit {
    set: SubmitSet,
    info: SubmitInfo,
    buffer: ash::vk::CommandBuffer
}


pub(crate) struct QueueGraph {
    schedule: Schedule,
    submits: Vec<Submit>,
    commands: Commands,
    command_pool: ash::vk::CommandPool,
    queue_family: u32,
    timeline: ash::vk::Semaphore,
}   

impl QueueGraph {
    pub fn new(
        device: &LogicalDevice,
        label: Queue,
        queue_family: u32,
        pipeline_layout: ash::vk::PipelineLayout,
        descriptor_set: ash::vk::DescriptorSet
    ) -> Result<Self> {
        let mut schedule = Schedule::new(label);
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        // Create queue submit timeline
        let mut semaphore_type_create_info = ash::vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(ash::vk::SemaphoreType::TIMELINE);
        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::default()
            .push_next(&mut semaphore_type_create_info);
        let timeline = unsafe { device.create_semaphore(&semaphore_create_info, None)? };

        // Create command pool for queue family 
        let command_pool_create_info = ash::vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family);
        let command_pool = unsafe {
            device.create_command_pool(&command_pool_create_info, None)?
        };
                
        // Allocate command buffer for first submit set
        let buffer = unsafe {
            device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .command_buffer_count(1)
            )?[0]
        };

        let commands = Commands::new(device, label, descriptor_set, pipeline_layout)?;

        Ok(Self {
            schedule,
            submits: vec![Submit { buffer, ..Default::default() }].into(),
            commands,
            command_pool,
            queue_family,
            timeline,
        })
    }

    pub fn add_nodes<M>(&mut self, nodes: impl IntoSystemConfigs<M>) {
        // TODO: validate node signatures?

        let current_set = self.submits.last().unwrap().set;
        self.schedule.add_systems(
            nodes.in_set(current_set)
        );
    }

    /// Set current submit info and add next submit set
    pub fn add_submit(&mut self, info: SubmitInfo) -> Result<u32> {
        let device = self.commands.device();

        // Update current submit
        let current_submit = self.submits.last_mut().unwrap();
        current_submit.info = info;

        let current_set = current_submit.set;

        // Allocate command buffer for next submit set
        let buffer = unsafe {
            device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.command_pool)
                    .command_buffer_count(1)
            )?[0]
        };

        // Initialize next submit
        let next_set = SubmitSet(current_set.0 + 1);
        let mut next_submit = Submit { buffer, ..Default::default() };
        next_submit.set = next_set;

        // Add submit system after current submit set and before next submit set
        {
            let queue_family = self.queue_family;       // 
            let current_buffer = current_submit.buffer; // expose for capture in submit closure
            let next_buffer = next_submit.buffer;       // 

            let submit = (move |commands: ResMut<Commands>| {
                let device = commands.device();
                
                // Submit current command buffer
                unsafe { device.end_command_buffer(current_buffer).unwrap(); }
        
                let command_buffer_info = ash::vk::CommandBufferSubmitInfo::default()
                    .command_buffer(current_buffer);
                let submit_info = ash::vk::SubmitInfo2::default()
                    .command_buffer_infos(std::slice::from_ref(&command_buffer_info));
                    // TODO: Signal and wait semaphores

                unsafe {
                    let queue = device.get_device_queue(queue_family, 0); // TODO: dynamically select compute queue index?
                    device.queue_submit2(queue, std::slice::from_ref(&submit_info), ash::vk::Fence::null()).unwrap();
                }

                // Initialize next command buffer
                commands.init_command_buffer(next_buffer).unwrap();
            }).after(current_set).before(next_set);
            self.schedule.add_systems(submit);
        }
            
        // Add next submit after current submit
        self.submits.push(next_submit);
        self.schedule.configure_sets(next_set.after(current_set));

        Ok(next_set.0)
    }

    pub fn run(&mut self, world: &mut World, rebuild: bool) -> Result<()> {
        if rebuild {
            world.insert_resource(self.commands.clone());

            // Initialize first submit command buffer
            self.commands.init_command_buffer(self.submits[0].buffer)?;
    
            // Run schedule to record and submit commands
            self.schedule.run(world);        
        } else {
            for submit in self.submits.iter() {
                let command_buffer_info = ash::vk::CommandBufferSubmitInfo::default()
                    .command_buffer(submit.buffer);
                let submit_info = ash::vk::SubmitInfo2::default()
                    .command_buffer_infos(std::slice::from_ref(&command_buffer_info));
                    // TODO: Signal and wait semaphores

                unsafe {
                    let device = self.commands.device();
                    
                    let queue = device.get_device_queue(self.queue_family, 0); // TODO: dynamically select compute queue index?
                    device.queue_submit2(queue, std::slice::from_ref(&submit_info), ash::vk::Fence::null()).unwrap();
                }
            }
        }

        Ok(())
    }
}

impl Drop for QueueGraph {
    fn drop(&mut self) {
        unsafe {
            let device = self.commands.device();

            device.destroy_semaphore(self.timeline, None);
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}


impl crate::context::Context {
    pub fn add_nodes<M>(&mut self, queue: Queue, nodes: impl IntoSystemConfigs<M>) -> Result<&mut Self> {
        let device = &mut self.devices[self.configuring_device as usize];
        device.dirty = true;
        
        // Add nodes to queue graph in latest submit set
        match queue {
            Queue::Compute => device.compute_graph.add_nodes(nodes),
            Queue::Graphics => device.graphics_graph.add_nodes(nodes),
        }

        Ok(self)
    }

    pub fn add_submit(&mut self, queue: Queue, info: SubmitInfo) -> Result<u32> {
        let device = &mut self.devices[self.configuring_device as usize];
        device.dirty = true;

        // Get per-queue access
        let queue_graph = match queue {
            Queue::Compute => &mut device.compute_graph,
            Queue::Graphics => &mut device.graphics_graph
        };

        queue_graph.add_submit(info)
    }
}