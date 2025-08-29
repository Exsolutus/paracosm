pub mod commands;

use std::u64;

use commands::Commands;
use crate::{device::LogicalDevice, resource::ResourceManager};

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

#[derive(SystemSet, PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
struct SubmitSet(pub u32);

pub(crate) struct QueueGraph {
    device: *const LogicalDevice,
    queue_label: Queue,
    pub queue: ash::vk::Queue,
    descriptor_set: ash::vk::DescriptorSet,
    pipeline_layout: ash::vk::PipelineLayout,
    pub command_pool: ash::vk::CommandPool,
    pub immediate_command_buffer: ash::vk::CommandBuffer,
    schedule: Schedule,

    // Sync
    frame_number: u64,
    queue_timeline: ash::vk::Semaphore,

    // Submissions
    command_buffers: [Vec<ash::vk::CommandBuffer>; 2],
    queue_waits: Vec<Option<(ash::vk::Semaphore, u32)>>,
    // signal: Vec<[ash::vk::Semaphore]>,

    open_submit: bool,
    locked: bool
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
            .queue_family_index(queue_family)
            .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe {
            device.create_command_pool(&command_pool_create_info, None)?
        };
        let immediate_command_buffer = unsafe { 
            device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .command_buffer_count(1)
            )?[0]
        };

        let mut schedule = Schedule::new(queue_label);
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        let mut semaphore_type_create_info = ash::vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(ash::vk::SemaphoreType::TIMELINE);
        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::default()
            .push_next(&mut semaphore_type_create_info);
        let queue_timeline = unsafe { device.create_semaphore(&semaphore_create_info, None)? };

        Ok(Self {
            device,
            queue_label,
            queue,
            descriptor_set,
            pipeline_layout,
            command_pool,
            immediate_command_buffer,
            schedule,
            frame_number: 0,
            queue_timeline,
            command_buffers: Default::default(),
            queue_waits: Default::default(),
            open_submit: false,
            locked: false
        })
    }

    pub fn add_nodes<M>(&mut self, nodes: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Result<()> {
        if self.locked {
            bail!("Cannot add nodes to a queue that has already been run.")
        }

        // TODO: validate node signatures?

        if !self.open_submit {
            self.open_submit = true;
        }

        let current_set = SubmitSet(self.command_buffers.len() as u32);
        self.schedule.add_systems(
            nodes.in_set(current_set)
        );

        Ok(())
    }

    /// Set current submit info and add next submit set
    pub fn add_submit(
        &mut self,
        wait: Option<(ash::vk::Semaphore, u32)>
    ) -> Result<u32> {
        if !self.open_submit {
            bail!("Queue has no unsubmitted nodes.")
        }

        let device = unsafe { self.device.as_ref().unwrap() };

        // Configure current submit set
        let index = self.command_buffers.len() as u32;
        self.schedule.configure_sets(SubmitSet(index).before(SubmitSet(index + 1)));

        // Allocate command buffers for submit set
        let command_buffers = unsafe { 
            device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.command_pool)
                    .command_buffer_count(2)
            )?
        };
        self.command_buffers[0].push(command_buffers[0]);
        self.command_buffers[1].push(command_buffers[1]);

        self.queue_waits.push(wait);

        // Add command buffer increment system after last submit set and before current submit set
        let mut swap_command_buffer = (move |mut commands: ResMut<Commands>| {
            commands.index += 1;
        }).into_configs();
        swap_command_buffer = swap_command_buffer.after(SubmitSet(index)).before(SubmitSet(index + 1));
        
        self.schedule.add_systems(swap_command_buffer);

        self.open_submit = false;

        Ok(index)
    }

    pub fn run(&mut self, world: &mut World) -> Result<()> {
        let device = unsafe { self.device.as_ref().unwrap() };

        if self.open_submit {
            bail!("Queue has unsubmitted nodes.")
        }

        if !self.command_buffers[0].is_empty() {
            let frame_index = (self.frame_number % 2) as usize;
            let submit_count = self.command_buffers[0].len();

            // Wait on frames in flight
            if self.frame_number >= 2 {
                unsafe { device.wait_semaphores(
                    &ash::vk::SemaphoreWaitInfo::default()
                        .semaphores(&[self.queue_timeline])
                        .values(&[((self.frame_number - 1) * submit_count as u64)]), 
                    u64::MAX)?
                }
            };

            // Initialize submit command buffers
            for command_buffer in self.command_buffers[frame_index].iter() {
                self.init_command_buffer(*command_buffer)?;
            }

            world.insert_resource(Commands::new(device, self.command_buffers[frame_index].clone().into()));

            // Acquire next surface images
            let mut acquire_semaphores = vec![]; 
            let mut submit_semaphores = vec![]; 
            if self.queue_label == Queue::Graphics {
                let mut resource_manager = world.resource_mut::<ResourceManager>();
                let mut image_barriers = vec![];

                for surface in resource_manager.surfaces.iter_mut() {
                    let (barrier, acquire_semaphore, submit_semaphore) = surface.acquire()?;
                    
                    image_barriers.push(barrier);
                    acquire_semaphores.push(ash::vk::SemaphoreSubmitInfo::default().semaphore(acquire_semaphore));
                    submit_semaphores.push(ash::vk::SemaphoreSubmitInfo::default().semaphore(submit_semaphore));
                }

                unsafe { device.cmd_pipeline_barrier2(
                    self.command_buffers[frame_index][0], 
                    &ash::vk::DependencyInfo::default()
                        .image_memory_barriers(&image_barriers)
                ) };
            }

            // Run schedule to record commands
            self.schedule.run(world);

            // Submit all recorded commands
            for (submit_index, command_buffer) in self.command_buffers[frame_index].iter().enumerate() {
                let mut wait_semaphore_infos: Vec<ash::vk::SemaphoreSubmitInfo<'_>> = self.queue_waits.iter()
                    .filter_map(|wait| match wait {
                        Some((semaphore, value)) => Some(
                            ash::vk::SemaphoreSubmitInfo::default()
                                .semaphore(*semaphore)
                                .value(*value as u64)
                        ),
                        None => None
                    }).collect();
                let mut signal_semaphore_infos: Vec<ash::vk::SemaphoreSubmitInfo<'_>> = vec![];

                if submit_index == 0 {
                    wait_semaphore_infos.append(&mut acquire_semaphores);
                }
                if submit_index == submit_count - 1 {
                    signal_semaphore_infos.append(&mut submit_semaphores);
            
                    let mut image_barriers = vec![];
                    let mut resource_manager = world.resource_mut::<ResourceManager>();
                    for surface in resource_manager.surfaces.iter_mut() {
                        image_barriers.push(surface.finish()?);
                    }
                    
                    unsafe { device.cmd_pipeline_barrier2(
                        *command_buffer, 
                        &ash::vk::DependencyInfo::default()
                            .image_memory_barriers(&image_barriers)
                    ) };
                }

                let timeline_value = (self.frame_number * submit_count as u64) + submit_index  as u64;
                wait_semaphore_infos.push(
                    ash::vk::SemaphoreSubmitInfo::default()
                        .semaphore(self.queue_timeline)
                        .value(timeline_value)
                );
                signal_semaphore_infos.push(
                    ash::vk::SemaphoreSubmitInfo::default()
                        .semaphore(self.queue_timeline)
                        .value(timeline_value + 1)
                );

                unsafe { device.end_command_buffer(*command_buffer)? };

                let command_buffer_info = ash::vk::CommandBufferSubmitInfo::default()
                    .command_buffer(*command_buffer);
                let submit_info = ash::vk::SubmitInfo2::default()
                    .command_buffer_infos(std::slice::from_ref(&command_buffer_info))
                    .wait_semaphore_infos(wait_semaphore_infos.as_slice())
                    .signal_semaphore_infos(signal_semaphore_infos.as_slice());

                unsafe { device.queue_submit2(self.queue, std::slice::from_ref(&submit_info), ash::vk::Fence::null()).unwrap(); }

                let mut resource_manager = world.resource_mut::<ResourceManager>();
                for surface in resource_manager.surfaces.iter_mut() {
                    surface.present(self.queue)?;
                }
            }
        }

        self.locked = true;
        self.frame_number += 1;

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

            device.destroy_semaphore(self.queue_timeline, None);
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

    pub fn add_submit(&mut self, queue: Queue, wait: Option<(Queue, u32)>) -> Result<u32> {
        let device = &mut self.devices[self.configuring_device as usize];

        let wait = match wait {
            Some((Queue::Compute, value)) => Some((device.compute_graph.queue_timeline, value)),
            Some((Queue::Graphics, value)) => Some((device.graphics_graph.queue_timeline, value)),
            None => None
        };

        // Get per-queue access
        match queue {
            Queue::Compute => device.compute_graph.add_submit(wait),
            Queue::Graphics => device.graphics_graph.add_submit(wait)
        }
    }

    pub fn clear_queue(&mut self, queue: Queue) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];

        let graph = match queue {
            Queue::Compute => &mut device.compute_graph,
            Queue::Graphics => &mut device.graphics_graph
        };
        unsafe { device.logical_device.reset_command_pool(graph.command_pool, ash::vk::CommandPoolResetFlags::empty())? };
        graph.command_buffers = Default::default();

        Ok(())
    }
}