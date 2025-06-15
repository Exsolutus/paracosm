pub mod commands;

use ash::vk::SemaphoreSubmitInfo;
use commands::Commands;
use crate::device::LogicalDevice;

use anyhow::{bail, Result};
use bevy_ecs::{
    schedule::{ExecutorKind, IntoScheduleConfigs, Schedule, ScheduleLabel, SystemSet}, 
    system::{ResMut, ScheduleSystem}, 
    world::World
};


#[derive(ScheduleLabel, Hash, Eq, PartialEq, Clone, Copy, Debug, Default)]
pub enum Queue {
    #[default] Graphics,
    Compute,
}

#[derive(Default)]
pub struct SubmitInfo {
    pub wait: Box<[(crate::queue::Queue, u32)]>,
    pub signal: Box<[u32]>
}

#[derive(SystemSet, PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
struct SubmitID(pub u32);

#[derive(Default)]
struct SubmitSet {
    id: SubmitID,
    command_buffer: ash::vk::CommandBuffer,
    wait: Box<[(ash::vk::Semaphore, u32)]>,
    signal: Box<[u32]>
}


pub(crate) struct QueueGraph {
    device: *const LogicalDevice,
    queue_label: Queue,
    queue: ash::vk::Queue,
    descriptor_set: ash::vk::DescriptorSet,
    pipeline_layout: ash::vk::PipelineLayout,
    command_pool: ash::vk::CommandPool,
    schedule: Schedule,
    submit_sets: Vec<SubmitSet>,
    timeline: ash::vk::Semaphore,

    open_submit: bool,
    dirty: bool,
}   

impl QueueGraph {
    pub fn new(
        device: &LogicalDevice,
        queue_label: Queue,
        queue_family: u32,
        pipeline_layout: ash::vk::PipelineLayout,
        descriptor_set: ash::vk::DescriptorSet
    ) -> Result<Self> {
        let queue = unsafe { device.get_device_queue(queue_family, 0) };

        let command_pool_create_info = ash::vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family);
        let command_pool = unsafe {
            device.create_command_pool(&command_pool_create_info, None)?
        };

        let mut schedule = Schedule::new(queue_label);
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        let mut semaphore_type_create_info = ash::vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(ash::vk::SemaphoreType::TIMELINE);
        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::default()
            .push_next(&mut semaphore_type_create_info);
        let timeline = unsafe { device.create_semaphore(&semaphore_create_info, None)? };

        Ok(Self {
            device,
            queue_label,
            queue,
            descriptor_set,
            pipeline_layout,
            command_pool,
            schedule,
            submit_sets: vec![],
            timeline,
            open_submit: true,
            dirty: false,
        })
    }

    pub fn add_nodes<M>(&mut self, nodes: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Result<()> {
        // TODO: validate node signatures?

        if !self.open_submit {
            self.open_submit = true;
        }

        //     let device = self.commands.device();

        //     // Allocate command buffer for new submit set
        //     let buffer = unsafe {
        //         device.allocate_command_buffers(
        //             &ash::vk::CommandBufferAllocateInfo::default()
        //                 .command_pool(self.command_pool)
        //                 .command_buffer_count(1)
        //         )?[0]
        //     };

        //     // Initialize new submit
        //     let next_set = SubmitID(current_set.0 + 1);
        //     let mut next_submit = SubmitSet { buffer, ..Default::default() };
        //     next_submit.id = next_set;

        //     // Add next submit after current submit
        //     self.submit_sets.push(next_submit);
        //     self.schedule.configure_sets(next_set.after(current_set));
        // }

        let current_set = SubmitID(self.submit_sets.len() as u32);
        self.schedule.add_systems(
            nodes.in_set(current_set)
        );

        self.dirty = true;

        Ok(())
    }

    /// Set current submit info and add next submit set
    pub fn add_submit(
        &mut self,
        wait: Box<[(ash::vk::Semaphore, u32)]>,
        signal: Box<[u32]>
    ) -> Result<u32> {
        if !self.open_submit {
            bail!("No nodes added since last submit.")
        }

        let device = unsafe { self.device.as_ref().unwrap() };
        
        // Allocate command buffer for submit set
        let command_buffer = unsafe { 
            device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.command_pool)
                    .command_buffer_count(1)
            )?[0]
        };

        // Finalize current submit set
        let index = self.submit_sets.len() as u32;
        let submit_set = SubmitSet {
            id: SubmitID(index),
            command_buffer,
            wait,
            signal
        };
        let previous_submit_set = self.submit_sets.last();
        if let Some(set) = previous_submit_set {
            submit_set.id.after(set.id);
        }

        // Add command buffer swap system after last submit set and before current submit set
        let mut swap_command_buffer = (move |commands: ResMut<Commands>| {
            unsafe { *commands.command_buffer.get() = command_buffer; }
        }).into_configs();
        swap_command_buffer = match previous_submit_set {
            Some(set) => swap_command_buffer.before(submit_set.id).after(set.id),
            None => swap_command_buffer.before(submit_set.id)
        };
        self.schedule.add_systems(swap_command_buffer);

        self.schedule.configure_sets(submit_set.id);
        self.submit_sets.push(submit_set);

        self.open_submit = false;

        Ok(index)
    }

    pub fn run(&mut self, world: &mut World) -> Result<()> {
        let device = unsafe { self.device.as_ref().unwrap() };

        if self.dirty {
            // Initialize submit command buffers
            for SubmitSet { command_buffer, .. } in self.submit_sets.iter() {
                self.init_command_buffer(*command_buffer)?;
            }

            world.insert_resource(Commands::new(device, self.submit_sets[0].command_buffer));
    
            // Run schedule to record commands
            self.schedule.run(world);

            self.dirty = false;
        }

        // Submit all recorded commands
        for submit in self.submit_sets.iter() {
            let signal_semaphore_infos: Vec<SemaphoreSubmitInfo<'_>> = submit.signal.iter().map(|&value| {
                SemaphoreSubmitInfo::default()
                    .semaphore(self.timeline)
                    .value(value as u64)
            }).collect();
            let wait_semaphore_infos: Vec<SemaphoreSubmitInfo<'_>> = submit.wait.iter().map(|(semaphore, value)| {
                SemaphoreSubmitInfo::default()
                    .semaphore(*semaphore)
                    .value(*value as u64)
            }).collect();

            unsafe { device.end_command_buffer(submit.command_buffer)? };

            let command_buffer_info = ash::vk::CommandBufferSubmitInfo::default()
                .command_buffer(submit.command_buffer);
            let submit_info = ash::vk::SubmitInfo2::default()
                .command_buffer_infos(std::slice::from_ref(&command_buffer_info))
                .signal_semaphore_infos(signal_semaphore_infos.as_slice())
                .wait_semaphore_infos(wait_semaphore_infos.as_slice());

            unsafe { device.queue_submit2(self.queue, std::slice::from_ref(&submit_info), ash::vk::Fence::null()).unwrap(); }
        }

        Ok(())
    }

    fn init_command_buffer(&self, command_buffer: ash::vk::CommandBuffer) -> Result<()> {
        unsafe {
            let device = self.device.as_ref().unwrap();

            device.begin_command_buffer(command_buffer, &ash::vk::CommandBufferBeginInfo::default())?;

            device.cmd_bind_descriptor_sets(
                command_buffer, 
                ash::vk::PipelineBindPoint::COMPUTE, 
                self.pipeline_layout, 
                0, 
                std::slice::from_ref(&self.descriptor_set), 
                &[]
            );
            if self.queue_label == Queue::Graphics {
                device.cmd_bind_descriptor_sets(
                    command_buffer, 
                    ash::vk::PipelineBindPoint::GRAPHICS, 
                    self.pipeline_layout, 
                    0, 
                    std::slice::from_ref(&self.descriptor_set), 
                    &[]
                );
            }
            device.cmd_bind_descriptor_sets(
                command_buffer, 
                ash::vk::PipelineBindPoint::RAY_TRACING_KHR, 
                self.pipeline_layout, 
                0, 
                std::slice::from_ref(&self.descriptor_set), 
                &[]
            );

            Ok(())
        }
    }
}

impl Drop for QueueGraph {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_ref().unwrap();

            device.destroy_semaphore(self.timeline, None);
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}


impl crate::context::Context {
    pub fn add_nodes<M>(&mut self, queue: Queue, nodes: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Result<&mut Self> {
        let device = &mut self.devices[self.configuring_device as usize];
        
        // Add nodes to queue graph in latest submit set
        match queue {
            Queue::Compute => device.compute_graph.add_nodes::<M>(nodes)?,
            Queue::Graphics => device.graphics_graph.add_nodes::<M>(nodes)?,
        }

        Ok(self)
    }

    pub fn add_submit(&mut self, queue: Queue, info: SubmitInfo) -> Result<u32> {
        let device = &mut self.devices[self.configuring_device as usize];

        let wait = info.wait.iter().map(|(queue, value)| {
            match queue {
                Queue::Compute => (device.compute_graph.timeline, *value),
                Queue::Graphics => (device.graphics_graph.timeline, *value)
            }
        }).collect();

        // Get per-queue access
        match queue {
            Queue::Compute => device.compute_graph.add_submit(wait, info.signal),
            Queue::Graphics => device.graphics_graph.add_submit(wait, info.signal)
        }
    }
}