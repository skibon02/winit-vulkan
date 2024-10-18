use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use ash::vk;
use ash::vk::{BufferUsageFlags, CommandBuffer, DescriptorBufferInfo, DescriptorPool, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorType, Extent2D, ImageTiling, PipelineBindPoint, PipelineLayout, SampleCountFlags, ShaderStageFlags, WriteDescriptorSet, WHOLE_SIZE};
use sparkles_macro::range_event_start;
use crate::util::image::read_image_from_file;
use crate::vulkan_backend::wrappers::image::imageview_info_for_image;

pub struct DescriptorSets {
    device: VkDeviceRef,

    descriptor_set_layout: DescriptorSetLayout,
    descriptor_pool: DescriptorPool,

    descriptor_set: DescriptorSet,

    buffer: BufferResource,
    imageview: vk::ImageView,
    sampler: vk::Sampler,
}

impl DescriptorSets {
    pub fn new(device: VkDeviceRef, resource_manager: &mut ResourceManager) -> DescriptorSets {
        let g = range_event_start!("[Vulkan] Create descriptor sets");
        
        // 1. Create layout
        let bindings = [
            DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .stage_flags(ShaderStageFlags::FRAGMENT),

            DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(ShaderStageFlags::FRAGMENT)];
        let descriptor_set_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .unwrap()
        };

        // 2. Create Descriptor set
        let pool_sizes = [
            DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(DescriptorType::UNIFORM_BUFFER),
            DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(DescriptorType::COMBINED_IMAGE_SAMPLER)];
        let desc_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&desc_pool_info, None)
                .unwrap()
        };

        let set_layouts = [descriptor_set_layout];
        let alloc_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };

        // Create resources
        let buffer = resource_manager.create_buffer(3 * 4, BufferUsageFlags::UNIFORM_BUFFER);
        resource_manager.fill_buffer(buffer, &[0.0f32, 0.0, 0.0]);
        let (image_data, extent) = read_image_from_file("resources/damndashie.png").unwrap();

        let image = resource_manager.create_image(extent, vk::Format::R8G8B8A8_UNORM, ImageTiling::OPTIMAL,
                                                  vk::ImageUsageFlags::SAMPLED, SampleCountFlags::TYPE_1);

        resource_manager.fill_image(image, image_data.as_slice());

        let imageview_info = imageview_info_for_image(image.image, image.info, vk::ImageAspectFlags::COLOR);
        let imageview = unsafe { device.create_image_view(&imageview_info, None) }.unwrap();
        let sampler = resource_manager.create_sampler();

        // Update descriptor set
        let buffer_info = [DescriptorBufferInfo::default()
            .offset(0)
            .buffer(buffer.buffer)
            .range(WHOLE_SIZE)];
        let image_info = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(imageview)
            .sampler(sampler)];

        let descriptor_writes = [
            WriteDescriptorSet::default()
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .buffer_info(&buffer_info),

            WriteDescriptorSet::default()
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .dst_set(descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .image_info(&image_info),
        ];
        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) }

        Self {
            device,
            descriptor_set_layout,
            descriptor_set,
            descriptor_pool,

            buffer,
            imageview,
            sampler
        }
    }

    pub fn get_descriptor_set_layout(&self) -> DescriptorSetLayout {
        self.descriptor_set_layout
    }
    pub fn update(&mut self, resource_manager: &mut ResourceManager, new_color: [f32; 3]) {
        resource_manager.fill_buffer(self.buffer, &new_color);
    }

    pub fn bind_sets(&self, command_buffer: CommandBuffer, pipeline_layout: PipelineLayout) {
        let descriptor_sets = [self.descriptor_set];
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            );
        }
    }
}

impl Drop for DescriptorSets {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            self.device.destroy_image_view(self.imageview, None);
        }
    }
}
