mod compute;

use crate::device::LogicalDevice;

use anyhow::{bail, Context, Result};
use bevy_ecs::system::Resource;

use std::{
    any::{type_name, TypeId}, 
    collections::HashMap, 
    path::{Path, PathBuf}, 
};


pub trait PipelineLabel { }

pub enum ShaderSource {
    Crate(PathBuf),
    SPV(PathBuf),
}

pub struct ShaderModule {
    pub(crate) spv_path: Box<Path>,
    pub(crate) crate_path: Option<Box<Path>>
}

pub enum PipelineInfo {
    Compute {
        shader_module: ShaderModule,
        entry_point: &'static str,
    },
    Graphics { },
    RayTracing { }
}

pub(crate) struct Pipeline {
    pub info: PipelineInfo,
    inner: ash::vk::Pipeline
}

impl std::ops::Deref for Pipeline {
    type Target = ash::vk::Pipeline;
    fn deref(&self) -> &Self::Target { &self.inner }
}

#[derive(Resource)]
pub(crate) struct PipelineManager {
    device: *const LogicalDevice,
    pub pipeline_layout: ash::vk::PipelineLayout,
    pipelines: HashMap<TypeId, Pipeline>,
    max_push_constants_size: u32
}
unsafe impl Send for PipelineManager {  }   // SAFETY: safe while graph execution is single threaded
unsafe impl Sync for PipelineManager {  }

impl PipelineManager {
    pub fn new(
        device: &LogicalDevice,
        max_push_constants_size: u32,
        descriptor_set_layout: &ash::vk::DescriptorSetLayout
    ) -> Result<Self> {
        let push_constant_range = ash::vk::PushConstantRange::default()
            .stage_flags(ash::vk::ShaderStageFlags::ALL)
            .size(max_push_constants_size);
        let pipeline_layout_create_info = ash::vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(descriptor_set_layout))
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));
        let pipeline_layout = unsafe { 
            device.create_pipeline_layout(&pipeline_layout_create_info, None) 
                .context("PipelineLayout should be created.")?
        };

        Ok(Self {
            device,
            pipeline_layout,
            pipelines: HashMap::default(),
            max_push_constants_size
        })
    }

    pub fn get<L: PipelineLabel + 'static>(&self, _label: L) -> Result<&Pipeline>  {
        match self.pipelines.get(&TypeId::of::<L>()) {
            Some(pipeline) => Ok(pipeline),
            None => bail!("No pipeline found with label: {:?}", type_name::<L>())
        }
    }

    fn set<L: PipelineLabel + 'static>(&mut self, _label: L, info: PipelineInfo) -> Result<()> {
        if let Some(_) = self.pipelines.get(&TypeId::of::<L>()) {
            bail!("Overwriting pipelines is not currently supported.")
        }

        match &info {
            PipelineInfo::Compute { shader_module, entry_point } => {
                let device = unsafe { self.device.as_ref().unwrap() };
                let pipeline = compute::create_compute_pipeline(
                    device,
                    shader_module, 
                    &entry_point, 
                    &self.pipeline_layout,
                    #[cfg(debug_assertions)] std::any::type_name::<L>()
                )?;

                self.pipelines.insert(TypeId::of::<L>(), Pipeline {
                    info,
                    inner: pipeline
                });
            },
            _ => todo!()
        }

        Ok(())
    }
}

impl Drop for PipelineManager {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_ref().unwrap();

            // TODO: Verify destruction safety requirements
            for (_, pipeline) in self.pipelines.iter() {
                device.destroy_pipeline(**pipeline, None);
            }

            device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}


impl crate::context::Context {
    pub fn load_shader_module(&self, source: ShaderSource) -> Result<ShaderModule> {
        match source {
            ShaderSource::Crate(path) => {
                let full_path = std::path::absolute(&path)?;
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                let mut dir = PathBuf::from(manifest_dir);  
                dir.push("shader_builder");
            
                let output = std::process::Command::new("cargo")
                    .current_dir(dir)
                    .args([
                        "+nightly-2024-11-22",
                        "run",
                        "--release"
                    ])
                    .arg(full_path)
                    .output()?;

                if !output.status.success() {
                    bail!("Failed to build shader module.\n{}", String::from_utf8(output.stderr)?)
                }

                let stdout = String::from_utf8(output.stdout)?;
                //let _stderr = String::from_utf8(output.stderr)?;

                let lines: Vec<&str> = stdout.lines().collect();
                let spv_path = Path::new(lines[lines.len() - 1]);

                Ok(ShaderModule {
                    spv_path: spv_path.into(),
                    crate_path: Some(path.into())
                })
            },
            ShaderSource::SPV(path) => {
                Ok(ShaderModule {
                    spv_path: path.into(),
                    crate_path: None
                })
            }
        }
    }

    pub fn set_pipeline(&mut self, label: impl PipelineLabel + 'static, info: PipelineInfo) -> Result<()> {
        self.devices[self.configuring_device as usize].graph_world
            .resource_mut::<PipelineManager>()
            .set(label, info)
    }
}