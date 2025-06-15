use crate::resource::BUFFER_BINDING;

use super::{BufferLabel, ResourceManager, TransferMode};

use anyhow::{bail, Ok, Result};
use ash::vk::{BufferCreateInfo, BufferUsageFlags, MemoryPropertyFlags};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use std::{any::{type_name, TypeId}, ffi::CString, ptr::{slice_from_raw_parts, slice_from_raw_parts_mut}};


pub(crate) enum Buffer {
    Persistent {
        buffer: ash::vk::Buffer,
        allocation: vk_mem::Allocation,

        transfer_mode: TransferMode,
        type_id: TypeId,
        length: usize,
        #[cfg(debug_assertions)] debug_name: &'static str
    },
    Transient {
        type_id: TypeId,
        length: usize,
        offset: usize,
        #[cfg(debug_assertions)] debug_name: &'static str
    }
}


impl crate::context::Context {
    pub fn create_buffer<L: BufferLabel + 'static, T: 'static>(
        &mut self,
        transfer_mode: TransferMode,
        length: usize,
    ) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = unsafe { device.graph_world.get_resource_mut::<ResourceManager>().unwrap_unchecked() };

        let debug_name = type_name::<L>();

        // Create new storage buffer
        let buffer_create_info = BufferCreateInfo::default()
            .size((length * size_of::<T>()) as u64)
            .usage(BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::STORAGE_BUFFER);
        let allocation_create_info = match transfer_mode {
            TransferMode::Auto |
            TransferMode::AutoUpload |
            TransferMode::AutoDownload => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: MemoryPropertyFlags::DEVICE_LOCAL,
                ..Default::default()
            },
            TransferMode::Stream => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: MemoryPropertyFlags::DEVICE_LOCAL,
                flags: AllocationCreateFlags::MAPPED
                     | AllocationCreateFlags::HOST_ACCESS_RANDOM,
                ..Default::default()
            }
        };

        let (buffer, allocation) = unsafe { resource_manager.allocator.create_buffer(&buffer_create_info, &allocation_create_info)? };
        resource_manager.buffers.insert(
            TypeId::of::<L>(),
            Buffer::Persistent { 
                buffer, 
                allocation, 
                transfer_mode,
                length,
                type_id: TypeId::of::<T>(),
                #[cfg(debug_assertions)] debug_name
            }
        );

        let buffer_info = [
            ash::vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .range((length * size_of::<T>()) as u64)
        ];

        unsafe { device.logical_device.update_descriptor_sets(
            &[
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(resource_manager.descriptor_set)
                    .dst_binding(BUFFER_BINDING)
                    .dst_array_element(0)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_info)
            ], 
            &[]);
        }

        #[cfg(debug_assertions)]
        unsafe {
            let buffer_name = CString::new(format!("Storage Buffer: {}", debug_name))?;
            let buffer_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(buffer)
                .object_name(&buffer_name);
            device.logical_device.debug_utils.set_debug_utils_object_name(&buffer_name_info)?;
        }

        Ok(())
    }

    pub fn create_transient_buffer(
        &mut self, 
        label: impl BufferLabel, 
        size: u32
    ) -> Result<()> {
        todo!()
    }

    pub fn get_buffer_memory<L: BufferLabel + 'static, T: 'static>(&self) -> Result<&[T]> {
        let device = &self.devices[self.configuring_device as usize];
        let resource_manager = unsafe { device.graph_world.get_resource::<ResourceManager>().unwrap_unchecked() };

        match resource_manager.buffers.get(&TypeId::of::<L>()) {
            Some(Buffer::Persistent { buffer: _, allocation, transfer_mode, type_id, length, debug_name }) => {
                if *type_id != TypeId::of::<T>() {
                    bail!("Buffer data is not of type {}.", type_name::<T>());
                }

                match transfer_mode {
                    TransferMode::Auto |
                    TransferMode::AutoUpload |
                    TransferMode::AutoDownload => bail!("Buffer {} is not host mapped.", debug_name),
                    TransferMode::Stream => unsafe {
                        let data_ptr = resource_manager.allocator.get_allocation_info(allocation).mapped_data;
                        let data = slice_from_raw_parts::<T>(data_ptr as *const T, *length).as_ref().unwrap();
                        
                        Ok(data)
                    }
                }
            }
            Some(Buffer::Transient { .. }) => {
                bail!("No buffer found with label {}.", type_name::<L>())
            },
            None => bail!("No buffer found with label {}.", type_name::<L>())
        }
    }

    pub fn get_buffer_memory_mut<L: BufferLabel + 'static, T: 'static>(&mut self) -> Result<&mut [T]> {
        let device = &mut self.devices[self.configuring_device as usize];
        let resource_manager = unsafe { device.graph_world.get_resource_mut::<ResourceManager>().unwrap_unchecked() };

        match resource_manager.buffers.get(&TypeId::of::<L>()) {
            Some(Buffer::Persistent { buffer: _, allocation, transfer_mode, type_id, length, debug_name }) => {
                if *type_id != TypeId::of::<T>() {
                    bail!("Buffer data is not of type {}.", type_name::<T>());
                }

                match transfer_mode {
                    TransferMode::Auto |
                    TransferMode::AutoUpload |
                    TransferMode::AutoDownload => bail!("Buffer {} is not host mapped.", debug_name),
                    TransferMode::Stream => unsafe {
                        let data_ptr = resource_manager.allocator.get_allocation_info(allocation).mapped_data;
                        let data = slice_from_raw_parts_mut::<T>(data_ptr as *mut T, *length).as_mut().unwrap();
                        
                        Ok(data)
                    }
                }
            }
            Some(Buffer::Transient { .. }) => {
                bail!("No buffer found with label {}.", type_name::<L>())
            },
            None => bail!("No buffer found with label {}.", type_name::<L>())
        }
    }


    pub fn destroy_buffer<L: BufferLabel + 'static>(&mut self, _label: L ) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = unsafe { device.graph_world.get_resource_mut::<ResourceManager>().unwrap_unchecked() };

        match resource_manager.buffers.remove(&TypeId::of::<L>()) {
            Some(Buffer::Persistent { buffer, mut allocation, .. }) => unsafe { 
                resource_manager.allocator.destroy_buffer(buffer, &mut allocation); 
            }
            Some(Buffer::Transient { .. }) => {
                todo!()
            },
            None => bail!("No buffer found with label {}", type_name::<L>())
        }

        Ok(())
    }
}