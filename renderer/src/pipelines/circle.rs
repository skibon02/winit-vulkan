use crate::pipelines::{Pipeline, VertexAssembly};

pub struct CircleAttributes {
    pub position: [f32; 2],
    pub radius: f32,
    pub color: [f32; 4],
}
#[derive(Default)]
pub struct CirclePipleine;

impl Pipeline for CirclePipleine {
    type Attributes = CircleAttributes;
    type Uniforms = ();
    const VERTEX_ASSEMBLY: VertexAssembly = VertexAssembly::TriangleStrip;
    const VERTICES_PER_INSTANCE: u32 = 4;

}