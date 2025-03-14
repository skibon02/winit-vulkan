use ash::vk;
use ash::khr::swapchain;
use ash::vk::{Extent2D, Format, Image, ImageAspectFlags, ImageTiling, ImageUsageFlags, ImageView, PhysicalDevice, PresentModeKHR, SampleCountFlags, SwapchainKHR};
use log::info;
use sparkles_macro::range_event_start;
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use crate::vulkan_backend::wrappers::image::{image_2d_info, imageview_info_for_image, swapchain_info};
use crate::vulkan_backend::wrappers::surface::VkSurfaceRef;

pub struct SwapchainWrapper {
    swapchain: SwapchainKHR,
    pub swapchain_loader: swapchain::Device,
    pub swapchain_images: Vec<Image>,
    swapchain_image_views: Vec<ImageView>,
    swapchain_format: Format,
    pub swapchain_extent: Extent2D,

    device: VkDeviceRef,
    surface: VkSurfaceRef
}

impl SwapchainWrapper {
    pub fn new(device: VkDeviceRef, physical_device: PhysicalDevice,
               extent: Extent2D, surface_ref: VkSurfaceRef, old_swapchain: Option<SwapchainKHR>) -> anyhow::Result<SwapchainWrapper> {
        let g = range_event_start!("[Vulkan] Init swapchain");

        let surface_loader = surface_ref.loader();
        let surface = surface_ref.surface();

        // TODO: replace with wrapper methods
        let surface_capabilities = unsafe { surface_loader.get_physical_device_surface_capabilities(physical_device, *surface)? };
        let surface_formats = unsafe { surface_loader.get_physical_device_surface_formats(physical_device, *surface)? };
        let surface_present_modes = unsafe { surface_loader.get_physical_device_surface_present_modes(physical_device, *surface)? };

        //prefer B8G8R8A8_UNORM and SRGB_NONLINEAR
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
        // let present_mode = &PresentModeKHR::FIFO;

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


        let swapchain_loader = swapchain::Device::new(device.instance(), &device);
        let swapchain_image_info = image_2d_info(surface_format.format, ImageUsageFlags::COLOR_ATTACHMENT,
                                             swapchain_extent, SampleCountFlags::TYPE_1, ImageTiling::OPTIMAL);
        let swapchain_create_info = swapchain_info(swapchain_image_info, surface_format.color_space)
            .surface(*surface)
            .min_image_count(image_count)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(*present_mode)
            .clipped(true);

        // add old swapchain
        let swapchain_create_info = if let Some(old_swapchain) = old_swapchain {
            swapchain_create_info.old_swapchain(old_swapchain)
        }
        else {
            swapchain_create_info
        };

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        let swapchain_image_views = swapchain_images.iter().map(|image| {
            let image_view_create_info = imageview_info_for_image(*image, swapchain_image_info, ImageAspectFlags::COLOR);
            unsafe { device.create_image_view(&image_view_create_info, None).unwrap() }
        }).collect::<Vec<_>>();
        
        Ok(SwapchainWrapper {
            swapchain,
            swapchain_loader,
            swapchain_images,
            swapchain_image_views,
            swapchain_format: surface_format.format,
            swapchain_extent,

            device,
            surface: surface_ref
        })
    }

    pub fn get_swapchain(&self) -> SwapchainKHR {
        self.swapchain
    }
    pub fn get_image_views(&self) -> Vec<ImageView> {
        self.swapchain_image_views.clone()
    }

    pub fn get_surface_format(&self) -> Format {
        self.swapchain_format
    }

    pub fn get_extent(&self) -> Extent2D {
        self.swapchain_extent
    }


    /// # Safety
    /// Image views should not be used. Swapchain should not be used.
    pub unsafe fn recreate(&mut self, physical_device: PhysicalDevice,
                           extent: Extent2D, surface: VkSurfaceRef) -> anyhow::Result<()> {

        let swapchain = self.swapchain;
        *self = Self::new(self.device.clone(), physical_device, extent, surface, Some(swapchain))?;
        Ok(())
    }
}

impl Drop for SwapchainWrapper {
    fn drop(&mut self) {
        let g = range_event_start!("[Vulkan] Destroy swapchain");
        for image_view in self.swapchain_image_views.iter() {
            unsafe { self.device.destroy_image_view(*image_view, None); }
        }
        unsafe { self.swapchain_loader.destroy_swapchain(self.swapchain, None); }
    }
}