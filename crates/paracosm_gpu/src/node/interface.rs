use crate::{
    device::LogicalDevice,
    pipeline::{Pipeline, PipelineLabel, PipelineManager},
    queue::commands::{CommandRecorder, Commands, CommonCommands, compute::ComputeCommands, graphics::GraphicsCommands, transfer::TransferCommands},
};

use anyhow::Result;
use bevy_ecs::{system::{Res, ResMut, SystemParam}};

use std::marker::PhantomData;


// Command interface parameters
#[derive(SystemParam)]
pub struct ComputeInterface<'w, 's> {
    inner: NodeInterface<'w, 's>,
    pipelines: Res<'w, PipelineManager>,
}

#[derive(SystemParam)]
pub struct GraphicsInterface<'w, 's> {
    inner: NodeInterface<'w, 's>,
    pipelines: Res<'w, PipelineManager>,
}

#[derive(SystemParam)]
pub struct TransferInterface<'w, 's> {
    inner: NodeInterface<'w, 's>,
}

#[derive(SystemParam)]
struct NodeInterface<'w, 's> {
    commands: ResMut<'w, Commands>,
    _marker: PhantomData<&'s ()>
}

// TODO: manually implement SystemParam

impl<'w, 's> CommandRecorder for ComputeInterface<'w, 's> {
    fn device(&self) -> &LogicalDevice { &self.inner.commands.device() }
    fn command_buffer(&self) -> ash::vk::CommandBuffer { self.inner.commands.command_buffer() }
    fn pipeline<L: PipelineLabel + 'static>(&self, label: L) -> Result<&Pipeline> { self.pipelines.get(label) }
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32) { (self.pipelines.pipeline_layout, self.pipelines.max_push_constants_size) }
}

impl<'w, 's> CommandRecorder for GraphicsInterface<'w, 's> {
    fn device(&self) -> &LogicalDevice { &self.inner.commands.device() }
    fn command_buffer(&self) -> ash::vk::CommandBuffer { self.inner.commands.command_buffer() }
    fn pipeline<L: PipelineLabel + 'static>(&self, label: L) -> Result<&Pipeline> { self.pipelines.get(label) }
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32) { (self.pipelines.pipeline_layout, self.pipelines.max_push_constants_size) }
}

impl<'w, 's> CommandRecorder for TransferInterface<'w, 's> {
    fn device(&self) -> &LogicalDevice { &self.inner.commands.device() }
    fn command_buffer(&self) -> ash::vk::CommandBuffer { self.inner.commands.command_buffer() }
    fn pipeline<L: PipelineLabel + 'static>(&self, _label: L) -> Result<&Pipeline> {
        panic!("This should never panic. Notify library maintainers.")  // Pipelines are not supported on Transfer queues
    }
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32) {
        panic!("This should never panic. Notify library maintainers.")  // Pipelines are not supported on Transfer queues
    }
}

impl<'w, 's> CommonCommands for ComputeInterface<'w, 's> {  }
impl<'w, 's> CommonCommands for GraphicsInterface<'w, 's> {  }

impl<'w, 's> ComputeCommands for ComputeInterface<'w, 's> {  }
impl<'w, 's> ComputeCommands for GraphicsInterface<'w, 's> {  }

impl<'w, 's> GraphicsCommands for GraphicsInterface<'w, 's> {  }

impl<'w, 's> TransferCommands for ComputeInterface<'w, 's> {  }
impl<'w, 's> TransferCommands for GraphicsInterface<'w, 's> {  }
impl<'w, 's> TransferCommands for TransferInterface<'w, 's> {  }
