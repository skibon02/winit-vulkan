use ash::vk;
use ash::vk::{CommandBuffer, CommandBufferAllocateInfo, CommandPool};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;

pub struct VkCommandPool {
    device: VkDeviceRef,
    command_pool: CommandPool
}

impl VkCommandPool {
    pub fn new(device: VkDeviceRef, queue_family_index: u32) -> VkCommandPool {
        let command_pool = unsafe { device.create_command_pool(&vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER), None)
        }.unwrap();
        Self {
            device,
            command_pool
        }
    }

    pub fn alloc_command_buffers(&self, n: u32) -> Vec<CommandBuffer> {
        let info = CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(n);
        unsafe { self.device.allocate_command_buffers(&info).unwrap() }
    }
}

impl Drop for VkCommandPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_command_pool(self.command_pool, None) };
    }
}