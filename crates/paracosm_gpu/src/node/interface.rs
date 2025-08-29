use crate::{
    pipeline::{Pipeline, PipelineLabel, PipelineManager},
    node::resource::ResourceIndex,
    queue::commands::{compute::ComputeCommands, graphics::GraphicsCommands, transfer::TransferCommands, CommandRecorder, Commands, CommonCommands}, 
    resource::{
        ResourceManager,
        buffer::BufferLabel,
        image::{Image, ImageLabel}
    }
};

use anyhow::{Context, Result};
use bevy_ecs::system::{Res, ResMut, SystemParam};

use std::{any::{type_name, TypeId}, marker::PhantomData};


// Command interface parameters
#[derive(SystemParam)]
pub struct ComputeInterface<'w, 's> {
    inner: NodeInterface<'w, 's, 'c'>,
    pipelines: Res<'w, PipelineManager>,
    resources: Res<'w, ResourceManager>,
}

#[derive(SystemParam)]
pub struct GraphicsInterface<'w, 's> {
    inner: NodeInterface<'w, 's, 'g'>,
    pipelines: Res<'w, PipelineManager>,
    resources: Res<'w, ResourceManager>,
}

#[derive(SystemParam)]
pub struct TransferInterface<'w, 's> {
    inner: NodeInterface<'w, 's, 't'>,
    resources: Res<'w, ResourceManager>,
}

#[derive(SystemParam)]
struct NodeInterface<'w, 's, const Q: char> {
    commands: ResMut<'w, Commands>,
    _marker: PhantomData<&'s ()>
}

// TODO: manually implement SystemParam


impl<'w, 's> CommandRecorder for ComputeInterface<'w, 's> {
    fn device(&self) -> &ash::Device { &self.inner.commands.device() }
    fn command_buffer(&self) -> ash::vk::CommandBuffer { self.inner.commands.command_buffer() }
    fn pipeline<L: PipelineLabel + 'static>(&self, label: L) -> Result<&Pipeline> { self.pipelines.get(label) }
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32) { (self.pipelines.pipeline_layout, self.pipelines.max_push_constants_size) }
    fn resources(&self) -> &ResourceManager { &self.resources }
}

impl<'w, 's> CommandRecorder for GraphicsInterface<'w, 's> {
    fn device(&self) -> &ash::Device { &self.inner.commands.device() }
    fn command_buffer(&self) -> ash::vk::CommandBuffer { self.inner.commands.command_buffer() }
    fn pipeline<L: PipelineLabel + 'static>(&self, label: L) -> Result<&Pipeline> { self.pipelines.get(label) }
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32) { (self.pipelines.pipeline_layout, self.pipelines.max_push_constants_size) }
    fn resources(&self) -> &ResourceManager { &self.resources }
}

impl<'w, 's> CommandRecorder for TransferInterface<'w, 's> {
    fn device(&self) -> &ash::Device { &self.inner.commands.device() }
    fn command_buffer(&self) -> ash::vk::CommandBuffer { self.inner.commands.command_buffer() }
    fn pipeline<L: PipelineLabel + 'static>(&self, _label: L) -> Result<&Pipeline> {
        panic!("This should never panic. Notify library maintainers.")  // Pipelines are not supported on Transfer queues
    }
    fn pipeline_constants(&self) -> (ash::vk::PipelineLayout, u32) {
        panic!("This should never panic. Notify library maintainers.")  // Pipelines are not supported on Transfer queues
    }
    fn resources(&self) -> &ResourceManager { &self.resources }
}

impl<'w, 's> CommonCommands for ComputeInterface<'w, 's> {  }
impl<'w, 's> CommonCommands for GraphicsInterface<'w, 's> {  }

impl<'w, 's> ComputeCommands for ComputeInterface<'w, 's> {  }
impl<'w, 's> ComputeCommands for GraphicsInterface<'w, 's> {  }

impl<'w, 's> GraphicsCommands for GraphicsInterface<'w, 's> {  }

impl<'w, 's> TransferCommands for ComputeInterface<'w, 's> {  }
impl<'w, 's> TransferCommands for GraphicsInterface<'w, 's> {  }
impl<'w, 's> TransferCommands for TransferInterface<'w, 's> {  }
