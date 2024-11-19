use std::collections::BTreeMap;
use crate::object_handles::UniformResourceId;
use crate::pipelines::{PipelineDesc, VertexAssembly};
use crate::state::uniform_state::UniformState;
use crate::uniforms::{MapStats, Time};

pub struct CircleAttributes {
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub trig_time: u32,
}
#[derive(Default)]
pub struct CirclePipleine;

impl PipelineDesc for CirclePipleine {
    type Attributes = CircleAttributes;
    type Uniforms = (UniformState<MapStats>, UniformState<Time>);
    fn get_uniform_ids(uniforms: Self::Uniforms) -> BTreeMap<u32, UniformResourceId> {
        let (map_stats, time) = uniforms;
        let mut uniform_ids = BTreeMap::new();
        uniform_ids.insert(0, map_stats.id());
        uniform_ids.insert(1, time.id());
        uniform_ids
    }
    const VERTEX_ASSEMBLY: VertexAssembly = VertexAssembly::TriangleStrip;
    const VERTICES_PER_INSTANCE: u32 = 4;
}