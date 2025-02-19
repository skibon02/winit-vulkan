use ash::vk;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Copy, Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum InFlightFrames {
    One = 1,
    Two
}


pub struct VulkanRenderConfig {
    pub msaa_samples: Option<u32>,
    pub in_flight_frames: InFlightFrames,
}

impl VulkanRenderConfig {
    pub fn get_msaa_samples(&self) -> Option<vk::SampleCountFlags> {
        self.msaa_samples.map(|msaa_samples|
            match msaa_samples {
                1 => vk::SampleCountFlags::TYPE_1,
                2 => vk::SampleCountFlags::TYPE_2,
                4 => vk::SampleCountFlags::TYPE_4,
                8 => vk::SampleCountFlags::TYPE_8,
                16 => vk::SampleCountFlags::TYPE_16,
                32 => vk::SampleCountFlags::TYPE_32,
                64 => vk::SampleCountFlags::TYPE_64,
                _ => vk::SampleCountFlags::TYPE_1,
            }
        )
    }
}