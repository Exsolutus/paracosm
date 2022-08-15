
use ash::{
    extensions::{
        ext::DebugUtils,
    },
    vk
};

use bevy_log::prelude::*;

use std::{
    ffi::CStr,
    ops::Deref,
    os::raw::{
        c_char,
        c_void
    },
    sync::Arc
};

/// Internal data for the Vulkan instance.
///
/// [`Instance`] is the public API for interacting with the Vulkan instance.
pub struct InstanceInternal {
    pub entry: ash::Entry,
    instance: ash::Instance,

    #[cfg(debug_assertions)] _debug_utils: DebugUtils,
    #[cfg(debug_assertions)] _debug_callback: vk::DebugUtilsMessengerEXT
}

impl InstanceInternal {
    pub fn bar(&self) {
        info!("bar");
    }
}

impl Deref for InstanceInternal {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl Drop for InstanceInternal {
    fn drop(&mut self) {
        info!("Dropping Instance!");

        unsafe {
            // Safety: vkDestroyDebugUtilsMessengerEXT
            //  Host Synchronization
            //   -  Host access to messenger must be externally synchronized
            //
            //  Messenger is private to this object
            #[cfg(debug_assertions)]
            self._debug_utils.destroy_debug_utils_messenger(self._debug_callback, None);

            //  Safety: vkDestroyInstance
            //  Host Synchronization
            //   -  Host access to instance must be externally synchronized
            //   -  Host access to all VkPhysicalDevice objects enumerated from instance must be externally synchronized
            //  
            //  Synchronized host access to instance guaranteed by borrow checker with '&mut self'
            //  Device objects created with this instance retain a reference, so this should only drop after all Devices drop  
            self.instance.destroy_instance(None);
        }
    }
}


/// Public API for interacting with the Vulkan instance.
pub struct Instance {
    internal: Arc<InstanceInternal>
}

impl Instance {
    pub fn new(
        entry: ash::Entry,
        app_info: vk::ApplicationInfo,
        extensions: &mut Vec<*const c_char>
    ) -> Result<Self, String> {
        info!("Creating Vulkan instance");
        
        // Define instance layers to request
        let mut layers: Vec<*const c_char> = vec![
            // Layer names must be null-terminated
            
        ];
        #[cfg(debug_assertions)]
        layers.append(&mut vec!["VK_LAYER_KHRONOS_validation\0".as_ptr() as *const c_char]);

        // Add DebugUtils to extensions to request
        #[cfg(debug_assertions)]
        {
            let debug_ext = DebugUtils::name().as_ptr();
            if !extensions.contains(&debug_ext) {
                extensions.append(&mut vec![
                    debug_ext
                ]);
            }
        }

        // Create instance
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(extensions);
        let mut instance = match unsafe { entry.create_instance(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        };

        #[cfg(debug_assertions)]
        let (_debug_utils, _debug_callback) = setup_debug_utils(&entry, &mut instance);

        Ok(Self { 
            internal: Arc::new(InstanceInternal {
                entry,
                instance,

                #[cfg(debug_assertions)] _debug_utils,
                #[cfg(debug_assertions)] _debug_callback
            }) 
        })
    }
}

// Manually implement Clone to log ref counts for debugging
impl Clone for Instance {
    #[inline]
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        info!("Ref count: {}", Arc::strong_count(&self.internal) + 1);

        Self { internal: self.internal.clone() }
    }
}

impl Deref for Instance {
    type Target = InstanceInternal;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        info!("Dropping ref to Instance!");
    }
}



/// Vulkan Debug Utils callback function
#[cfg(debug_assertions)]
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message = CStr::from_ptr((*p_callback_data).p_message);
    println!("{}{}{:?}", severity, types, message);

    vk::FALSE
}

#[cfg(debug_assertions)]
fn setup_debug_utils(_entry: &ash::Entry, instance: &mut ash::Instance) -> (DebugUtils, vk::DebugUtilsMessengerEXT) {
    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR | 
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | 
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback))
        .build();
    
    let debug_utils_loader = DebugUtils::new(_entry, instance);
    /*  The application must ensure that vkCreateDebugUtilsMessengerEXT is not executed in parallel with any 
     *  Vulkan command that is also called with instance or child of instance as the dispatchable argument.
     *  
     *  Guaranteed by borrow checker with 'instance: &mut ash::Instance'                                        */
    let debug_callback = unsafe {
        debug_utils_loader
            .create_debug_utils_messenger(&debug_info, None)
            .unwrap()
    };

    (debug_utils_loader, debug_callback)
}
