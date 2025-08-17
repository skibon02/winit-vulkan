use ash::{Entry, Instance};
use ash::vk::{PhysicalDevice, TimeDomainEXT};
use log::info;

pub struct CalibratedTimestamps {
    instance: ash::ext::calibrated_timestamps::Instance,

    time_domains: Vec<TimeDomainEXT>,
}

impl CalibratedTimestamps {
    pub fn new(instance: &Instance, physical_device: PhysicalDevice) -> Self {
        let entry = Entry::linked();
        let instance = ash::ext::calibrated_timestamps::Instance::new(&entry, instance);
        unsafe {
            let time_domains = instance.get_physical_device_calibrateable_time_domains(physical_device).unwrap();
            info!("Calibrated timestamps time domains: {:?}", time_domains);
            Self {
                instance,
                time_domains
            }
        }
    }
}
