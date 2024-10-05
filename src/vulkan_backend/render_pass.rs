use ash::{vk, Device};
use ash::vk::{CommandBufferBeginInfo, Extent2D, Format, Framebuffer, ImageView, PipelineBindPoint, RenderPass, RenderPassBeginInfo};
use sparkles_macro::range_event_start;
use crate::vulkan_backend::pipeline::TrianglePipeline;

pub struct RenderPassWrapper {
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,

    extent: Extent2D,

    pipeline: TrianglePipeline,
}

impl RenderPassWrapper {
    pub fn new(device: &Device, surface_format: Format, extent: Extent2D, image_views: impl Iterator<Item=ImageView>) -> Self {
        let g = range_event_start!("Create render pass");
        let render_pass = {
            let color_attachments = [vk::AttachmentDescription::default()
                .format(surface_format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
            let color_attachment_refs = [vk::AttachmentReference::default()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
            let subpasses = [vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&color_attachment_refs)];
            let dependencies = [vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
            let render_pass_create_info = vk::RenderPassCreateInfo::default()
                .attachments(&color_attachments)
                .subpasses(&subpasses)
                .dependencies(&dependencies);
            unsafe { device.create_render_pass(&render_pass_create_info, None).unwrap() }
        };

        let framebuffers = image_views.map(|image_view| {
            let attachments = [image_view];

            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            unsafe { device.create_framebuffer(&framebuffer_create_info, None).unwrap() }
        }).collect::<Vec<_>>();

        let pipeline = TrianglePipeline::new(&device, &render_pass);

        Self {
            render_pass,
            framebuffers,

            extent,
            pipeline
        }
    }

    pub fn record_draw(&mut self, device: &Device, image_index: u32, command_buffer: vk::CommandBuffer) {
        let g = range_event_start!("[Vulkan] Command buffer recording");
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[image_index as usize])
            .render_area(self.extent.into())
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.8, 0.4, 0.7, 1.0],
                },
            }]);

        let viewport = vk::Viewport::default()
            .width(self.extent.width as f32)
            .height(self.extent.height as f32);
        let scissors = self.extent.into();
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

    pub fn destroy(&mut self, device: &Device) {
        unsafe { self.pipeline.destroy(device); }
        //framebuffers
        for framebuffer in self.framebuffers.drain(..) {
            unsafe { device.destroy_framebuffer(framebuffer, None); }
        }
        //render pass
        unsafe { device.destroy_render_pass(self.render_pass, None); }
    }
}