use std::ops::Deref;
use ash::Instance;

/// RAII vulakn instance
pub struct VkInstance {
    instance: Instance
}

impl Deref for VkInstance {
    type Target = Instance;
    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl VkInstance {
    pub fn new(instance: Instance) -> VkInstance {
        VkInstance {
            instance,
        }
    }
}

impl Drop for VkInstance {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None); }
    }
}