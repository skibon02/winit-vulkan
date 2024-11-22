use smallvec::{smallvec, SmallVec};
use crate::object_handles::{TypedUniformResourceId, UniformResourceId};
use crate::pipelines::{AttributesDesc, PipelineDesc, VertexAssembly};
use crate::uniforms::{MapStats, Time};
use crate::use_shader;
use crate::vulkan_backend::pipeline::VertexInputDesc;

pub struct CircleAttributes {
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub trig_time: u32,
}
#[derive(Default)]
pub struct CirclePipleine;

impl AttributesDesc for CircleAttributes {
    fn get_attributes_configuration() -> VertexInputDesc {
        let vert_desc = VertexInputDesc::new()
            .attrib_4_floats()
            .attrib_2_floats()
            .attrib_u32();

        vert_desc
    }
}

impl PipelineDesc for CirclePipleine {
    type AttributesPerIns = CircleAttributes;
    type Uniforms = (TypedUniformResourceId<Time>, TypedUniformResourceId<MapStats>);
    const SHADERS: (&'static [u8], &'static [u8]) = use_shader!("circle");
    fn get_uniform_ids(uniforms: Self::Uniforms) -> SmallVec<[(u32, UniformResourceId); 5]> {
        let (time, map_stats) = uniforms;
        smallvec![(0, time.id), (1, map_stats.id)]
    }
    fn get_uniform_bindings() -> SmallVec<[u32; 5]> {
        smallvec![0, 1]
    }
    const VERTEX_ASSEMBLY: VertexAssembly = VertexAssembly::TriangleStrip;
    const VERTICES_PER_INSTANCE: usize = 4;
}