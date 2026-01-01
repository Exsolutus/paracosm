pub mod compute;
pub mod graphics;

use crate::device::LogicalDevice;

use anyhow::{bail, Context, Result};
use bevy_ecs::resource::Resource;

use std::{
    any::{TypeId, type_name}, collections::HashMap, fs::File, path::{Path, PathBuf}, rc::Rc, time::SystemTime
};


pub trait PipelineLabel { }

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ShaderSource {
    Crate(PathBuf),
    SPV(PathBuf),
}

pub(crate) struct ShaderModule {
    pub(crate) spv_path: Rc<Path>,
    pub(crate) crate_path: Option<Rc<Path>>,
    pub(crate) loaded_at: SystemTime,
    pub(crate) inner: ash::vk::ShaderModule
}

impl std::ops::Deref for ShaderModule {
    type Target = ash::vk::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub enum PipelineInfo {
    Compute {
        shader_source: ShaderSource,
        entry_point: &'static str,
    },
    Graphics {
        task_shader: Option<(ShaderSource, &'static str)>,
        mesh_shader: (ShaderSource, &'static str),
        fragment_shader: (ShaderSource, &'static str),
        viewport: graphics::ViewportInfo,
        rasterization: graphics::RasterizationInfo,
        multisample: graphics::MultisampleInfo,
        depth_stencil: graphics::DepthTestInfo,
        attachment: graphics::AttachmentInfo,
    },
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
    pub max_push_constants_size: u32,
    shader_modules: HashMap<ShaderSource, ShaderModule>,
    pipelines: HashMap<TypeId, Pipeline>
}
// SAFETY: Valid so long as mutable access to PipelineManager is only exposed through the Context
unsafe impl Send for PipelineManager {  }
unsafe impl Sync for PipelineManager {  }   

impl PipelineManager {
    pub fn new(
        device: &LogicalDevice,
        max_push_constants_size: u32,
        descriptor_set_layout: &ash::vk::DescriptorSetLayout
    ) -> Result<Self> {
        let push_constant_range = ash::vk::PushConstantRange::default()
            .stage_flags(
                ash::vk::ShaderStageFlags::ALL | 
                ash::vk::ShaderStageFlags::MESH_EXT | 
                ash::vk::ShaderStageFlags::TASK_EXT
            )
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
            max_push_constants_size,
            shader_modules: HashMap::default(),
            pipelines: HashMap::default()
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
            // TODO: shader hot reloading

            bail!("Overwriting pipelines is not currently supported.")
        }

        let device = unsafe { self.device.as_ref().unwrap() };
        let pipeline = match &info {
            PipelineInfo::Compute { shader_source, entry_point } => {
                let pipeline_layout = self.pipeline_layout;

                self.load_shader_module(shader_source)?;

                let shader_module = unsafe { self.shader_modules.get(shader_source).unwrap_unchecked() };

                let pipeline = compute::create_compute_pipeline(
                    device,
                    &shader_module, 
                    entry_point, 
                    pipeline_layout,
                    #[cfg(debug_assertions)] std::any::type_name::<L>()
                )?;

                pipeline
            },
            PipelineInfo::Graphics { 
                task_shader, 
                mesh_shader, 
                fragment_shader, 
                viewport, 
                rasterization, 
                multisample, 
                depth_stencil, 
                attachment
            } => {
                let pipeline_layout = self.pipeline_layout;
                
                match task_shader {
                    Some((shader_source, entry_point)) => {
                        self.load_shader_module(shader_source)?;        
                        self.load_shader_module(&mesh_shader.0)?;
                        self.load_shader_module(&fragment_shader.0)?;

                        let task_shader_module = unsafe { self.shader_modules.get(shader_source).unwrap_unchecked() };
                        let mesh_shader_module = unsafe { self.shader_modules.get(&mesh_shader.0).unwrap_unchecked() };
                        let fragment_shader_module = unsafe { self.shader_modules.get(&fragment_shader.0).unwrap_unchecked() };

                        let pipeline = graphics::create_graphics_pipeline(
                            device, 
                            Some((&task_shader_module, entry_point)), 
                            (&mesh_shader_module, mesh_shader.1), 
                            (&fragment_shader_module, fragment_shader.1), 
                            &viewport, 
                            &rasterization, 
                            &multisample, 
                            &depth_stencil, 
                            &attachment,
                            pipeline_layout,
                            #[cfg(debug_assertions)] std::any::type_name::<L>()
                        )?;

                        pipeline
                    },
                    None => {
                        self.load_shader_module(&mesh_shader.0)?;
                        self.load_shader_module(&fragment_shader.0)?;
                                
                        let mesh_shader_module = unsafe { self.shader_modules.get(&mesh_shader.0).unwrap_unchecked() };
                        let fragment_shader_module = unsafe { self.shader_modules.get(&fragment_shader.0).unwrap_unchecked() };
                        
                        let pipeline = graphics::create_graphics_pipeline(
                            device, 
                            None, 
                            (&mesh_shader_module, mesh_shader.1), 
                            (&fragment_shader_module, fragment_shader.1), 
                            viewport, 
                            rasterization, 
                            multisample, 
                            depth_stencil, 
                            attachment,
                            pipeline_layout,
                            #[cfg(debug_assertions)] std::any::type_name::<L>()
                        )?;

                        pipeline
                    }
                }
            }
            PipelineInfo::RayTracing {  } => todo!()
        };
        
        self.pipelines.insert(TypeId::of::<L>(), Pipeline {
            info,
            inner: pipeline
        });

        Ok(())
    }

    fn load_shader_module(&mut self, source: &ShaderSource) -> Result<()> {
        let device = unsafe { self.device.as_ref().unwrap() };

        match self.shader_modules.contains_key(source) {
            true => {
                // TODO: shader hot reloading
            },
            false => match source {
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
                    //println!("{:?}", lines);
                    let spv_path = Path::new(lines[lines.len() - 1]);
                    let spv_file = &mut File::open(spv_path)?;
                    let spirv = &ash::util::read_spv(spv_file)?;
                    
                    let shader_module = unsafe { 
                        device.create_shader_module(
                            &ash::vk::ShaderModuleCreateInfo::default()
                                .code(&spirv), 
                            None
                        )? 
                    };

                    self.shader_modules.insert(
                        source.clone(), 
                        ShaderModule {
                            spv_path: spv_path.into(),
                            crate_path: Some(path.as_path().into()),
                            loaded_at: SystemTime::now(),
                            inner: shader_module
                        }
                    );
                },
                ShaderSource::SPV(path) => {
                    let spv_file = &mut File::open(&path)?;
                    let spirv = &ash::util::read_spv(spv_file)?;
                    
                    let shader_module = unsafe { 
                        device.create_shader_module(
                            &ash::vk::ShaderModuleCreateInfo::default()
                                .code(&spirv), 
                            None
                        )? 
                    };

                    self.shader_modules.insert(
                        source.clone(), 
                        ShaderModule {
                            spv_path: path.as_path().into(),
                            crate_path: None,
                            loaded_at: SystemTime::now(),
                            inner: shader_module
                        }
                    );
                    
                }
            }
        }

        Ok(())
    }
}

impl Drop for PipelineManager {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_ref().unwrap();

            for (_, shader_module) in self.shader_modules.iter() {
                device.destroy_shader_module(**shader_module, None);
            }

            // TODO: Verify destruction safety requirements
            for (_, pipeline) in self.pipelines.iter() {
                device.destroy_pipeline(**pipeline, None);
            }

            device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}


impl crate::context::Context {
    pub fn create_pipeline(&mut self, label: impl PipelineLabel + 'static, info: PipelineInfo) -> Result<()> {
        self.active_device.world
            .resource_mut::<PipelineManager>()
            .set(label, info)
    }
}