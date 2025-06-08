use super::{BufferLabel, ResourceManager, TransferMode};

use anyhow::Result;
use ash::vk::{BufferCreateInfo, BufferUsageFlags, MemoryPropertyFlags};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use std::{any::TypeId, ffi::CString};


pub struct BufferInfo {
    pub size: usize,
    pub transfer_mode: TransferMode,

    #[cfg(debug_assertions)] pub debug_name: &'static str
}


pub struct BufferView {
    index: usize,
    size: usize,
    offset: usize,
    transfer_mode: TransferMode,

    #[cfg(debug_assertions)] debug_name: &'static str
}

pub(crate) struct PersistentBuffer {
    pub storage_buffer_index: usize,
    pub staging_buffer_index: usize,
    pub descriptor_index: usize
}

impl crate::context::Context {
    pub fn create_buffer(&mut self, info: BufferInfo) -> Result<BufferView> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = unsafe { device.graph_world.get_resource_mut::<ResourceManager>().unwrap_unchecked() };

        // Create new storage buffer
        let buffer_create_info = BufferCreateInfo::default()
            .size(info.size as u64)
            .usage(BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::STORAGE_BUFFER);
        let allocation_create_info = match info.transfer_mode {
            TransferMode::Auto |
            TransferMode::AutoUpload |
            TransferMode::AutoDownload => todo!(),
            TransferMode::Stream => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: MemoryPropertyFlags::DEVICE_LOCAL,
                flags: AllocationCreateFlags::MAPPED
                     | AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                ..Default::default()
            }
        };

        let (buffer, allocation) = unsafe { resource_manager.allocator.create_buffer(&buffer_create_info, &allocation_create_info)? };
        resource_manager.storage_buffers.push((buffer, allocation));

        #[cfg(debug_assertions)]
        unsafe {
            let buffer_name = CString::new(format!("Storage Buffer: {}", info.debug_name))?;
            let buffer_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(buffer)
                .object_name(&buffer_name);
            device.logical_device.debug_utils.set_debug_utils_object_name(&buffer_name_info)?;
        }

        Ok(BufferView {
            index: resource_manager.storage_buffers.len(),
            size: info.size,
            offset: 0,
            transfer_mode: info.transfer_mode,
            #[cfg(debug_assertions)] debug_name: info.debug_name
        })
    }

    pub fn destroy_buffer(&mut self, buffer: BufferView) -> Result<()> {
        todo!()
    }

    pub fn set_persistent_buffer<L: BufferLabel + 'static>(&mut self, _label: L, buffer_view: &BufferView) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = unsafe { device.graph_world.get_resource_mut::<ResourceManager>().unwrap_unchecked() };

        Ok(())
    }

    pub fn set_transient_buffer(
        &mut self, 
        label: impl BufferLabel, 
        size: u32,
        #[cfg(debug_assertions)] debug_name: &'static str
    ) -> Result<()> {
        todo!()
    }
}