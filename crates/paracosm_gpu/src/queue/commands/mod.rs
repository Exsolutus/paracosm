pub mod compute;
pub mod graphics;
pub mod transfer;

use crate::{
    device::LogicalDevice, 
    node::resource::ResourceIndex, 
    pipeline::{Pipeline, PipelineInfo, PipelineLabel}, 
    queue::Queue, resource::ResourceLabel
};

use anyhow::Result;
use bevy_ecs::system::Resource;

use std::{cell::UnsafeCell, sync::Arc};


#[derive(Resource, Clone)]
pub(crate) struct Commands {
    inner: Arc<CommandsInner>
}

pub(crate) struct CommandsInner {
    device: *const LogicalDevice,
    queue: Queue,
    descriptor_set: ash::vk::DescriptorSet,
    pipeline_layout: ash::vk::PipelineLayout,
    current_command_buffer: UnsafeCell<ash::vk::CommandBuffer>,
}
unsafe impl Send for CommandsInner {  }   // HACK: safe while graph execution is single threaded
unsafe impl Sync for CommandsInner {  }     

impl std::ops::Deref for Commands {
    type Target = CommandsInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Commands {
    pub fn new(
        device: &LogicalDevice,
        queue: Queue,
        descriptor_set: ash::vk::DescriptorSet, 
        pipeline_layout: ash::vk::PipelineLayout
    ) -> Result<Self> {
        Ok(Self { inner: Arc::new(
            CommandsInner {
                device,
                queue,
                descriptor_set,
                pipeline_layout,
                current_command_buffer: ash::vk::CommandBuffer::null().into()
            }
        ) })
    }

    pub fn device(&self) -> &LogicalDevice {
        unsafe { self.device.as_ref().unwrap() }
    }

    pub fn current_command_buffer(&self) -> Result<ash::vk::CommandBuffer> {
        let buffer = unsafe { self.current_command_buffer.get().as_mut().unwrap() };

        Ok(*buffer)
    }

    pub fn init_command_buffer(&self, buffer: ash::vk::CommandBuffer) -> Result<()> {
        let device = self.device();

        unsafe {
            device.begin_command_buffer(buffer, &ash::vk::CommandBufferBeginInfo::default())?;

            device.cmd_bind_descriptor_sets(
                buffer, 
                ash::vk::PipelineBindPoint::COMPUTE, 
                self.pipeline_layout, 
                0, 
                std::slice::from_ref(&self.descriptor_set), 
                &[]
            );
            if self.queue == Queue::Graphics {
                device.cmd_bind_descriptor_sets(
                    buffer, 
                    ash::vk::PipelineBindPoint::GRAPHICS, 
                    self.pipeline_layout, 
                    0, 
                    std::slice::from_ref(&self.descriptor_set), 
                    &[]
                );
            }
            device.cmd_bind_descriptor_sets(
                buffer, 
                ash::vk::PipelineBindPoint::RAY_TRACING_KHR, 
                self.pipeline_layout, 
                0, 
                std::slice::from_ref(&self.descriptor_set), 
                &[]
            );

            *self.current_command_buffer.get().as_mut().unwrap() = buffer;
        }

        Ok(())
    }
}


pub(crate) trait CommandRecorder {
    fn device(&self) -> &ash::Device;
    fn command_buffer(&self) -> Result<ash::vk::CommandBuffer>;
    fn pipeline<L: PipelineLabel + 'static>(&self, label: L) -> Result<&Pipeline>;
    fn resource<L: ResourceLabel + 'static>(&self, label: L) -> Result<&ResourceIndex<L>>;
}

#[allow(private_bounds)]
pub trait CommonCommands: CommandRecorder {
    fn bind_pipeline<P: PipelineLabel + 'static>(&mut self, label: P) -> Result<()> {
        let pipeline = self.pipeline(label)?;
        let bind_point = match pipeline.info {
            PipelineInfo::Compute { .. } => ash::vk::PipelineBindPoint::COMPUTE,
            PipelineInfo::Graphics { .. } => ash::vk::PipelineBindPoint::GRAPHICS,
            PipelineInfo::RayTracing { .. } => ash::vk::PipelineBindPoint::RAY_TRACING_KHR,
        };

        unsafe {
            let command_buffer = self.command_buffer()?;
            let device = self.device();

            device.cmd_bind_pipeline(command_buffer, bind_point, **pipeline);
        }

        Ok(())
    }
}