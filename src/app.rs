use std::time::Instant;
use ash::vk::SampleCountFlags;

pub struct App {
    start: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn get_msaa_samples(&self) -> Option<SampleCountFlags> {
        Some(SampleCountFlags::TYPE_4)
    }

    pub fn get_vertex_data(&self) -> Vec<f32> {
        vec![-1.0f32, 1.0, 0.0, 1.0, 0.0, 1.0,
             0.0, -1.0, 0.0, 0.0, 1.0, 1.0,
             1.0, 1.0, 0.0, 1.0, 1.0, 0.0]
    }
    fn get_time(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    /// Called before drawing next frame to make uniform buffers update
    pub fn new_frame(&mut self) -> [f32; 3] {
        let time = self.get_time();
        [time.cos() * 0.5, time.sin() * 0.5, time.cos() * time.sin()]
    }
}