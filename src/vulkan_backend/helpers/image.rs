use ash::vk;
use ash::vk::{ColorSpaceKHR, Extent2D, Extent3D, Format, Image, ImageAspectFlags, ImageCreateInfo, ImageSubresourceRange, ImageType, ImageUsageFlags, ImageViewCreateInfo, SampleCountFlags, SurfaceFormatKHR, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainCreateInfoKHR};



/// Generate create info for a simple 2d image
/// - 1 layer
/// - 1 mip level
/// - empty flags
/// - sharing mode exclusive
/// - tiling optimal
/// - initial layout: Undefined
/// - type 2d
pub fn image_2d_info<'a>(format: Format, usage: ImageUsageFlags, extent: impl Into<Extent3D>, samples: SampleCountFlags) -> ImageCreateInfo<'a> {
    vk::ImageCreateInfo::default()
        .format(format)
        .usage(usage)
        .extent(extent.into())
        .samples(samples)

        .array_layers(1)
        .flags(vk::ImageCreateFlags::empty())
        .mip_levels(1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .image_type(vk::ImageType::TYPE_2D)
}

/// Generate imageview create info for a simple 2d image
/// - 1 layer from layer 0
/// - 1 mip level from layer 0
/// - empty flags
/// - type same as input image
/// - format same as input image
pub fn imageview_info_for_image(image: Image, info: ImageCreateInfo, aspect: ImageAspectFlags) -> ImageViewCreateInfo {
    let imageview_type = match info.image_type {
        ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
        ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
        ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
        _ => unimplemented!("Unknown ImageType value: {:?}", info.image_type),
    };

    let imageview_info = vk::ImageViewCreateInfo::default()
        .format(info.format)
        .components(vk::ComponentMapping::default())
        .image(image)
        .view_type(imageview_type)
        .subresource_range(ImageSubresourceRange::default()
            .aspect_mask(aspect)
            .layer_count(1)
            .level_count(1));

    imageview_info
}

/// Fill in 
pub fn swapchain_info(image_info: ImageCreateInfo, color_space: ColorSpaceKHR) -> SwapchainCreateInfoKHR {
    vk::SwapchainCreateInfoKHR::default()
        .image_color_space(color_space)
        .image_format(image_info.format)
        .image_extent(Extent2D {width: image_info.extent.width, height: image_info.extent.height})
        .image_array_layers(image_info.array_layers)
        .image_sharing_mode(image_info.sharing_mode)
        .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
}