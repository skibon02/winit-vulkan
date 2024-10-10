use std::mem::take;
use ash::{vk, Device};
use ash::vk::{AccessFlags, CommandBufferBeginInfo, DeviceMemory, Extent2D, Format, Framebuffer, Image, ImageAspectFlags, ImageUsageFlags, ImageView, MemoryAllocateInfo, MemoryType, PipelineBindPoint, PipelineStageFlags, RenderPass, RenderPassBeginInfo, SampleCountFlags};
use sparkles_macro::range_event_start;
use crate::use_shader;
use crate::vulkan_backend::helpers::image::{image_2d_info, imageview_info_for_image};
use crate::vulkan_backend::pipeline::{PipelineDesc, VulkanPipeline};

pub struct RenderPassResources {
    pub framebuffers: Vec<Framebuffer>,
    pub depth_images_memory: Vec<DeviceMemory>,
    pub depth_images: Vec<Image>,
    pub depth_image_views: Vec<ImageView>,
}

impl RenderPassResources {
    pub unsafe fn destroy(&mut self, device: &Device) {
        // framebuffers
        for framebuffer in self.framebuffers.drain(..) {
            unsafe { device.destroy_framebuffer(framebuffer, None); }
        }

        // depth buffer things
        for image_view in take(&mut self.depth_image_views) {
            unsafe { device.destroy_image_view(image_view, None) };
        }
        for memory in take(&mut self.depth_images_memory) {
            unsafe { device.free_memory(memory, None) };
        }
        for image in take(&mut self.depth_images) {
            unsafe { device.destroy_image(image, None) };
        }
    }
}

pub struct RenderPassWrapper {
    render_pass: RenderPass,
    pipeline: VulkanPipeline,
}

impl RenderPassWrapper {
    pub fn new(device: &Device, surface_format: Format) -> Self {
        let g = range_event_start!("Create render pass");

        let render_pass = {
            let attachments = [
                // 0. final color attachment
                vk::AttachmentDescription::default()
                    .format(surface_format)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),

                // 1. depth attachment
                vk::AttachmentDescription::default()
                    .format(Format::D16_UNORM)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)];

            let color_attachment_refs = [vk::AttachmentReference::default()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
            let depth_attachment_ref = vk::AttachmentReference::default()
                .attachment(1)
                .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            let subpasses = [vk::SubpassDescription::default()
                .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
                .color_attachments(&color_attachment_refs)
                .depth_stencil_attachment(&depth_attachment_ref)];
            let dependencies = [vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .src_access_mask(AccessFlags::empty())
                .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)];
            let render_pass_create_info = vk::RenderPassCreateInfo::default()
                .attachments(&attachments)
                .subpasses(&subpasses)
                .dependencies(&dependencies);
            unsafe { device.create_render_pass(&render_pass_create_info, None).unwrap() }
        };

        let pipeline_desc = PipelineDesc::new(use_shader!("solid"));
        let pipeline = VulkanPipeline::new(device, &render_pass, pipeline_desc);

        Self {
            render_pass,
            pipeline
        }
    }

    pub fn create_render_pass_resources(&self, device: &Device, image_views: Vec<ImageView>, extent: Extent2D, mem_types: &[MemoryType]) -> RenderPassResources {
        // create imageviews for depth attachments
        let depth_image_create_info = image_2d_info(Format::D16_UNORM, ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                                                    extent, SampleCountFlags::TYPE_1);

        let depth_images: Vec<_> = (0..image_views.len()).map(|_| {
            unsafe { device.create_image(&depth_image_create_info, None) }.unwrap()
        }).collect();


        let depth_images_memory_reqs = unsafe { device.get_image_memory_requirements(depth_images[0]) };

        let mem_type_i = mem_types.iter().enumerate().position(|(i, memory_type)| {
            depth_images_memory_reqs.memory_type_bits & (1 << i) != 0 && memory_type.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
        }).unwrap();
        let alloc_info = MemoryAllocateInfo::default()
            .allocation_size(depth_images_memory_reqs.size)
            .memory_type_index(mem_type_i as u32);

        let depth_images_memory = (0..depth_images.len()).map(|_| {
            unsafe { device.allocate_memory(&alloc_info, None) }.unwrap()
        }).collect::<Vec<_>>();

        for (image, memory) in depth_images.iter().zip(depth_images_memory.iter()) {
            unsafe { device.bind_image_memory(*image, *memory, 0).unwrap() }
        }

        let depth_image_views: Vec<_> = depth_images.iter().map(|image| {
            let imageview_info =
                imageview_info_for_image(*image, depth_image_create_info, ImageAspectFlags::DEPTH);

            unsafe {device.create_image_view(&imageview_info, None).unwrap() }
        }).collect();

        let framebuffers = image_views.into_iter().zip(depth_image_views.iter()).map(|(image_view, depth_imageview)| {
            let attachments = [image_view, *depth_imageview];

            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(self.render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            unsafe { device.create_framebuffer(&framebuffer_create_info, None).unwrap() }
        }).collect::<Vec<_>>();

        RenderPassResources {
            depth_images_memory,
            depth_images,
            depth_image_views,
            framebuffers
        }
    }

    pub fn record_draw(&mut self, device: &Device, command_buffer: vk::CommandBuffer, framebuffer: Framebuffer, extent: Extent2D) {
        let g = range_event_start!("[Vulkan] Command buffer recording");
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(framebuffer)
            .render_area(extent.into())
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.8, 0.4, 0.7, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            }]);

        let viewport = vk::Viewport::default()
            .width(extent.width as f32)
            .height(extent.height as f32);
        let scissors = extent.into();
        unsafe {
            device.begin_command_buffer(command_buffer, &command_buffer_begin_info).unwrap();
            device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);

            //bind dynamic states
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissors]);
            //draw
            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline.get_pipeline());
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer).unwrap();
        }
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        unsafe { self.pipeline.destroy(device); }
        //render pass
        unsafe { device.destroy_render_pass(self.render_pass, None); }
    }
}