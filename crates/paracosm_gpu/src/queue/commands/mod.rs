pub mod compute;
pub mod graphics;
pub mod transfer;

use crate::{
    device::LogicalDevice, 
    node::resource::ResourceIndex, 
    pipeline::{Pipeline, PipelineInfo, PipelineLabel}, 
    resource::ResourceLabel
};

use anyhow::Result;
use bevy_ecs::prelude::Resource;

use std::cell::UnsafeCell;


#[derive(Resource)]
pub(crate) struct Commands {
    device: *const LogicalDevice,
    pub command_buffer: UnsafeCell<ash::vk::CommandBuffer>,
}
unsafe impl Send for Commands {  }   // HACK: safe while graph execution is single threaded?
unsafe impl Sync for Commands {  }     


impl Commands {
    pub fn new(
        device: &LogicalDevice,
        command_buffer: ash::vk::CommandBuffer
    ) -> Self {
        Self {
            device,
            command_buffer: command_buffer.into()
        }
    }

    pub fn device(&self) -> &LogicalDevice {
        unsafe { self.device.as_ref().unwrap() }
    }

    pub fn command_buffer(&self) -> Result<ash::vk::CommandBuffer> {
        let buffer = unsafe { self.command_buffer.get().as_ref().unwrap() };

        Ok(*buffer)
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