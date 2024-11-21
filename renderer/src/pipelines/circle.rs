use std::collections::BTreeMap;
use smallvec::SmallVec;
use crate::object_handles::{TypedUniformResourceId, UniformResourceId};
use crate::pipelines::{AttributesDesc, PipelineDesc, VertexAssembly};
use crate::state::uniform_state::UniformState;
use crate::uniforms::{MapStats, Time};
use crate::use_shader;

pub struct CircleAttributes {
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub trig_time: u32,
}
#[derive(Default)]
pub struct CirclePipleine;

impl AttributesDesc for CircleAttributes {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const CircleAttributes as *const u8,
                std::mem::size_of::<CircleAttributes>(),
            )
        }
    }
}

impl PipelineDesc for CirclePipleine {
    type Attributes = CircleAttributes;
    type Uniforms = (TypedUniformResourceId<Time>, TypedUniformResourceId<MapStats>);
    const SHADERS: (&'static [u8], &'static [u8]) = use_shader!("circle");
    fn get_uniform_ids(uniforms: Self::Uniforms) -> SmallVec<[(u32, UniformResourceId); 5]> {
        let (map_stats, time) = uniforms;
        let mut uniform_ids = SmallVec::new();
        uniform_ids.push((0, map_stats.id));
        uniform_ids.push((1, time.id));
        uniform_ids
    }
    const VERTEX_ASSEMBLY: VertexAssembly = VertexAssembly::TriangleStrip;
    const VERTICES_PER_INSTANCE: u32 = 4;
}