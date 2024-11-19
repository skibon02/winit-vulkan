use glsl_layout::Uniform;
use crate::state::uniform_state::UniformDesc;

#[derive(Uniform, Copy, Clone)]
pub struct MapStats {
    r: f32,
    ar: f32,
}

impl UniformDesc for MapStats { }

#[derive(Uniform, Copy, Clone)]
pub struct Time {
    time: u32
}

impl UniformDesc for Time { }