use std::ops::Deref;
use std::sync::Arc;
use ash::Device;

pub type VkDeviceRef = Arc<VkDevice>;

/// Reference to the vulkan Device.
/// When last reference is destroyed, device is destroyed as well
#[derive(Clone)]
pub struct VkDevice {
    device: Device,
}
impl VkDevice {
    pub fn new(device: Device) -> VkDevice {
        VkDevice {
            device,
        }
    }
}

impl Deref for VkDevice {
    type Target = Device;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Drop for VkDevice {
    fn drop(&mut self) {
        // Safety: We use raii and ensure that everyon who use device
        unsafe { self.device.destroy_device(None); }
    }
}