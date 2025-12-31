use crate::resource::{BUFFER_BINDING, ResourceHandle, SyncLabel};

use super::TransferMode;

use anyhow::{Context, Ok, Result, bail};
use bevy_ecs::{component::Component, entity::Entity};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use std::any::type_name;


pub struct BufferHandle {
    host_entity: Entity
}

impl ResourceHandle for BufferHandle {
    fn host_entity(&self) -> Entity { self.host_entity }
}

pub trait BufferLabel: SyncLabel { }

#[derive(Clone, Copy, Default)]
pub struct BufferInfo {
    pub transfer_mode: TransferMode,
    pub size: usize,
    pub shader_mutable: bool,
    #[cfg(debug_assertions)] pub debug_name: &'static str
}

#[derive(Component)]
pub struct Buffer {
    pub info: BufferInfo,
    pub descriptor_index: u32,

    pub(crate) inner: ash::vk::Buffer,
    pub(crate) allocation: vk_mem::Allocation,
}


impl crate::context::Context {
    pub fn create_buffer(
        &mut self,
        info: BufferInfo
    ) -> Result<BufferHandle> {
        let device = &mut self.active_device;

        // Create new storage buffer
        let queue_properties = device.physical_device.properties.queue;
        let queue_families = [queue_properties.graphics_family, queue_properties.compute_family, queue_properties.transfer_family];
        let buffer_create_info = ash::vk::BufferCreateInfo::default()
            .size(info.size as u64)
            .usage(
                ash::vk::BufferUsageFlags::TRANSFER_SRC | 
                ash::vk::BufferUsageFlags::TRANSFER_DST | 
                ash::vk::BufferUsageFlags::STORAGE_BUFFER
            )
            .sharing_mode(ash::vk::SharingMode::CONCURRENT)
            .queue_family_indices(&queue_families);
        let allocation_create_info = match info.transfer_mode {
            TransferMode::Auto |
            TransferMode::AutoUpload |
            TransferMode::AutoDownload => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
                ..Default::default()
            },
            TransferMode::Stream => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
                flags: AllocationCreateFlags::MAPPED
                     | AllocationCreateFlags::HOST_ACCESS_RANDOM,
                ..Default::default()
            }
        };

        let descriptor_index = match device.buffer_descriptors.free_descriptors.pop() {
            Some(index) => index,
            None => {
                let index = device.buffer_descriptors.next_descriptor;
                device.buffer_descriptors.next_descriptor += 1;
                index
            }
        };

        let (buffer, allocation) = unsafe {
            device.allocator.create_buffer(&buffer_create_info, &allocation_create_info)? 
        };
        let host_entity = device.world.spawn(
            Buffer { inner: buffer, allocation, descriptor_index, info }
        ).id();

        let buffer_info = [
            ash::vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .range(ash::vk::WHOLE_SIZE)
        ];
        unsafe { device.update_descriptor_sets(
            &[
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(device.descriptor_set)
                    .dst_binding(BUFFER_BINDING)
                    .dst_array_element(descriptor_index)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_info)
            ], 
            &[]
        ); }

        #[cfg(debug_assertions)]
        unsafe {
            let buffer_name = std::ffi::CString::new(format!("Storage Buffer: {}", info.debug_name))?;
            let buffer_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(buffer)
                .object_name(&buffer_name);
            self.active_device.debug_utils.set_debug_utils_object_name(&buffer_name_info)?;
        }

        Ok(BufferHandle { host_entity })
    }

    pub fn get_buffer(&self, handle: &BufferHandle) -> Result<&Buffer> {
        self.active_device.world.get::<Buffer>(handle.host_entity)
            .context("Buffer not found.")
    }

    pub fn get_buffer_memory<T>(&self, handle: &BufferHandle) -> Result<&T> {
        let device = &self.active_device;

        let Some(buffer) = device.world.get::<Buffer>(handle.host_entity) else {
            bail!("Buffer not found.")
        };

        match buffer.info.transfer_mode {
            TransferMode::Auto |
            TransferMode::AutoUpload |
            TransferMode::AutoDownload => todo!(),
            TransferMode::Stream => unsafe {
                if size_of::<T>() != buffer.info.size {
                    bail!("Size of {} does not match buffer size.", type_name::<T>())
                }

                let data_ptr = device.allocator.get_allocation_info(&buffer.allocation).mapped_data;
                let data = data_ptr as *const T;

                Ok(&*data)
            }
        }
    }

    pub fn get_buffer_memory_mut<T>(&mut self, handle: &mut BufferHandle) -> Result<&mut T> {
        let device = &self.active_device;

        let Some(buffer) = device.world.get::<Buffer>(handle.host_entity) else {
            bail!("Buffer not found.")
        };

        match buffer.info.transfer_mode {
            TransferMode::Auto |
            TransferMode::AutoUpload |
            TransferMode::AutoDownload => todo!(),
            TransferMode::Stream => unsafe {
                if size_of::<T>() != buffer.info.size {
                    bail!("Size of {} does not match buffer size.", type_name::<T>())
                }

                let data_ptr = device.allocator.get_allocation_info(&buffer.allocation).mapped_data;
                let data = data_ptr as *mut T;

                Ok(&mut *data)
            }
        }
    }

    pub fn destroy_buffer(&mut self, handle: BufferHandle) -> Result<()> {
        let device = &mut self.active_device;

        let mut entity = device.world.entity_mut(handle.host_entity);
        let Some(mut buffer) = entity.take::<Buffer>() else {
            bail!("Buffer not found.")
        };
        device.world.despawn(handle.host_entity);

        let buffer_info = [
            ash::vk::DescriptorBufferInfo::default()
                .buffer(ash::vk::Buffer::null())
                .range(ash::vk::WHOLE_SIZE)
        ];
    
        device.buffer_descriptors.free_descriptors.push(buffer.descriptor_index);
        unsafe {
            device.allocator.destroy_buffer(buffer.inner, &mut buffer.allocation);
            device.update_descriptor_sets(
                &[
                    ash::vk::WriteDescriptorSet::default()
                        .dst_set(device.descriptor_set)
                        .dst_binding(BUFFER_BINDING)
                        .dst_array_element(buffer.descriptor_index)
                        .descriptor_count(1)
                        .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(&buffer_info)
                ], 
                &[]
            );
        } 

        let success = device.world.despawn(handle.host_entity);

        Ok(())
    }
}