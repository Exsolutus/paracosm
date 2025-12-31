pub mod compute;
pub mod graphics;
pub mod transfer;

use crate::{
    device::LogicalDevice,
    pipeline::{Pipeline, PipelineInfo, PipelineLabel}
};

use anyhow::{bail, Ok, Result};
use bevy_ecs::resource::Resource;



#[derive(Resource)]
pub(crate) struct Commands {
    device: *const LogicalDevice,
    pub index: u32,
    pub command_buffers: Box<[ash::vk::CommandBuffer]>,
}
// SAFETY: Valid so long as mutable access to Commands is only exposed through the Context
unsafe impl Send for Commands {  }
unsafe impl Sync for Commands {  }   


impl Commands {
    pub fn new(
        device: &LogicalDevice,
        command_buffers: Box<[ash::vk::CommandBuffer]>
    ) -> Self {
        Self {
            device,
            index: 0,
            command_buffers
        }
    }

    pub fn device(&self) -> &LogicalDevice {
        unsafe { self.device.as_ref().unwrap() }
    }

    pub fn command_buffer(&self) -> ash::vk::CommandBuffer {
        self.command_buffers[self.index as usize]
    }
}


pub(crate) trait CommandRecorder {
    fn device(&self) -> &LogicalDevice;
    fn command_buffer(&self) -> ash::vk::CommandBuffer;
    fn pipeline<L: PipelineLabel + 'static>(&self, label: L) -> Result<&Pipeline>;
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32);
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
            let device = self.device();
            let command_buffer = self.command_buffer();

            device.cmd_bind_pipeline(command_buffer, bind_point, **pipeline);
        }

        Ok(())
    }

    fn set_push_constant<T>(&mut self, push_constant: T) -> Result<()> {
        unsafe {
            let device = self.device();
            let command_buffer = self.command_buffer();

            let (pipeline_layout, max_push_constant_size) = self.pipeline_constants();

            if size_of::<T>() > max_push_constant_size as usize {
                bail!("Push constant should be no larger than {} bytes.", max_push_constant_size)
            }

            let data = std::slice::from_raw_parts(
                &push_constant as *const T as *const u8,
                size_of::<T>()
            );

            device.cmd_push_constants(command_buffer, pipeline_layout, ash::vk::ShaderStageFlags::ALL, 0, data);
        }

        Ok(())
    }
}