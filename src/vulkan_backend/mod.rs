

pub mod swapchain_wrapper;
pub mod helpers;
pub mod resource_manager;
pub mod pipeline;
pub mod render_pass;

use swapchain_wrapper::SwapchainWrapper;

use anyhow::Context;
use log::{error, info, warn};
use winit::window::Window;

use ash::{Device, Entry, Instance};
use ash::vk::{self, make_api_version, ApplicationInfo, CommandBuffer, CommandBufferBeginInfo, Extent2D, FenceCreateFlags, Queue, RenderPassBeginInfo, Semaphore, SurfaceKHR};

use std::ffi::{c_char, CString};
use ash_window::create_surface;
use sparkles_macro::{instant_event, range_event_start};
use winit::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use helpers::{CapabilitiesChecker, DebugUtilsHelper};
use pipeline::TrianglePipeline;
use render_pass::RenderPassWrapper;

pub struct VulkanBackend {
    instance: Instance,

    debug_utils: DebugUtilsHelper,

    surface_loader: ash::khr::surface::Instance,
    surface: SurfaceKHR,

    device: Device,
    queue: Queue,
    command_pool: vk::CommandPool,

    // 2 semaphores because at most 2 images can be acquired at the same time for the rendering operation
    command_buffers: Vec<CommandBuffer>,
    image_available_semaphores: [Semaphore; 2],
    render_finished_semaphores: [Semaphore; 2],
    fences: [vk::Fence; 2],

    swapchain_wrapper: SwapchainWrapper,

    // stuff for actual rendering
    render_pass: RenderPassWrapper,

    cur_frame: usize
}

impl VulkanBackend {
    /// Initialize vulkan resources and use window to create surface
    ///
    /// Must be called from main thread!
    pub fn new_for_window(window: &Window) -> anyhow::Result<Self> {
        let g = range_event_start!("[Vulkan] INIT");
        // we need window_handle to create Vulkan surface
        let window_handle = window.raw_window_handle()?;
        // we need display_handle to get required extensions
        let display_handle = window.raw_display_handle()?;
        let window_size = window.inner_size();
        info!("Vulkan init started! Got window with dimensions: {:?}", window_size);

        let entry = Entry::linked();

        let app_name = CString::new("Hello Vulkan")?;

        let app_info = ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(make_api_version(0, 1, 0, 0))
            .engine_name(&app_name)
            .engine_version(make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_0);


        //define desired layers
        // 1. Khronos validation layers (optional)
        let mut instance_layers = vec![];
        if cfg!(feature="validation_layers") {
            instance_layers.push(CString::new("VK_LAYER_KHRONOS_validation")?);
        }
        let instance_layers_refs: Vec<*const c_char> = instance_layers.iter().map(|l| l.as_ptr())
            .collect();

        //define desired extensions
        // 1 Debug utils
        // 2,3 Required extensions for surface support (platform_specific surface + general surface)
        let surface_required_extensions = ash_window::enumerate_required_extensions(display_handle)?;
        let mut instance_extensions: Vec<*const c_char> =
            surface_required_extensions.to_vec();
        instance_extensions.push(ash::ext::debug_utils::NAME.as_ptr());


        let mut debug_utils_messenger_info = DebugUtilsHelper::get_messenger_create_info();
        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&instance_layers_refs)
            .enabled_extension_names(&instance_extensions)
            .push_next(&mut debug_utils_messenger_info);

        let mut caps_checker = CapabilitiesChecker::new();

        // caps_checker will check requested layers and extensions and enable only the
        // supported ones, which can be requested later
        let instance = caps_checker.create_instance(&entry, &mut create_info)?;

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = unsafe { create_surface(&entry, &instance, display_handle, window_handle, None).context("Surface creation")? };

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
        info!("Chosen device: {}", dev_name.to_str().unwrap());


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

        let command_buffer_count = 2;
        let command_buffers = unsafe {
            device.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(command_buffer_count)).unwrap()
        };

        let image_available_semaphores = [
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() },
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() }
        ];

        let render_finished_semaphores = [
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() },
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() }
        ];

        let fences = [
            unsafe { device.create_fence(&vk::FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED), None).unwrap() },
            unsafe { device.create_fence(&vk::FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED), None).unwrap() }
        ];


        let extent = Extent2D { width: window_size.width, height: window_size.height };
        let swapchain_wrapper = SwapchainWrapper::new(&instance, &device, physical_device, extent, surface, &surface_loader)?;
        let render_pass = RenderPassWrapper::new(&device, swapchain_wrapper.get_surface_format(), swapchain_wrapper.get_resolution(), swapchain_wrapper.get_image_views());

        Ok(VulkanBackend {
            instance, 

            surface_loader,
            surface,
            debug_utils,

            device,
            queue,
            command_pool,

            swapchain_wrapper,

            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            fences,

            render_pass,

            cur_frame: 0
        })
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let g = range_event_start!("[Vulkan] render");
        let frame_index = self.cur_frame;
        self.cur_frame = (frame_index + 1) % 2;

        // 1) Acquire next image
        let (image_index, is_suboptimal) = unsafe {
            let g = range_event_start!("[Vulkan] Wait for fences...");
            self.device.wait_for_fences(&[self.fences[frame_index]], true, u64::MAX).unwrap();
            drop(g);
            self.device.reset_fences(&[self.fences[frame_index]]).unwrap();
            let g = range_event_start!("[Vulkan] Acquire next image...");
            let res = self.swapchain_wrapper.swapchain_loader.acquire_next_image(
                self.swapchain_wrapper.swapchain,
                u64::MAX,
                self.image_available_semaphores[frame_index],
                vk::Fence::null(),
            ).expect("Failed to acquire next image.");

            instant_event!("[Vulkan] New frame!");
            res
        };
        if is_suboptimal {
            warn!("Swapchain is suboptimal!");
        }

        // 2) record command buffer
        self.render_pass.record_draw(&self.device, image_index, self.command_buffers[frame_index]);

        let g = range_event_start!("[Vulkan] Submit command buffer");
        // 2.1) submit command buffer
        let wait_semaphores = [self.image_available_semaphores[frame_index]];
        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.command_buffers[frame_index]];
        let signal_semaphores = [self.render_finished_semaphores[frame_index]];
        let submit_infos = [vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)];
        unsafe {
            self.device.queue_submit(self.queue,
            &submit_infos,
            self.fences[frame_index],
            ).unwrap();
        }
        drop(g);

        //3) present
        let g = range_event_start!("[Vulkan] Queue present");
        let swapchains = [self.swapchain_wrapper.swapchain];
        let semaphores = [self.render_finished_semaphores[frame_index]];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .image_indices(&image_indices)
            .wait_semaphores(&semaphores);

        unsafe {
            match self.swapchain_wrapper.swapchain_loader.queue_present(self.queue, &present_info) {
                Ok(is_suboptimal) => {
                    if is_suboptimal {
                        warn!("swapchain suboptimal!");
                    }
                }
                Err(e) => {
                    error!("queue_present: {}", e);
                }
            }
        }
        drop(g);

        Ok(())
    }

    pub fn wait_idle(&self) {
        let start = std::time::Instant::now();
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
        let end = std::time::Instant::now();
        info!("Waited for idle for {:?}", end - start);
    }
}

impl Drop for VulkanBackend {
    fn drop(&mut self) {
        info!("vulkan: drop");
        self.wait_idle();
        unsafe { self.render_pass.destroy(&self.device); }
        unsafe { self.swapchain_wrapper.destroy(); }

        for semaphore in self.image_available_semaphores {
            unsafe { self.device.destroy_semaphore(semaphore, None); }
        }
        for semaphore in self.render_finished_semaphores {
            unsafe { self.device.destroy_semaphore(semaphore, None); }
        }
        for fence in self.fences {
            unsafe { self.device.destroy_fence(fence, None); }
        }

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
