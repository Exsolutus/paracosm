use ash::vk;

use gpu_allocator::vulkan::Allocation;

#[derive(Debug)]
pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Allocation,
}