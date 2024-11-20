use glsl_layout::Uniform;
use crate::state::uniform_state::UniformDesc;

#[derive(Uniform, Copy, Clone)]
pub struct MapStats {
    pub r: f32,
    pub ar: f32,
}

impl UniformDesc for MapStats { }

#[derive(Uniform, Copy, Clone)]
pub struct Time {
    pub time: u32
}

impl UniformDesc for Time { }