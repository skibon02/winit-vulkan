use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use ash::vk;
use ash::vk::{BufferUsageFlags, CommandBuffer, DescriptorBufferInfo, DescriptorPool, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorType, Extent2D, ImageTiling, PipelineBindPoint, PipelineLayout, SampleCountFlags, ShaderStageFlags, WriteDescriptorSet, WHOLE_SIZE};
use smallvec::SmallVec;
use sparkles_macro::range_event_start;
use crate::util::get_resource;
use crate::util::image::read_image_from_bytes;
use crate::vulkan_backend::wrappers::image::imageview_info_for_image;

pub struct DescriptorSetPool {
    device: VkDeviceRef,

    descriptor_pool: DescriptorPool,

    allocated_sets: u32,
    capacity_sets: u32,

    allocated_uniform_buffers: u32,
    capacity_uniform_buffers: u32,

    allocated_image_samplers: u32,
    capacity_image_samplers: u32,
}

impl DescriptorSetPool {
    pub fn new(device: VkDeviceRef) -> Self {
        let capacity_sets = 10;
        let capacity_uniform_buffers = 10;
        let capacity_image_samplers = 10;

        let pool_sizes = [
            DescriptorPoolSize::default()
                .descriptor_count(capacity_uniform_buffers)
                .ty(DescriptorType::UNIFORM_BUFFER),
            DescriptorPoolSize::default()
                .descriptor_count(capacity_image_samplers)
                .ty(DescriptorType::COMBINED_IMAGE_SAMPLER)];
        let desc_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(capacity_sets)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&desc_pool_info, None)
                .unwrap()
        };

        DescriptorSetPool {
            device,
            descriptor_pool,

            capacity_image_samplers,
            capacity_sets,
            capacity_uniform_buffers,
            
            allocated_image_samplers: 0,
            allocated_sets: 0,
            allocated_uniform_buffers: 0,
        }
    }


    pub fn allocate_descriptor_sets(&mut self, descriptor_set_layout: DescriptorSetLayout, 
        bindings: impl Iterator<Item=(u32, BufferResource)>) -> DescriptorSet {

        let set_layouts = [descriptor_set_layout];
        let alloc_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { self.device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };
        

        let bindings: Vec<_> = bindings.collect();
        
        self.allocated_sets += 1;
        self.allocated_uniform_buffers += bindings.len() as u32;
        if self.allocated_sets > self.capacity_sets || self.allocated_uniform_buffers > self.capacity_uniform_buffers {
            panic!("Descriptor set pool exceeded capacity");
        }
        // Update descriptor set
        let buffer_infos: Vec<_> = bindings.iter().map(|(_, buffer)| {
            [
                DescriptorBufferInfo::default()
                    .offset(0)
                    .buffer(buffer.buffer)
                    .range(WHOLE_SIZE)
            ]
        }).collect();
        // let image_infos: Vec<_> = bindings.iter().filter_map(|binding| {
        //     if let DescriptorSetBindingResource::Image(image, sampler) = binding {
        //
        //         Some([vk::DescriptorImageInfo::default()
        //                 .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        //                 .image_view(*image)
        //                 .sampler(*sampler)
        //         ])
        //     }
        //     else {
        //         None
        //     }
        // }).collect();

        // let mut image_info_i = 0;
        let descriptor_writes: Vec<_> = bindings.iter().enumerate().map(|(i, (binding, _))| {
            let res = WriteDescriptorSet::default()
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .dst_set(descriptor_set)
                .dst_binding(*binding)
                .dst_array_element(0)
                .buffer_info(&buffer_infos[i]);

            res
                // DescriptorSetBindingResource::Image(image, sampler) => {
                //
                //     let res = WriteDescriptorSet::default()
                //         .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                //         .descriptor_count(1)
                //         .dst_set(descriptor_set)
                //         .dst_binding(1)
                //         .dst_array_element(0)
                //         .image_info(&image_infos[image_info_i]);
                //
                //     image_info_i += 1;
                //     res
                // }
        }).collect();

        unsafe { self.device.update_descriptor_sets(&descriptor_writes, &[]) }

        descriptor_set
    }

}

impl Drop for DescriptorSetPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}


pub struct ObjectDescriptorSet {
    device: VkDeviceRef,

    descriptor_set_layout: DescriptorSetLayout,
    descriptor_set: DescriptorSet,
}

impl ObjectDescriptorSet {
    pub fn new(device: VkDeviceRef, descriptor_set_pool: &mut DescriptorSetPool,
           descriptor_set_layout: DescriptorSetLayout, uniform_bindings: impl Iterator<Item=(u32, BufferResource)>) -> ObjectDescriptorSet {
        let g = range_event_start!("[Vulkan] Create descriptor sets");

        // // 2. Create images and buffers
        // let buffer = resource_manager.create_buffer(3 * 4, BufferUsageFlags::UNIFORM_BUFFER);
        // resource_manager.fill_buffer(buffer, &[0.0f32, 0.0, 0.0]);
        // let data = get_resource("resources/damndashie.png".into()).unwrap();
        // let (image_data, extent) = read_image_from_bytes(data).unwrap();
        //
        // let image = resource_manager.create_image(extent, vk::Format::R8G8B8A8_UNORM, ImageTiling::OPTIMAL,
        //                                           vk::ImageUsageFlags::SAMPLED, SampleCountFlags::TYPE_1);
        //
        // resource_manager.fill_image(image, image_data.as_slice());
        //
        // let imageview_info = imageview_info_for_image(image.image, image.info, vk::ImageAspectFlags::COLOR);
        // let imageview = unsafe { device.create_image_view(&imageview_info, None) }.unwrap();
        // let sampler = resource_manager.create_sampler();
        //
        // let bindings = vec![
        //     DescriptorSetBindingResource::Buffer(buffer),
        //     DescriptorSetBindingResource::Image(imageview, sampler)
        // ];

        // Ask pool to allocate descriptor set and perform writes
        let descriptor_set = descriptor_set_pool.allocate_descriptor_sets(descriptor_set_layout, uniform_bindings);
        
        Self {
            device,
            descriptor_set_layout,
            descriptor_set,
        }
    }

    pub fn get_descriptor_set_layout(&self) -> DescriptorSetLayout {
        self.descriptor_set_layout
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

impl Drop for ObjectDescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

        }
    }
}
