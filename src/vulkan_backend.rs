pub mod pipeline {
    use ash::vk::{self, PipelineLayout};
    use vk::Pipeline;


    pub struct TrianglePipeline {
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
    }
}

pub mod swapchain_wrapper;
pub mod helpers;
pub mod resource_manager;

use crate::vulkan_backend::swapchain_wrapper::SwapchainWrapper;

use anyhow::Context;
use log::{error, info, warn};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use ash::{Device, Entry, Instance};
use ash::vk::{self, make_api_version, ApplicationInfo, CommandBuffer, CommandBufferBeginInfo, FenceCreateFlags, Queue, RenderPassBeginInfo, Semaphore, SurfaceKHR};

use std::ffi::{c_char, CString};
use ash_window::create_surface;
use sparkles_macro::{instant_event, range_event_start};
use winit::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::vulkan_backend::helpers::{CapabilitiesChecker, DebugUtilsHelper};

pub struct VulkanBackend {
    entry: Entry,
    instance: Instance,

    debug_utils: DebugUtilsHelper,

    surface_loader: ash::khr::surface::Instance,
    surface: SurfaceKHR,
    surface_resolution: PhysicalSize<u32>,

    capabilities_checker: CapabilitiesChecker,
    physical_device: vk::PhysicalDevice,

    device: Device,
    queue: Queue,
    command_pool: vk::CommandPool,

    swapchain_wrapper: Option<SwapchainWrapper>,

    /// 2 semaphores because at most 2 images can be acquired at the same time for the rendering operation
    command_buffers: Vec<CommandBuffer>,
    image_available_semaphores: [Semaphore; 2],
    render_finished_semaphores: [Semaphore; 2],
    fences: [vk::Fence; 2],

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

        let mut res = VulkanBackend {
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

            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            fences,

            cur_frame: 0
        };

        res.init_swapchain().unwrap();

        Ok(res)
    }

    fn init_swapchain(&mut self) -> anyhow::Result<()> {
        self.swapchain_wrapper = Some(SwapchainWrapper::new(self)?);

        Ok(())
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let g = range_event_start!("[Vulkan] render");
        let frame_index = self.cur_frame;
        self.cur_frame = (frame_index + 1) % 2;

        let swapchain_wrapper = self.swapchain_wrapper.as_mut().unwrap();

        // 1) Acquire next image
        let (image_index, is_suboptimal) = unsafe {
            let g = range_event_start!("[Vulkan] Wait for fences...");
            self.device.wait_for_fences(&[self.fences[frame_index]], true, u64::MAX).unwrap();
            drop(g);
            self.device.reset_fences(&[self.fences[frame_index]]).unwrap();
            let g = range_event_start!("[Vulkan] Acquire next image...");
            let res = swapchain_wrapper.swapchain_loader.acquire_next_image(
                swapchain_wrapper.swapchain,
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

        let g = range_event_start!("[Vulkan] Command buffer recording");
        // 2) record command buffer
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(swapchain_wrapper.render_pass)
            .framebuffer(swapchain_wrapper.framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_wrapper.swapchain_extent,
            })
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.8, 0.4, 0.7, 1.0],
                },
            }]);

        unsafe {
            self.device.begin_command_buffer(self.command_buffers[frame_index], &command_buffer_begin_info).unwrap();
            self.device.cmd_begin_render_pass(self.command_buffers[frame_index], &render_pass_begin_info, vk::SubpassContents::INLINE);
            self.device.cmd_end_render_pass(self.command_buffers[frame_index]);
            self.device.end_command_buffer(self.command_buffers[frame_index]).unwrap();
        }
        drop(g);

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
        let swapchains = [swapchain_wrapper.swapchain];
        let semaphores = [self.render_finished_semaphores[frame_index]];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .image_indices(&image_indices)
            .wait_semaphores(&semaphores);

        unsafe {
            match swapchain_wrapper.swapchain_loader.queue_present(self.queue, &present_info) {
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
        if let Some(mut swapchain) = self.swapchain_wrapper.take() {
            unsafe { swapchain.destroy() };
        }

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
