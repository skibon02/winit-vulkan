pub mod pipeline{
    use ash::vk::{self, PipelineLayout};
    use vk::Pipeline;


    pub struct TrianglePipeline {
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
    }
}

pub mod swapchain_wrapper;


use crate::helpers::{self, DebugUtilsHelper, CapabilitiesChecker};
use crate::vulkan_backend::swapchain_wrapper::SwapchainWrapper;

use anyhow::Context;
use ash::extensions::khr::Surface;
use ash_window::create_surface;
use log::{info};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::dpi::{PhysicalSize};
use winit::window::Window;

use ash::{Entry, Instance, Device};
use ash::vk::{self, make_api_version, ApplicationInfo, SurfaceKHR, Queue};

use std::ffi::CString;


pub struct VulkanBackend {
    entry: Entry,
    instance: Instance,
    surface_loader: Surface,

    surface: SurfaceKHR,
    surface_resolution: PhysicalSize<u32>,
    debug_utils: helpers::DebugUtilsHelper,

    capabilities_checker: helpers::CapabilitiesChecker,
    physical_device: vk::PhysicalDevice,

    device: Device,
    queue: Queue,
    command_pool: vk::CommandPool,

    swapchain_wrapper: Option<SwapchainWrapper>,
}

impl VulkanBackend {
    // Initialize vulkan resources and use window to create surface
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let entry = Entry::linked();

        let app_name = CString::new("Hello Triangle")?;

        let app_info = ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(make_api_version(0, 1, 0, 0))
            .engine_name(&app_name)
            .engine_version(make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_0);


        //define desired layers
        let mut instance_layers = vec![];
        if cfg!(debug_assertions) {
            instance_layers.push(CString::new("VK_LAYER_KHRONOS_validation")?);
        }
        let instance_layers_refs: Vec<*const i8> = instance_layers.iter().map(|l| l.as_ptr())
            .collect();

        //define desired extensions
        let display_handle = window.raw_display_handle().unwrap();
        let window_handle = window.raw_window_handle().unwrap();

        let surface_required_extensions = ash_window::enumerate_required_extensions(display_handle)?;
        let mut instance_extensions: Vec<*const i8> = 
            surface_required_extensions.to_vec();
        instance_extensions.push(ash::extensions::ext::DebugUtils::name().as_ptr());


        let mut debug_utils_messanger_info = DebugUtilsHelper::get_messenger_create_info();
        let mut create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&instance_layers_refs)
            .enabled_extension_names(&instance_extensions)
            .push_next(&mut debug_utils_messanger_info);

        let mut caps_checker = CapabilitiesChecker::new();

        // caps_checker will check requested layers and extensions for support and enable only the
        // supported ones, so we can request them later
        let instance = caps_checker.create_instance(&entry, &mut create_info)?;

        let surface_loader = Surface::new(&entry, &instance);
        let surface = unsafe { create_surface(&entry, &instance, display_handle, window_handle, None).context("Surface creation")? };
        let surface_resolution = window.inner_size();

        let debug_utils = helpers::DebugUtilsHelper::new(&entry, &instance)?;
        // instance is created. debug utils ready


        let physical_devices = unsafe { instance.enumerate_physical_devices().unwrap() };

        let physical_device = *physical_devices.iter().find(|&d| {
            let properties = unsafe { instance.get_physical_device_properties(*d) };
            properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
        }).or_else(|| {
            physical_devices.iter().find(|&d| {
                let properties = unsafe { instance.get_physical_device_properties(*d) };
                properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
            })
        }).or_else(|| {
            physical_devices.iter().find(|&d| {
                let properties = unsafe { instance.get_physical_device_properties(*d) };
                properties.device_type == vk::PhysicalDeviceType::CPU
            })
        }).unwrap_or_else(|| {
            panic!("No avaliable physical device found");
        });
        
        //select chosen physical device
        let dev_name_array = unsafe { instance.get_physical_device_properties(physical_device).device_name };
        let dev_name = unsafe {std::ffi::CStr::from_ptr(dev_name_array.as_ptr())};
        println!("Chosen device: {}", dev_name.to_str().unwrap());


        let queue_family_properties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let queue_family_index = queue_family_properties.iter().enumerate().find(|(_, p)| {

            let support_graphics = p.queue_flags.contains(vk::QueueFlags::GRAPHICS) ;
            let support_presentation = unsafe { surface_loader.get_physical_device_surface_support(physical_device, 0, surface) }.unwrap();

            support_graphics && support_presentation
        }).map(|(i, _)| i as u32).unwrap_or_else(|| {
            panic!("No avaliable queue family found");
        });

        let device_extensions = vec![vk::KhrSwapchainFn::name().as_ptr()];

        let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build()];
        let mut device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions);

        let device = caps_checker.create_device(&instance, physical_device, &mut device_create_info)?;

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        let command_pool = unsafe { device.create_command_pool(&vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .build(), None) }.context("Command pool creation")?;

        Ok(VulkanBackend {
            entry,
            instance, 

            surface_loader,
            surface,
            surface_resolution,
            debug_utils,
            capabilities_checker: caps_checker,
            physical_device,

            device,
            queue,
            command_pool,

            swapchain_wrapper: None,
        })
    }

    pub fn init_swapchain(&mut self) -> anyhow::Result<()> {

        self.swapchain_wrapper = Some(SwapchainWrapper::new(self)?);

        Ok(())
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        info!("render");


        Ok(())
    }
}

impl Drop for VulkanBackend {
    fn drop(&mut self) {
        info!("drop");
        if let Some(mut swapchain) = self.swapchain_wrapper.take() {
            unsafe { swapchain.destroy() };
        }

        unsafe { self.device.device_wait_idle().unwrap() };
        unsafe { self.device.destroy_command_pool(self.command_pool, None) };
        unsafe { self.device.destroy_device(None) };
        unsafe { self.surface_loader.destroy_surface(self.surface, None)};
        unsafe { self.debug_utils.destroy() };
        unsafe { self.instance.destroy_instance(None) };

    }
}

#[derive(Debug, Default)]
pub struct AppData {

}
