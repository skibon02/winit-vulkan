pub mod descriptor_sets;
pub mod pipeline;
pub mod render_pass;
pub mod resource_manager;
pub mod swapchain_wrapper;
pub mod wrappers;
pub mod config;
pub(super) mod object_resource_pool;

use swapchain_wrapper::SwapchainWrapper;

use log::{debug, error, info, warn};

use ash::vk::{
    self, make_api_version, ApplicationInfo, BufferUsageFlags, CommandBuffer,
    CommandBufferBeginInfo, DeviceSize, Extent2D, FenceCreateFlags, PhysicalDevice,
    PipelineBindPoint, PrimitiveTopology, Queue, RenderPassBeginInfo, Semaphore,
};

use crate::use_shader;
use crate::vulkan_backend::descriptor_sets::ObjectDescriptorSet;
use crate::vulkan_backend::pipeline::{PipelineDesc, VertexInputDesc, VulkanPipeline};
use crate::vulkan_backend::render_pass::RenderPassResources;
use crate::vulkan_backend::resource_manager::{BufferResource, ResourceManager};
use crate::vulkan_backend::wrappers::capabilities_checker::CapabilitiesChecker;
use crate::vulkan_backend::wrappers::command_pool::VkCommandPool;
use crate::vulkan_backend::wrappers::debug_utils::VkDebugUtils;
use crate::vulkan_backend::wrappers::device::VkDeviceRef;
use crate::vulkan_backend::wrappers::surface::{VkSurface, VkSurfaceRef};
use render_pass::RenderPassWrapper;
use sparkles_macro::{instant_event, range_event_start};
use std::array::from_fn;
use std::ffi::{c_char, CString};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::state::{DrawStateCollect};
use crate::vulkan_backend::config::VulkanRenderConfig;
use crate::vulkan_backend::object_resource_pool::ObjectResourcePool;

pub struct VulkanBackend {
    config: VulkanRenderConfig,

    debug_utils: VkDebugUtils,
    surface: VkSurfaceRef,
    physical_device: PhysicalDevice,
    device: VkDeviceRef,
    queue: Queue,
    command_pool: VkCommandPool,

    resource_manager: ResourceManager,

    command_buffers: [CommandBuffer; 1],
    image_available_semaphores: [Semaphore; 1],
    render_finished_semaphores: [Semaphore; 1],
    fences: [vk::Fence; 1],
    cur_command_buffer: usize,
    command_buffer_last_index: [Option<usize>; 1],

    swapchain_wrapper: SwapchainWrapper,

    object_resource_pool: ObjectResourcePool,

    // stuff for actual rendering
    render_pass: RenderPassWrapper,
    render_pass_resources: RenderPassResources,
}

impl VulkanBackend {
    /// Initialize vulkan resources and use window to create surface
    ///
    /// Must be called from main thread!
    pub fn new_for_window(window_handle: RawWindowHandle, display_handle: RawDisplayHandle, window_size: (u32, u32), config: VulkanRenderConfig) -> anyhow::Result<Self> {
        let g = range_event_start!("[Vulkan] INIT");
        info!(
            "Vulkan init started! Initializing for size: {:?}",
            window_size
        );

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
        if cfg!(feature = "validation_layers") {
            instance_layers.push(CString::new("VK_LAYER_KHRONOS_validation")?);
        }
        let instance_layers_refs: Vec<*const c_char> =
            instance_layers.iter().map(|l| l.as_ptr()).collect();

        //define desired extensions
        // 1 Debug utils
        // 2,3 Required extensions for surface support (platform_specific surface + general surface)
        let surface_required_extensions =
            ash_window::enumerate_required_extensions(display_handle)?;
        let mut instance_extensions: Vec<*const c_char> = surface_required_extensions.to_vec();
        instance_extensions.push(ash::ext::debug_utils::NAME.as_ptr());

        let mut debug_utils_messenger_info = VkDebugUtils::get_messenger_create_info();
        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&instance_layers_refs)
            .enabled_extension_names(&instance_extensions)
            .push_next(&mut debug_utils_messenger_info);

        let mut caps_checker = CapabilitiesChecker::new();

        // caps_checker will check requested layers and extensions and enable only the
        // supported ones, which can be requested later
        let instance = caps_checker.create_instance(&mut create_info)?;

        let surface = VkSurface::new(instance.clone(), display_handle, window_handle)?;

        let debug_utils = VkDebugUtils::new(instance.clone())?;
        // instance is created. debug utils ready

        let physical_devices = unsafe { instance.enumerate_physical_devices()? };

        let physical_device = *physical_devices
            .iter()
            .find(|&d| {
                let properties = unsafe { instance.get_physical_device_properties(*d) };
                properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
            })
            .or_else(|| {
                warn!("Discrete GPU was not found!");
                physical_devices.iter().find(|&d| {
                    let properties = unsafe { instance.get_physical_device_properties(*d) };
                    properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
                })
            })
            .or_else(|| {
                warn!("Integrated GPU was not found!");
                physical_devices.iter().find(|&d| {
                    let properties = unsafe { instance.get_physical_device_properties(*d) };
                    properties.device_type == vk::PhysicalDeviceType::CPU
                })
            })
            .unwrap_or_else(|| {
                panic!("No avaliable physical device found");
            });

        //select chosen physical device
        let dev_name_array = unsafe {
            instance
                .get_physical_device_properties(physical_device)
                .device_name
        };
        let dev_name = unsafe { std::ffi::CStr::from_ptr(dev_name_array.as_ptr()) };
        info!("Chosen device: {}", dev_name.to_str().unwrap());

        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let queue_family_index = queue_family_properties
            .iter()
            .enumerate()
            .find(|(_, p)| {
                let support_graphics = p.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                let support_presentation = surface.query_presentation_support(physical_device);

                support_graphics && support_presentation
            })
            .map(|(i, _)| i as u32)
            .unwrap_or_else(|| {
                panic!("No available queue family found");
            });

        let device_extensions = vec![ash::khr::swapchain::NAME.as_ptr()];

        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])];
        let mut device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions);

        let device = caps_checker.create_device(
            instance.clone(),
            physical_device,
            &mut device_create_info,
        )?;

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        let command_pool = VkCommandPool::new(device.clone(), queue_family_index);
        let command_buffers = command_pool.alloc_command_buffers(1);

        let image_available_semaphores = from_fn(|_| unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        });
        let render_finished_semaphores = from_fn(|_| unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        });

        let fences = from_fn(|_| unsafe {
            device
                .create_fence(
                    &vk::FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED),
                    None,
                )
                .unwrap()
        });

        let mut resource_manager =
            ResourceManager::new(physical_device, device.clone(), queue, &command_pool);

        let extent = Extent2D {
            width: window_size.0,
            height: window_size.1,
        };
        let swapchain_wrapper = SwapchainWrapper::new(
            device.clone(),
            physical_device,
            extent,
            surface.clone(),
            None,
        )?;

        let msaa_samples = config.get_msaa_samples();

        let render_pass = RenderPassWrapper::new(
            device.clone(),
            swapchain_wrapper.get_surface_format(),
            msaa_samples,
        );
        let render_pass_resources = render_pass.create_render_pass_resources(
            swapchain_wrapper.get_image_views(),
            swapchain_wrapper.get_extent(),
            &mut resource_manager,
        );

        let object_resource_pool = ObjectResourcePool::new(device.clone());


        Ok(VulkanBackend {
            config,

            surface,
            debug_utils,

            physical_device,
            device,
            queue,
            command_pool,

            resource_manager,

            swapchain_wrapper,
            command_buffers: command_buffers.try_into().unwrap(),
            image_available_semaphores,
            render_finished_semaphores,
            fences,
            cur_command_buffer: 0,
            command_buffer_last_index: [None; 1],

            object_resource_pool,

            render_pass,
            render_pass_resources,
        })
    }

    pub fn recreate_resize(&mut self, new_extent: (u32, u32)) {
        let g = range_event_start!("[Vulkan] Recreate swapchain");
        let new_extent = Extent2D {
            width: new_extent.0,
            height: new_extent.1,
        };
        self.wait_idle();

        //clear states
        self.command_buffer_last_index = [None; 1];

        // 1. Destroy swapchain dependent resources
        unsafe {
            self.render_pass_resources
                .destroy(&mut self.resource_manager);
        }

        // 2. Recreate swapchain
        let old_format = self.swapchain_wrapper.get_surface_format();
        unsafe {
            self.swapchain_wrapper
                .recreate(self.physical_device, new_extent, self.surface.clone())
                .unwrap()
        };
        let new_format = self.swapchain_wrapper.get_surface_format();
        if new_format != old_format {
            unimplemented!("Swapchain returned the wrong format");
        }

        // 3. Recreate swapchain_dependent resources
        self.render_pass_resources = self.render_pass.create_render_pass_resources(
            self.swapchain_wrapper.get_image_views(),
            self.swapchain_wrapper.get_extent(),
            &mut self.resource_manager,
        );
    }

    pub fn render(&mut self, draw_state_diff: &mut impl DrawStateCollect) -> anyhow::Result<()> {
        let g = range_event_start!("[Vulkan] render");
        let frame_index = self.cur_command_buffer;
        self.cur_command_buffer = (frame_index + 1) % self.command_buffers.len();
        let cur_fence = self.fences[frame_index];
        let cur_command_buffer = self.command_buffers[frame_index];

        // 1) Acquire next image
        let (image_index, is_suboptimal) = unsafe {
            let g = range_event_start!("[Vulkan] Wait for fences...");
            self.device
                .wait_for_fences(&[cur_fence], true, u64::MAX)
                .unwrap();
            drop(g);
            self.device.reset_fences(&[cur_fence]).unwrap();


            let g = range_event_start!("[Vulkan] Acquire next image...");
            let res = self
                .swapchain_wrapper
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain_wrapper.get_swapchain(),
                    u64::MAX,
                    self.image_available_semaphores[frame_index],
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next image.");

            instant_event!("[Vulkan] New frame!");
            res
        };
        if is_suboptimal {
            warn!("Swapchain is suboptimal!");
        }

        // 2) Update
        let g = range_event_start!("[Vulkan] Update draw state");

        // let uniform_state = draw_state_diff.collect_uniform_states();
        let objects_state = draw_state_diff.collect_object_states();
        self.object_resource_pool.update_objects(&mut self.resource_manager, objects_state, &self.render_pass);
        draw_state_diff.clear_state();
        drop(g);

        // 3) record command buffer (if index was changed)
        let image_index = image_index as usize;
        if self.command_buffer_last_index[frame_index] != Some(image_index) {
            self.record_draw(cur_command_buffer, image_index);
            self.command_buffer_last_index[frame_index] = Some(image_index);
        };

        let g = range_event_start!("[Vulkan] Submit command buffer");
        // 3.1) submit command buffer
        let wait_semaphores = [self.image_available_semaphores[frame_index]];
        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [cur_command_buffer];
        let signal_semaphores = [self.render_finished_semaphores[frame_index]];
        let submit_infos = [vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)];
        unsafe {
            self.device
                .queue_submit(self.queue, &submit_infos, self.fences[frame_index])
                .unwrap();
        }
        drop(g);

        // 4) present
        let g = range_event_start!("[Vulkan] Queue present");
        let swapchains = [self.swapchain_wrapper.get_swapchain()];
        let semaphores = [self.render_finished_semaphores[frame_index]];
        let image_indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .image_indices(&image_indices)
            .wait_semaphores(&semaphores);

        unsafe {
            match self
                .swapchain_wrapper
                .swapchain_loader
                .queue_present(self.queue, &present_info)
            {
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
        Ok(())
    }

    fn record_draw(&mut self, command_buffer: CommandBuffer, image_index: usize) {
        let device = &self.device;
        let framebuffer = self.render_pass_resources.framebuffers[image_index];
        let extent = self.swapchain_wrapper.get_extent();

        let g = range_event_start!("[Vulkan] Command buffer recording");
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(*self.render_pass.get_render_pass())
            .framebuffer(framebuffer)
            .render_area(extent.into())
            .clear_values(&[
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.8, 0.4, 0.7, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.8, 0.4, 0.7, 1.0],
                    },
                },
            ]);

        let viewport = vk::Viewport::default()
            .width(extent.width as f32)
            .height(extent.height as f32);
        let scissors = extent.into();
        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .unwrap();
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            //bind dynamic states
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissors]);

            // draw object states
            self.object_resource_pool.record_draw_commands(command_buffer);

            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer).unwrap();
        }
    }

    fn wait_idle(&self) {
        let start = std::time::Instant::now();
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
        let end = std::time::Instant::now();
        debug!("Waited for idle for {:?}", end - start);
    }
}

impl Drop for VulkanBackend {
    fn drop(&mut self) {
        info!("vulkan: drop");
        self.wait_idle();
        unsafe {
            self.render_pass_resources
                .destroy(&mut self.resource_manager);
        }

        for semaphore in self.image_available_semaphores {
            unsafe {
                self.device.destroy_semaphore(semaphore, None);
            }
        }
        for semaphore in self.render_finished_semaphores {
            unsafe {
                self.device.destroy_semaphore(semaphore, None);
            }
        }
        for fence in self.fences {
            unsafe {
                self.device.destroy_fence(fence, None);
            }
        }
    }
}
