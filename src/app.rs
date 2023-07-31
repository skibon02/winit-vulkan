pub mod pipeline{
    use ash::vk::{self, PipelineLayout};
    use vk::Pipeline;


    pub struct TrianglePipeline {
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
    }
}


use crate::helpers::{self, DebugUtilsHelper, CapabilitiesChecker};

use anyhow::Context;
use ash::extensions::khr::Surface;
use ash_window::create_surface;
use log::{info, debug};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

use ash::{Entry, Instance};
use ash::vk::{self, make_api_version, ApplicationInfo, SurfaceKHR};

use std::ffi::CString;

pub struct App{
    entry: Entry,
    instance: Instance,
    surface_loader: Surface,

    surface: SurfaceKHR,
    debug_utils: helpers::DebugUtilsHelper,

    capabilities_checker: helpers::CapabilitiesChecker
}

impl App {
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
        let display_handle = window.raw_display_handle();
        let window_handle = window.raw_window_handle();

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

        let debug_utils = helpers::DebugUtilsHelper::new(&entry, &instance)?;
        // instance is created. debug utils ready

        Ok(App {
            entry,
            instance, 

            surface_loader,
            surface,
            debug_utils,
            capabilities_checker: caps_checker
            
        })
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        info!("render");


        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        info!("drop");

        unsafe {self.surface_loader.destroy_surface(self.surface, None)};
        unsafe { self.debug_utils.destroy() };
        unsafe { self.instance.destroy_instance(None) };
    }
}

#[derive(Debug, Default)]
pub struct AppData {

}
