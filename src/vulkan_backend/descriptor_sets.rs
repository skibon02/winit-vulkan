use ash::{vk};
use ash::vk::{BufferUsageFlags, DescriptorBufferInfo, DescriptorPool, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags, DescriptorType, DeviceMemory, ShaderStageFlags, WriteDescriptorSet, WHOLE_SIZE};
use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;

pub struct DescriptorSets {
    device: VkDeviceRef,

    descriptor_set_layout: DescriptorSetLayout,
    descriptor_set: DescriptorSet,
    descriptor_pool: DescriptorPool,
    buffer: BufferResource
}

impl DescriptorSets {
    pub fn new(device: VkDeviceRef, resource_manager: &mut ResourceManager) -> DescriptorSets {
        // 1. Create layout
        let bindings = [DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .stage_flags(ShaderStageFlags::FRAGMENT)];
        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings);

        let descriptor_set_layout = unsafe {device.create_descriptor_set_layout(&descriptor_set_layout_info, None).unwrap()};

        // 2. Create Descriptor set
        let pool_sizes = [DescriptorPoolSize::default().descriptor_count(1).ty(DescriptorType::UNIFORM_BUFFER)];
        let desc_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {device.create_descriptor_pool(&desc_pool_info, None).unwrap()};

        let set_layouts = [descriptor_set_layout];
        let alloc_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };

        let buffer = resource_manager.create_buffer(3*4, BufferUsageFlags::UNIFORM_BUFFER);
        resource_manager.fill_buffer(buffer, &[0.0f32, 0.0, 0.0]);

        let buffer_info = [DescriptorBufferInfo::default()
            .offset(0)
            .buffer(buffer.buffer)
            .range(WHOLE_SIZE)];
        let descriptor_write = WriteDescriptorSet::default()
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .buffer_info(&buffer_info);
        unsafe { device.update_descriptor_sets(&[descriptor_write], &[]) }

        Self {
            device,
            descriptor_set_layout,
            descriptor_set,
            descriptor_pool,
            buffer
        }
    }

    pub fn get_descriptor_set_layout(&self) -> DescriptorSetLayout {
        self.descriptor_set_layout
    }
    pub fn update(&mut self, resource_manager: &mut ResourceManager, new_color: [f32; 3]) {
        resource_manager.fill_buffer(self.buffer, &new_color);
    }

    pub fn get_set(&self) -> DescriptorSet {
        self.descriptor_set
    }

    pub unsafe fn destroy(&self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}