use std::mem::take;
use std::sync::Arc;
use ash::{vk, Device};
use ash::vk::{AccessFlags, Buffer, CommandBufferBeginInfo, DescriptorSetLayout, DeviceMemory, Extent2D, Format, Framebuffer, Image, ImageAspectFlags, ImageTiling, ImageUsageFlags, ImageView, MemoryAllocateInfo, MemoryType, PipelineBindPoint, PipelineStageFlags, PrimitiveTopology, RenderPass, RenderPassBeginInfo, SampleCountFlags};
use sparkles_macro::range_event_start;
use crate::use_shader;
use crate::vulkan_backend::descriptor_sets::DescriptorSets;
use crate::vulkan_backend::helpers::image::{image_2d_info, imageview_info_for_image};
use crate::vulkan_backend::pipeline::{PipelineDesc, VertexInputDesc, VulkanPipeline};
use crate::vulkan_backend::resource_manager::{ImageResource, ResourceManager};

pub struct RenderPassResources {
    device: Arc<Device>,
    pub framebuffers: Vec<Framebuffer>,
    pub depth_images: Vec<ImageResource>,
    pub depth_image_views: Vec<ImageView>,
}

impl RenderPassResources {
    pub unsafe fn destroy(&mut self, resource_manager: &mut ResourceManager) {
        // framebuffers
        for framebuffer in self.framebuffers.drain(..) {
            unsafe { self.device.destroy_framebuffer(framebuffer, None); }
        }

        for depth_imageview in self.depth_image_views.drain(..) {
            unsafe { self.device.destroy_image_view(depth_imageview, None)};
        }
        for depth_image in self.depth_images.drain(..) {
            resource_manager.destroy_image(depth_image);
        }
    }
}

pub struct RenderPassWrapper {
    device: Arc<Device>,
    render_pass: RenderPass,
    descriptor_sets: DescriptorSets,
    pipeline: VulkanPipeline,
}

impl RenderPassWrapper {
    pub fn new(device: Arc<Device>, surface_format: Format, resource_manager: &mut ResourceManager) -> Self {
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

        let descriptor_sets = DescriptorSets::new(device.clone(), resource_manager);
        let pipeline_desc = PipelineDesc::new(use_shader!("solid"));
        let vert_desc = VertexInputDesc::new(PrimitiveTopology::TRIANGLE_LIST)
            .attrib_3_floats()  // 0: Pos 3D
            .attrib_3_floats();               // 1: Normal 3D
        let pipeline = VulkanPipeline::new(device.clone(), &render_pass, pipeline_desc, vert_desc, descriptor_sets.get_descriptor_set_layout());

        Self {
            device,

            render_pass,
            pipeline,
            descriptor_sets
        }
    }

    pub fn update(&mut self, resource_manager: &mut ResourceManager, color: [f32; 3]) {
        self.descriptor_sets.update(resource_manager, color);
    }
    pub fn create_render_pass_resources(&self, image_views: Vec<ImageView>, extent: Extent2D, resource_manager: &mut ResourceManager) -> RenderPassResources {
        // create imageviews for depth attachments
        let depth_images: Vec<_> = (0..image_views.len()).map(|_| {
            resource_manager.create_image(extent, Format::D16_UNORM, ImageTiling::OPTIMAL, ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        }).collect();

        let depth_image_views: Vec<_> = depth_images.iter().map(|image| {
            let img_info = image.info;
            let info = imageview_info_for_image(image.image, img_info, ImageAspectFlags::DEPTH);
            unsafe { self.device.create_image_view(&info, None).unwrap() }
        }).collect();

        let framebuffers = image_views.into_iter().zip(depth_image_views.iter()).map(|(image_view, depth_imageview)| {
            let attachments = [image_view, *depth_imageview];

            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(self.render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            unsafe { self.device.create_framebuffer(&framebuffer_create_info, None).unwrap() }
        }).collect::<Vec<_>>();

        RenderPassResources {
            device: self.device.clone(),
            depth_images,
            depth_image_views,
            framebuffers,
        }
    }

    pub fn record_draw(&mut self, device: &Device, command_buffer: vk::CommandBuffer, framebuffer: Framebuffer, vertex_buffer: Buffer, extent: Extent2D) {
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
            //bind
            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline.get_pipeline());
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
            let sets = [self.descriptor_sets.get_set()];
            device.cmd_bind_descriptor_sets(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline.get_pipeline_layout(), 0, &sets, &[]);
            //draw
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer).unwrap();
        }
    }

    pub unsafe fn destroy(&mut self) {
        unsafe { self.pipeline.destroy(); }
        unsafe { self.descriptor_sets.destroy() };
        //render pass
        unsafe { self.device.destroy_render_pass(self.render_pass, None); }
    }
}