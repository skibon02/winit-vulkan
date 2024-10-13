use std::collections::BTreeSet;
use std::ffi::CStr;
use std::slice;
use std::sync::Arc;
use ash::{vk, Entry};
use log::{info, warn};
use sparkles_macro::range_event_start;
use crate::vulkan_backend::wrappers::device::{VkDevice, VkDeviceRef};
use crate::vulkan_backend::wrappers::instance::VkInstance;

/// Helper for creating Instance and Device
pub struct CapabilitiesChecker {
    activated_layers: BTreeSet<String>,
    activated_instance_extensions: BTreeSet<String>,
    activated_device_extensions: BTreeSet<String>
}

impl CapabilitiesChecker {
    pub fn new() -> CapabilitiesChecker {
        CapabilitiesChecker{
            activated_layers: BTreeSet::new(),
            activated_instance_extensions: BTreeSet::new(),
            activated_device_extensions: BTreeSet::new()
        }
    }

    pub fn create_instance(&mut self, create_info: &mut vk::InstanceCreateInfo) -> anyhow::Result<VkInstance> {
        let g = range_event_start!("[VulkanHelpers] Create instance");
        let requested_layers = unsafe {slice::from_raw_parts(create_info.pp_enabled_layer_names, create_info.enabled_layer_count as usize)};
        let requested_layers: Vec<_> = requested_layers.iter()
            .map(|layer| unsafe { CStr::from_ptr(*layer) })
            .collect();

        let entry = Entry::linked();
        let supported_layers = unsafe { entry.enumerate_instance_layer_properties() }?;

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

        let supported_extensions = unsafe { entry.enumerate_instance_extension_properties(None) }?;

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
            warn!("Instance extension {name} is not supported!");
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
            info!("Activated instance extension: {}", e);
        }

        Ok(VkInstance::new(instance))
    }

    pub fn create_device(&mut self, instance: &ash::Instance, physical_device: vk::PhysicalDevice, create_info: &mut vk::DeviceCreateInfo) -> anyhow::Result<VkDeviceRef> {
        let g = range_event_start!("[VulkanHelpers] Create device");
        let requested_extensions = unsafe {slice::from_raw_parts(create_info.pp_enabled_extension_names, create_info.enabled_extension_count as usize)};
        let requested_extensions: Vec<_> = requested_extensions.iter()
            .map(|ext| unsafe { CStr::from_ptr(*ext) })
            .collect();

        let supported_extensions = unsafe { instance.enumerate_device_extension_properties(physical_device)? };

        let filtered_extensions: Vec<_> = requested_extensions.iter().filter(|e| {
            let name: &str = e.to_str().unwrap();
            let supported = supported_extensions.iter().find(|supported_extension| {
                let supported_e_name_bytes = supported_extension.extension_name;
                let supported_e_name = unsafe { CStr::from_ptr(supported_e_name_bytes.as_ptr()) }.to_str().unwrap();
                supported_e_name == name
            });

            if supported.is_some() {
                self.activated_device_extensions.insert(name.to_owned());
                return true;
            }
            warn!("Device extension {name} is not supported!");
            false
        }).map(|layer| layer.as_ptr()).collect();

        create_info.enabled_extension_count = filtered_extensions.len() as u32;
        create_info.pp_enabled_extension_names = filtered_extensions.as_ptr();

        let device = unsafe {instance.create_device(physical_device, create_info, None)?};

        for e in self.activated_device_extensions.iter() {
            info!("Activated device extension: {}", e);
        }

        Ok(VkDevice::new(device).into())
    }
}

impl Default for CapabilitiesChecker {
    fn default() -> Self {
        Self::new()
    }
}