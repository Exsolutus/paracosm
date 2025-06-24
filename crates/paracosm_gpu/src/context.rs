use crate::device::{Device, DeviceProperties, PhysicalDevice};
#[cfg(feature = "WSI")] use crate::surface::{HasSurfaceHandles, Surface, SurfaceConfig};
#[cfg(debug_assertions)] use crate::validation::*;

use anyhow::{bail, Context as _, Result};
use bevy_ecs::prelude::Resource;
#[cfg(feature = "WSI")] use bevy_utils::synccell::SyncCell;

use std::{
    collections::VecDeque, default::Default, ffi::{c_char, CStr, CString}, mem::ManuallyDrop
};


pub struct ContextInfo {
    pub application_name: Box<str>,
    pub application_version: (u32, u32, u32, u32),
    pub engine_name: Box<str>,
    pub engine_version: (u32, u32, u32, u32),
    //#[cfg(debug_assertions)] pub validation_callback: Option<ValidationCallback>
}

impl Default for ContextInfo {
    fn default() -> Self {
        Self {
            application_name: "Paracosm GPU App".into(),
            application_version: (0, 0, 1, 0),
            engine_name: "Paracosm".into(),
            engine_version: (0, 0, 1, 0),
            //#[cfg(debug_assertions)] validation_callback: None
        }
    }
}

// TODO: Builder pattern for Context initialization
#[derive(Resource)]
pub struct Context {
    info: ContextInfo,

    entry: ash::Entry,
    instance: ash::Instance,

    pub(crate) primary_device: u32,
    pub(crate) configuring_device: u32,
    pub(crate) devices: ManuallyDrop<Box<[Device]>>,

    #[cfg(feature = "WSI")] primary_surface: u32,
    #[cfg(feature = "WSI")] surfaces: ManuallyDrop<SyncCell<Vec<Surface>>>,

    #[cfg(debug_assertions)] _debug_utils: DebugUtilsInstance,
    #[cfg(debug_assertions)] _debug_utils_messenger: DebugUtilsMessenger
}
unsafe impl Send for Context {  }
unsafe impl Sync for Context {  }

impl Context {
    pub fn new(
        info: ContextInfo, 
        #[cfg(feature = "WSI")] primary_window: &dyn HasSurfaceHandles,
        #[cfg(feature = "WSI")] surface_config: SurfaceConfig
    ) -> Result<Self> {
        let entry = ash::Entry::linked();

        // Gather required instance layer and extension names
        let layer_names = Vec::from([
            #[cfg(debug_assertions)]
            unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") }
        ]);
        let layer_names_raw: Vec<*const c_char> = layer_names.iter()
            .map(|name| name.as_ptr())
            .collect();

        let mut extension_names = Vec::from([
            #[cfg(debug_assertions)] ash::ext::debug_utils::NAME.as_ptr(),
        ]);
        #[cfg(feature = "WSI")]
        if let Ok(display_handle) = primary_window.display_handle() {
            extension_names.append(&mut ash_window::enumerate_required_extensions(display_handle.as_raw())?.to_vec());
        }

        // Create Vulkan instance
        let application_name = CString::new(info.application_name.as_ref())?;
        let engine_name = CString::new(info.engine_name.as_ref())?;
        let application_info = ash::vk::ApplicationInfo::default()
            .application_name(application_name.as_c_str())
            .application_version(ash::vk::make_api_version(
                info.application_version.0, 
                info.application_version.1, 
                info.application_version.2, 
                info.application_version.3
            ))
            .engine_name(engine_name.as_c_str())
            .engine_version(ash::vk::make_api_version(
                info.engine_version.0, 
                info.engine_version.1, 
                info.engine_version.2, 
                info.engine_version.3
            ))
            .api_version(ash::vk::make_api_version(0, 1, 3, 0));
        let instance_create_info = ash::vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names);
        let instance = unsafe{
            entry.create_instance(&instance_create_info, None)?
        };

        // Create primary Vulkan surface
        #[cfg(feature = "WSI")]
        let mut primary_surface = Surface::new(&entry, &instance, primary_window)?;

        // Create Vulkan devices
        let mut physical_devices = unsafe { instance.enumerate_physical_devices()?
            .iter()
            .map(|&physical_device| {
                let mut physical_device = PhysicalDevice::new(&instance, physical_device).unwrap();

                #[cfg(feature = "WSI")]
                {
                    physical_device.properties.supports_presentation = primary_surface.get_physical_device_surface_support(
                        *physical_device, 
                        physical_device.properties.queue.graphics_family, 
                        primary_surface.surface
                    ).unwrap();
                }

                physical_device
            })
            .collect::<VecDeque<_>>()
        };

        println!("Available devices: {:?}", physical_devices.iter()
            .map(|physical_device| physical_device.properties.device_name_as_c_str().unwrap() )
            .collect::<Vec<_>>()
        );

        let primary_device = physical_devices.iter()
            .enumerate()
            .max_by_key(|(_, physical_device)| {
                #[cfg(feature = "WSI")]
                let check = physical_device.properties.supports_presentation && physical_device.properties.api_version >= ash::vk::API_VERSION_1_3;
                #[cfg(not(feature = "WSI"))]
                let check = physical_device.properties.api_version >= ash::vk::API_VERSION_1_3;

                match check {
                    true => match physical_device.properties.device_type {
                        ash::vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                        ash::vk::PhysicalDeviceType::INTEGRATED_GPU => 100,
                        ash::vk::PhysicalDeviceType::VIRTUAL_GPU => 10,
                        _ => 1
                    },
                    false => 0
                }
            })
            .map(|(index, _)| index as u32)
            .context("No valid device found.")?;

        println!("Selected primary device: {}, {:?}", primary_device, physical_devices[primary_device as usize].properties.device_name_as_c_str().unwrap());

        let mut devices = Vec::with_capacity(physical_devices.len());
        for _ in 0..physical_devices.len() {
            devices.push(Device::new(instance.clone(), physical_devices.pop_front().unwrap())?);
        }

        // Configure primary Vulkan surface
        #[cfg(feature = "WSI")]
        primary_surface.configure(&devices[primary_device as usize], surface_config)?;


        // Create Vulkan debug messenger
        #[cfg(debug_assertions)]
        let (_debug_utils, _debug_utils_messenger) = unsafe {
            //let user_data = info.as_mut() as *mut ContextInfo as *mut c_void;
            let messenger_create_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    MessageSeverity::ERROR |
                    MessageSeverity::WARNING |
                    MessageSeverity::INFO |
                    MessageSeverity::VERBOSE
                )
                .message_type(
                    MessageType::GENERAL |
                    MessageType::VALIDATION |
                    MessageType::PERFORMANCE
                )
                //.user_data(user_data)
                .pfn_user_callback(Some(debug_utils_messenger_callback));

            let debug_utils = DebugUtilsInstance::new(&entry, &instance);
            let debug_utils_messenger = debug_utils.create_debug_utils_messenger(&messenger_create_info, None)?;

            (debug_utils, debug_utils_messenger)
        };

        Ok(Self {
            info,
            entry,
            instance,
            primary_device,
            configuring_device: primary_device,
            devices: ManuallyDrop::new(devices.into_boxed_slice()),
            #[cfg(feature = "WSI")] primary_surface: 0,
            #[cfg(feature = "WSI")] surfaces: ManuallyDrop::new(SyncCell::new(vec![primary_surface])),
            #[cfg(debug_assertions)] _debug_utils,
            #[cfg(debug_assertions)] _debug_utils_messenger
        })
    }

    pub fn devices(&self) -> Box<[&DeviceProperties]> {
        self.devices.iter()
            .map(|device| {
                device.properties()
            })
            .collect::<Vec<&DeviceProperties>>()
            .into_boxed_slice()
    }

    pub fn set_primary_device(&mut self, index: u32) -> Result<()> {
        match index < self.devices.len() as u32 {
            true => self.primary_device = index,
            false => bail!("'index' should be a valid device index.")
        };

        // TODO: any necessary validation/cleanup/recreation
        todo!()
    }

    // TODO: properly implement multi-device support
    // pub fn configuring_device(&mut self, index: u32) -> Result<()> {
    //     match index < self.devices.len() as u32 {
    //         true => self.configuring_device = index,
    //         false => bail!("'index' should be a valid device index.")
    //     };

    //     todo!()
    // }

    /// Executes the frame graph of the primary device.
    pub fn execute(&mut self) -> Result<()> {
        let device = unsafe {
            self.devices.get_mut(self.primary_device as usize).unwrap_unchecked()
        };

        device.execute()?;

        Ok(())
    }
    
    pub fn present(&mut self) -> Result<()> {
        let device = unsafe {
            self.devices.get_mut(self.primary_device as usize).unwrap_unchecked()
        };

        device.present()?;

        todo!()
    }
}


impl Drop for Context {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        unsafe {
            //  Safety: vkDestroyDebugUtilsMessengerEXT
            //  Host Synchronization
            //   -  Host access to messenger must be externally synchronized
            //
            //  Messenger is private to this object
            self._debug_utils.destroy_debug_utils_messenger(self._debug_utils_messenger, None);
        }

        unsafe {
            // Drop all surfaces
            #[cfg(feature = "WSI")]
            ManuallyDrop::drop(&mut self.surfaces);

            // Drop all devices
            ManuallyDrop::drop(&mut self.devices);

            //  Safety: vkDestroyInstance
            //  Host Synchronization
            //   -  Host access to instance must be externally synchronized
            //   -  Host access to all VkPhysicalDevice objects enumerated from instance must be externally synchronized
            //
            //  Synchronized host access to instance guaranteed by borrow checker with '&mut self'
            //  Device object(s) created with this instance are dropped above, so this should only drop after all Devices drop
            self.instance.destroy_instance(None);
        }
    }
}