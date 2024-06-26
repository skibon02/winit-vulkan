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
use log::{error, info};
use winit::dpi::{PhysicalSize};
use winit::window::Window;

use ash::{Entry, Instance, Device};
use ash::vk::{self, make_api_version, ApplicationInfo, SurfaceKHR, Queue, Semaphore};

use std::ffi::{c_char, CString};
use std::marker::PhantomData;
use std::ptr;
use ash_window::create_surface;
use winit::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};


pub struct VulkanBackend {
    entry: Entry,
    instance: Instance,
    surface_loader: ash::khr::surface::Instance,

    surface: SurfaceKHR,
    surface_resolution: PhysicalSize<u32>,
    debug_utils: DebugUtilsHelper,

    capabilities_checker: CapabilitiesChecker,
    physical_device: vk::PhysicalDevice,

    device: Device,
    queue: Queue,
    command_pool: vk::CommandPool,

    swapchain_wrapper: Option<SwapchainWrapper>,

    window: Window,

    semaphores: [Semaphore; 2],
    cur_frame: u32
}

impl VulkanBackend {
    // Initialize vulkan resources and use window to create surface
    pub fn new(window: Window) -> anyhow::Result<Self> {
        let window_handle = window.raw_window_handle().unwrap();
        let display_handle = window.raw_display_handle().unwrap();
        let window_size = window.inner_size();

        let entry = Entry::linked();

        let app_name = CString::new("Hello Triangle")?;

        let app_info = ApplicationInfo::default()
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
        let instance_layers_refs: Vec<*const c_char> = instance_layers.iter().map(|l| l.as_ptr())
            .collect();

        //define desired extensions

        let surface_required_extensions = ash_window::enumerate_required_extensions(display_handle)?;
        let mut instance_extensions: Vec<*const c_char> =
            surface_required_extensions.to_vec();
        instance_extensions.push(ash::ext::debug_utils::NAME.as_ptr());


        let mut debug_utils_messanger_info = DebugUtilsHelper::get_messenger_create_info();
        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&instance_layers_refs)
            .enabled_extension_names(&instance_extensions)
            .push_next(&mut debug_utils_messanger_info);

        let mut caps_checker = CapabilitiesChecker::new();

        // caps_checker will check requested layers and extensions for support and enable only the
        // supported ones, so we can request them later
        let instance = caps_checker.create_instance(&entry, &mut create_info)?;

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = unsafe { create_surface(&entry, &instance, display_handle, window_handle, None).context("Surface creation")? };
        let surface_resolution = window_size;

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

        let device_extensions = vec![ash::khr::swapchain::NAME.as_ptr()];

        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])];
        let mut device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions);

        let device = caps_checker.create_device(&instance, physical_device, &mut device_create_info)?;

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        let command_pool = unsafe { device.create_command_pool(&vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER), None)
        }.context("Command pool creation")?;

        let semaphores = [
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() },
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() }
        ];

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
            window,

            semaphores,
            cur_frame: 0
        })
    }

    pub fn init_swapchain(&mut self) -> anyhow::Result<()> {
        self.swapchain_wrapper = Some(SwapchainWrapper::new(self)?);

        Ok(())
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        info!("render");

        let frame_index = (self.cur_frame % 2) as usize;
        self.cur_frame = (frame_index as u32 + 1) % 2;

        let swapchain_wrapper = self.swapchain_wrapper.as_mut().unwrap();

        let (image_index, is_suboptimal) = unsafe { swapchain_wrapper.swapchain_loader
            .acquire_next_image(
                swapchain_wrapper.swapchain,
                std::u64::MAX,
                self.semaphores[frame_index],
                vk::Fence::null(),
            ).expect("Failed to acquire next image.") };

        let swapchains = [swapchain_wrapper.swapchain];
        let semaphores = [self.semaphores[frame_index]];
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: ptr::null_mut(),
            _marker: PhantomData
        };

        unsafe {
            match swapchain_wrapper.swapchain_loader.queue_present(self.queue, &present_info) {
                Ok(r) => {
                    info!("Draw success!");
                }
                Err(e) => {
                    error!("queue_present: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn request_redraw(&mut self) {
        self.window.request_redraw();
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
