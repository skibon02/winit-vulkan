use ash::vk;
use ash::vk::Extent2D;
use log::info;
use super::VulkanBackend;

pub struct SwapchainWrapper {
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,

    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,

    device: ash::Device,
}

impl<'a> SwapchainWrapper {
    pub fn new(vulkan_backend: &VulkanBackend) -> anyhow::Result<SwapchainWrapper> {
        let device = &vulkan_backend.device;
        let surface_loader = &vulkan_backend.surface_loader;
        let physical_device = vulkan_backend.physical_device;
        let surface = vulkan_backend.surface;

        let surface_capabilities = unsafe { surface_loader.get_physical_device_surface_capabilities(physical_device, surface).unwrap() };
        let surface_formats = unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface).unwrap() };
        let surface_present_modes = unsafe { surface_loader.get_physical_device_surface_present_modes(physical_device, surface).unwrap() };

        //prefer VK_FORMAT_B8G8R8A8_UNORM and VK_COLOR_SPACE_SRGB_NONLINEAR_KHR
        let surface_format = surface_formats.iter().find(|f| {
            f.format == vk::Format::B8G8R8A8_UNORM && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        }).unwrap_or_else(|| {
            surface_formats.first().unwrap()
        });
        //prefer MAILBOX then IMMEDIATE or default FIFO
        let present_mode = surface_present_modes.iter().find(|m| {
            **m == vk::PresentModeKHR::MAILBOX
        }).unwrap_or_else(|| {
            surface_present_modes.iter().find(|m| {
                **m == vk::PresentModeKHR::IMMEDIATE
            }).unwrap_or_else(|| {
                surface_present_modes.first().unwrap()
            })
        });

        // 1 additional image, so we can acquire 2 images at a time.
        let image_count = surface_capabilities.min_image_count + 1;
        info!("\tCreating swapchain...\nPresent mode: {:?}\nSwapchain image count: {:?}, Color space: {:?}, Image formate: {:?}\n", present_mode, image_count, surface_format.color_space, surface_format.format);

        let extent = vulkan_backend.surface_resolution;

        let swapchain_extent = if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            let mut actual_extent = vk::Extent2D::default()
                .width(extent.width)
                .height(extent.height);
            actual_extent.width = actual_extent.width.max(surface_capabilities.min_image_extent.width).min(surface_capabilities.max_image_extent.width);
            actual_extent.height = actual_extent.height.max(surface_capabilities.min_image_extent.height).min(surface_capabilities.max_image_extent.height);
            actual_extent
        };


        let swapchain_loader = ash::khr::swapchain::Device::new(&vulkan_backend.instance, device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(vulkan_backend.surface)
            .min_image_count(image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(swapchain_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(*present_mode)
            .clipped(true);

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None).unwrap() };
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };

        let swapchain_image_views = swapchain_images.iter().map(|image| {
            let image_view_create_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping::default()
                    .r(vk::ComponentSwizzle::IDENTITY)
                    .g(vk::ComponentSwizzle::IDENTITY)
                    .b(vk::ComponentSwizzle::IDENTITY)
                    .a(vk::ComponentSwizzle::IDENTITY))
                .subresource_range(vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1));
            unsafe { device.create_image_view(&image_view_create_info, None).unwrap() }
        }).collect::<Vec<_>>();

        // swapchain and image views are created

        let render_pass = {
            let color_attachments = [vk::AttachmentDescription::default()
                .format(surface_format.format)
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

        let framebuffers = swapchain_image_views.iter().map(|image_view| {
            let attachments = [*image_view];

            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(swapchain_extent.width)
                .height(swapchain_extent.height)
                .layers(1);
            unsafe { device.create_framebuffer(&framebuffer_create_info, None).unwrap() }
        }).collect::<Vec<_>>();
        
        Ok(SwapchainWrapper {
            swapchain,
            swapchain_loader,
            swapchain_images,
            swapchain_image_views,
            swapchain_format: surface_format.format,
            swapchain_extent,

            framebuffers,
            render_pass,

            device: device.clone()
        })
    }

    pub unsafe fn destroy(&mut self) {
        unsafe {
            let device = &self.device;
            for framebuffer in self.framebuffers.iter() {
                device.destroy_framebuffer(*framebuffer, None);
            }
            device.destroy_render_pass(self.render_pass, None);
            for image_view in self.swapchain_image_views.iter() {
                device.destroy_image_view(*image_view, None);
            }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}
