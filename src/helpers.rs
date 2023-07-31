
use core::slice;
use std::{collections::BTreeSet, ffi::{CString, c_char, CStr}, ops::Deref};

use ash::{vk::{self, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCreateInfoEXT, LayerProperties}, Instance, Entry};
use log::{error, info, warn, debug};


pub struct DebugUtilsHelper {
    debug_utils_h: ash::extensions::ext::DebugUtils,
    debug_utils_messenger_h: vk::DebugUtilsMessengerEXT
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
    ) -> vk::Bool32 {
    let callback_data = unsafe { &*p_callback_data };
    let msg = unsafe { std::ffi::CStr::from_ptr(callback_data.p_message) };
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        _ => {}
    }
    vk::FALSE
}

impl DebugUtilsHelper {
    pub fn new(entry: &Entry, instance: &Instance) -> anyhow::Result<DebugUtilsHelper> {

        let debug_utils_h = ash::extensions::ext::DebugUtils::new(entry, instance);

        let debug_utils_messenger_h = unsafe { 
            debug_utils_h.create_debug_utils_messenger(
                &Self::get_messenger_create_info(), None) }?;


        Ok(DebugUtilsHelper {
            debug_utils_messenger_h,
            debug_utils_h
        })
    }

    pub fn get_messenger_create_info() -> DebugUtilsMessengerCreateInfoEXT {
        let debug_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(DebugUtilsMessageSeverityFlagsEXT::INFO | DebugUtilsMessageSeverityFlagsEXT::WARNING | DebugUtilsMessageSeverityFlagsEXT::ERROR)
            .message_type(DebugUtilsMessageTypeFlagsEXT::GENERAL | DebugUtilsMessageTypeFlagsEXT::VALIDATION | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
            .pfn_user_callback(Some(vulkan_debug_callback))
            .build();
        debug_messenger_create_info
    }

    // Safety: valid only in drop! (No usage after this call)
    pub unsafe fn destroy(&mut self) {
        unsafe { self.debug_utils_h.destroy_debug_utils_messenger(self.debug_utils_messenger_h, None) };

    }
}


pub struct CapabilitiesChecker {
    activated_layers: BTreeSet<String>,
    activated_instance_extensions: BTreeSet<String>
}

impl CapabilitiesChecker {
    pub fn new() -> CapabilitiesChecker {
        CapabilitiesChecker{
            activated_layers: BTreeSet::new(),
            activated_instance_extensions: BTreeSet::new(),
        }
    }

    pub fn create_instance(&mut self, entry: &ash::Entry, create_info: &mut vk::InstanceCreateInfo) -> anyhow::Result<ash::Instance> {

        let requested_layers = unsafe {slice::from_raw_parts(create_info.pp_enabled_layer_names, create_info.enabled_layer_count as usize)};
        let requested_layers: Vec<_> = requested_layers.iter()
            .map(|layer| unsafe { CStr::from_ptr(*layer) })
            .collect();

        let supported_layers = entry.enumerate_instance_layer_properties()?;

        let filtered_layers: Vec<_> = requested_layers.iter().filter(|l| {
            let name: &str = l.to_str().unwrap();
            let supported = supported_layers.iter().find(|supported_layer| {
                let supported_l_name_bytes = supported_layer.layer_name;
                let supported_l_name = unsafe { CStr::from_ptr(supported_l_name_bytes.as_ptr()) }.to_str().unwrap();
                supported_l_name == name
            });

            if supported.is_some() {
                self.activated_layers.insert(name.to_owned());
                return true;
            }
            warn!("Layer {name} is not supported!");
            false
        }).map(|layer| layer.as_ptr())
        .collect();


        let requested_extensions = unsafe {slice::from_raw_parts(create_info.pp_enabled_extension_names, create_info.enabled_extension_count as usize)};
        let requested_extensions: Vec<_> = requested_extensions.iter()
            .map(|ext| unsafe { CStr::from_ptr(*ext) })
            .collect();

        let supported_extensions = entry.enumerate_instance_extension_properties(None)?;

        let filtered_extensions: Vec<_> = requested_extensions.iter().filter(|e| {
            let name: &str = e.to_str().unwrap();
            let supported = supported_extensions.iter().find(|supported_extension| {
                let supported_e_name_bytes = supported_extension.extension_name;
                let supported_e_name = unsafe { CStr::from_ptr(supported_e_name_bytes.as_ptr()) }.to_str().unwrap();
                supported_e_name == name
            });

            if supported.is_some() {
                self.activated_instance_extensions.insert(name.to_owned());
                return true;
            }
            warn!("Extension {name} is not supported!");
            false
        }).map(|layer| layer.as_ptr()).collect();

        create_info.enabled_layer_count = filtered_layers.len() as u32;
        create_info.pp_enabled_layer_names = filtered_layers.as_ptr();

        create_info.enabled_extension_count = filtered_extensions.len() as u32;
        create_info.pp_enabled_extension_names = filtered_extensions.as_ptr();

        let instance = unsafe {entry.create_instance(create_info, None)}?;

        for l in self.activated_layers.iter() {
            info!("Activated layer: {}", l);
        }
        for e in self.activated_instance_extensions.iter() {
            info!("Activated extension: {}", e);
        }

        Ok(instance)
    }
}
