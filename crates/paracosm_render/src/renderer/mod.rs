use ash::vk;

use bevy_window::Window;

use paracosm_gpu::{Device, Instance};

type RenderContext = (Device, vk::Queue, SyncStructures);

pub fn initialize_renderer(
    instance: Instance, 
    window: &Window
) -> Result<RenderContext, String> {
    // Create Device
    let device = match Device::primary(instance, Some(window)) {
        Ok(result) => result,
        Err(error) => return Err(error.to_string()),
    };

    // Get first Graphics queue
    let queue = unsafe { device.get_device_queue(device.queues.graphics_family, 0) };

    // Create sync structures
    let create_info = vk::FenceCreateInfo::builder()
        .flags(vk::FenceCreateFlags::SIGNALED);
    let render_fence = match unsafe { device.create_fence(&create_info, None) } {
        Ok(result) => result,
        Err(error) => return Err(error.to_string())
    };
    let create_info = vk::SemaphoreCreateInfo::builder()
        .flags(vk::SemaphoreCreateFlags::empty());
    let render_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
        Ok(result) => result,
        Err(error) => return Err(error.to_string())
    };
    let present_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
        Ok(result) => result,
        Err(error) => return Err(error.to_string())
    };

    let sync_structures = SyncStructures {
        device: device.clone(),
        render_fence,
        render_semaphore,
        present_semaphore
    };

    Ok((device, queue, sync_structures))
}

// TODO: Maybe replace this with abstraction(s) in paracosm_gpu
pub struct SyncStructures {
    device: Device,
    pub render_fence: vk::Fence,
    pub render_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore
}

impl Drop for SyncStructures {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_semaphore(self.present_semaphore, None);
            self.device.destroy_semaphore(self.render_semaphore, None);
            self.device.destroy_fence(self.render_fence, None);
        }
    }
}
