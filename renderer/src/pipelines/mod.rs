use std::collections::BTreeMap;
use crate::object_handles::UniformResourceId;

pub mod circle;

pub enum VertexAssembly {
    TriangleStrip,
    TriangleList,
    LineStrip,
    LineList,
}
pub trait PipelineDesc: Default {
    type Attributes;
    type Uniforms;
    fn get_uniform_ids(uniforms: Self::Uniforms) -> BTreeMap<u32, UniformResourceId>;
    const VERTEX_ASSEMBLY: VertexAssembly;
    const VERTICES_PER_INSTANCE: u32;
}