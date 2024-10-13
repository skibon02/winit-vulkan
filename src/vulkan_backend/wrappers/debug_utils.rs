use ash::{vk, Entry, Instance};
use ash::vk::{DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCreateInfoEXT};
use log::{debug, error, info, warn};

pub struct VkDebugUtils {
    debug_utils_h: ash::ext::debug_utils::Instance,
    debug_utils_messenger_h: vk::DebugUtilsMessengerEXT
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    message_type: DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { &*p_callback_data };
    let msg = unsafe { std::ffi::CStr::from_ptr(callback_data.p_message) };
    match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!("{:?}: {}", message_type, msg.to_str().unwrap());
        },
        _ => {}
    }
    vk::FALSE
}

impl VkDebugUtils {
    /// Can be used AFTER instance is created
    pub fn new(instance: &Instance) -> anyhow::Result<VkDebugUtils> {
        let entry = Entry::linked();

        let debug_utils_h = ash::ext::debug_utils::Instance::new(&entry, instance);

        let debug_utils_messenger_h = unsafe {
            debug_utils_h.create_debug_utils_messenger(
                &Self::get_messenger_create_info(), None) }?;


        Ok(VkDebugUtils {
            debug_utils_messenger_h,
            debug_utils_h
        })
    }

    /// Can be used during instance creation
    pub fn get_messenger_create_info() -> DebugUtilsMessengerCreateInfoEXT<'static> {
        let debug_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(DebugUtilsMessageSeverityFlagsEXT::INFO | DebugUtilsMessageSeverityFlagsEXT::WARNING | DebugUtilsMessageSeverityFlagsEXT::ERROR)
            .message_type(DebugUtilsMessageTypeFlagsEXT::GENERAL | DebugUtilsMessageTypeFlagsEXT::VALIDATION | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
            .pfn_user_callback(Some(vulkan_debug_callback));
        debug_messenger_create_info
    }
}

impl Drop for VkDebugUtils {
    fn drop(&mut self) {
        unsafe { self.debug_utils_h.destroy_debug_utils_messenger(self.debug_utils_messenger_h, None) };
    }
}
