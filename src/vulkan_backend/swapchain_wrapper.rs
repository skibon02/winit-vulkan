use ash::{vk, Device, Instance};
use ash::vk::{Extent2D, Format, Image, ImageView, PhysicalDevice, SurfaceKHR, SwapchainKHR};
use log::info;
use sparkles_macro::range_event_start;

pub struct SwapchainWrapper {
    pub swapchain: SwapchainKHR,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain_images: Vec<Image>,
    swapchain_image_views: Vec<ImageView>,
    swapchain_format: Format,
    pub swapchain_extent: Extent2D,

    device: Device,
}

impl<'a> SwapchainWrapper {
    pub fn new(instance: &Instance, device: &Device, physical_device: PhysicalDevice, extent: Extent2D, surface: SurfaceKHR, surface_loader: &ash::khr::surface::Instance) -> anyhow::Result<SwapchainWrapper> {
        let g = range_event_start!("[Vulkan] Init swapchain");

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
        info!("\n\tCreating swapchain...\n\tPresent mode: {:?}\n\tSwapchain image count: {:?}, Color space: {:?}, Image formate: {:?}", present_mode, image_count, surface_format.color_space, surface_format.format);

        let swapchain_extent = if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            let mut actual_extent = extent;
            actual_extent.width = actual_extent.width.max(surface_capabilities.min_image_extent.width).min(surface_capabilities.max_image_extent.width);
            actual_extent.height = actual_extent.height.max(surface_capabilities.min_image_extent.height).min(surface_capabilities.max_image_extent.height);
            actual_extent
        };


        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
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
        
        Ok(SwapchainWrapper {
            swapchain,
            swapchain_loader,
            swapchain_images,
            swapchain_image_views,
            swapchain_format: surface_format.format,
            swapchain_extent,

            device: device.clone()
        })
    }

    pub fn get_image_views(&self) -> impl Iterator<Item=ImageView> + '_ {
        self.swapchain_image_views.iter().cloned()
    }

    pub fn get_surface_format(&self) -> vk::Format {
        self.swapchain_format
    }

    pub fn get_resolution(&self) -> vk::Extent2D {
        self.swapchain_extent
    }

    pub unsafe fn destroy(&mut self) {
        unsafe {
            let device = &self.device;
            for image_view in self.swapchain_image_views.iter() {
                device.destroy_image_view(*image_view, None);
            }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}
