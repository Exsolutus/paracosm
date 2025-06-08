// use crate::context::ContextInfo;

use ash::vk;

use std::ffi::{
    CStr,
    c_void
};


// Reexport types
pub use ash::vk::{
    DebugUtilsMessageSeverityFlagsEXT as MessageSeverity,
    DebugUtilsMessageTypeFlagsEXT as MessageType,
    DebugUtilsMessengerCallbackDataEXT as MessageData,
    DebugUtilsMessengerEXT as DebugUtilsMessenger,
};
pub(crate) use ash::ext::debug_utils::{
    Instance as DebugUtilsInstance,
    Device as DebugUtilsDevice,
};


pub type ValidationCallback = fn(MessageSeverity, MessageType, &CStr);

#[inline]
pub fn default_validation_callback(
    message_severity: MessageSeverity,
    message_type: MessageType,
    message: &CStr,
) {
    let mut exit = false;

    let severity = match message_severity {
        MessageSeverity::VERBOSE => "[Verbose]",
        MessageSeverity::INFO => "[Info]",
        MessageSeverity::WARNING => { exit = true; "[Warning]" },
        MessageSeverity::ERROR => { exit = true; "[Error]" },
        _ => "[Unknown]",
    };
    let types = match message_type {
        MessageType::GENERAL => "[General]",
        MessageType::PERFORMANCE => "[Performance]",
        MessageType::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };

    println!("{}{}\n{:?}\n", severity, types, message);
    debug_assert!(!exit, "VULKAN VALIDATION FAILURE");
}

#[cfg(debug_assertions)]
pub unsafe extern "system" fn debug_utils_messenger_callback(
    message_severity: MessageSeverity,
    message_type: MessageType,
    p_message_data: *const MessageData,
    _p_user_data: *mut c_void
) -> vk::Bool32 {
    // let info = std::ptr::NonNull::new(_p_user_data as *mut ContextInfo).unwrap().as_ref();
    let message = CStr::from_ptr((*p_message_data).p_message);

    // let validation_callback = match info.validation_callback {
    //     Some(callback) => callback,
    //     None => default_validation_callback
    // };
    // validation_callback(message_severity, message_type, message);

    default_validation_callback(message_severity, message_type, message);

    vk::FALSE
}