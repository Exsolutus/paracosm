use crate::{node::resource::ResourceIndex, resource::BUFFER_BINDING};

use super::{BufferLabel, ResourceManager, TransferMode};

use anyhow::{bail, Ok, Result};
use ash::vk::{BufferCreateInfo, BufferUsageFlags, MemoryPropertyFlags};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use std::{any::{type_name, type_name_of_val, Any}, ffi::CString, marker::PhantomData};


pub(crate) enum Buffer {
    Persistent {
        buffer: ash::vk::Buffer,
        allocation: vk_mem::Allocation,
        descriptor_index: u32,

        transfer_mode: TransferMode,
        size: usize,
        #[cfg(debug_assertions)] debug_name: &'static str
    },
    Transient {
        offset: usize,
        size: usize,
        #[cfg(debug_assertions)] debug_name: &'static str
    }
}


impl crate::context::Context {
    pub fn create_buffer<L: BufferLabel + 'static>(
        &mut self,
        label: L,
        transfer_mode: TransferMode,
        size: usize,
    ) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = device.graph_world.non_send_resource_mut::<ResourceManager>();

        let debug_name = type_name_of_val(&label);

        // Create new storage buffer
        let buffer_create_info = BufferCreateInfo::default()
            .size(size as u64)
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

        let descriptor_index = match resource_manager.buffers.free_descriptors.pop() {
            Some(index) => index,
            None => {
                let index = resource_manager.buffers.next_descriptor;
                resource_manager.buffers.next_descriptor += 1;
                index
            }
        };

        let (buffer, allocation) = unsafe { resource_manager.allocator.create_buffer(&buffer_create_info, &allocation_create_info)? };
        resource_manager.buffers.resources.insert(
            label.type_id(),
            Buffer::Persistent { 
                buffer, 
                allocation,
                descriptor_index,
                transfer_mode,
                size,
                #[cfg(debug_assertions)] debug_name
            }
        );

        let buffer_info = [
            ash::vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .range(ash::vk::WHOLE_SIZE)
        ];

        unsafe { device.logical_device.update_descriptor_sets(
            &[
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(resource_manager.descriptor_set)
                    .dst_binding(BUFFER_BINDING)
                    .dst_array_element(descriptor_index)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_info)
            ], 
            &[]
        ); }

        device.graph_world.insert_resource::<ResourceIndex<L>>(ResourceIndex::<L> { descriptor_index, _marker: PhantomData::<L>::default() });

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

    pub fn get_buffer_memory<T>(&self, label: impl BufferLabel + 'static) -> Result<&T> {
        let device = &self.devices[self.configuring_device as usize];
        let resource_manager = device.graph_world.non_send_resource::<ResourceManager>();

        match resource_manager.buffers.resources.get(&label.type_id()) {
            Some(Buffer::Persistent { buffer, allocation, descriptor_index, transfer_mode, size, debug_name }) => {
                match transfer_mode {
                    TransferMode::Auto |
                    TransferMode::AutoUpload |
                    TransferMode::AutoDownload => todo!(),
                    TransferMode::Stream => unsafe {
                        if size_of::<T>() != *size {
                            bail!("Size of {} does not match buffer size.", type_name::<T>())
                        }

                        let data_ptr = resource_manager.allocator.get_allocation_info(allocation).mapped_data;
                        let data = data_ptr as *const T;
                        
                        Ok(&*data)
                    }
                }
            }
            Some(Buffer::Transient { offset, size, debug_name }) => {
                bail!("Buffer {} is not host mapped.", debug_name)
            },
            None => bail!("No buffer found with label {}.", type_name_of_val(&label))
        }
    }

    pub fn get_buffer_memory_mut<T>(&mut self, label: impl BufferLabel + 'static) -> Result<&mut T> {
        let device = &mut self.devices[self.configuring_device as usize];
        let resource_manager = device.graph_world.non_send_resource::<ResourceManager>();

        match resource_manager.buffers.resources.get(&label.type_id()) {
            Some(Buffer::Persistent { buffer, allocation, descriptor_index, transfer_mode, size, debug_name }) => {
                match transfer_mode {
                    TransferMode::Auto |
                    TransferMode::AutoUpload |
                    TransferMode::AutoDownload => todo!(),
                    TransferMode::Stream => unsafe {
                        if size_of::<T>() != *size {
                            bail!("Size of {} does not match buffer size.", type_name::<T>())
                        }

                        let data_ptr = resource_manager.allocator.get_allocation_info(allocation).mapped_data;
                        let data = data_ptr as *mut T;
                        
                        Ok(&mut *data)
                    }
                }
            }
            Some(Buffer::Transient { offset, size, debug_name }) => {
                bail!("Buffer {} is not host mapped.", debug_name)
            },
            None => bail!("No buffer found with label {}.", type_name_of_val(&label))
        }
    }


    pub fn destroy_buffer(&mut self, label: impl BufferLabel + 'static) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = device.graph_world.non_send_resource_mut::<ResourceManager>();

        match resource_manager.buffers.resources.remove(&label.type_id()) {
            Some(Buffer::Persistent { buffer, mut allocation, descriptor_index, .. }) => unsafe {
                let buffer_info = [
                    ash::vk::DescriptorBufferInfo::default()
                        .buffer(ash::vk::Buffer::null())
                        .range(ash::vk::WHOLE_SIZE)
                ];
            
                device.logical_device.update_descriptor_sets(
                    &[
                        ash::vk::WriteDescriptorSet::default()
                            .dst_set(resource_manager.descriptor_set)
                            .dst_binding(BUFFER_BINDING)
                            .dst_array_element(descriptor_index)
                            .descriptor_count(1)
                            .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                            .buffer_info(&buffer_info)
                    ], 
                    &[]
                );

                resource_manager.buffers.free_descriptors.push(descriptor_index);
                resource_manager.allocator.destroy_buffer(buffer, &mut allocation); 
            }
            Some(Buffer::Transient { .. }) => {
                todo!()
            },
            None => bail!("No buffer found with label {}", type_name_of_val(&label))
        }

        Ok(())
    }
}