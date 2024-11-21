use std::any::TypeId;
use smallvec::SmallVec;
use crate::object_handles::UniformResourceId;

pub mod circle;

#[derive(Debug, Copy, Clone)]
pub enum VertexAssembly {
    TriangleStrip,
    TriangleList,
    LineStrip,
    LineList,
}

#[derive(Debug)]
pub struct AttributesDescWrapper {

}

pub trait AttributesDesc {
    fn as_bytes(&self) -> &[u8];
    fn collect() -> AttributesDescWrapper {
        AttributesDescWrapper {}
    }
}

pub trait PipelineDesc: Default + 'static {
    type Attributes: AttributesDesc;
    type Uniforms;
    const SHADERS: (&'static [u8], &'static [u8]);
    fn get_uniform_ids(uniforms: Self::Uniforms) -> SmallVec<[(u32, UniformResourceId); 5]>;
    const VERTEX_ASSEMBLY: VertexAssembly;
    const VERTICES_PER_INSTANCE: u32;

    fn get_id() -> TypeId {
        TypeId::of::<Self>()
    }
    fn collect() -> PipelineDescWrapper {
        PipelineDescWrapper {
            id: Self::get_id(),
            name: std::any::type_name::<Self>(),
            vertex_assembly: Self::VERTEX_ASSEMBLY,
            vertices_per_instance: Self::VERTICES_PER_INSTANCE,
            attributes: Self::Attributes::collect(),
        }
    }
}

#[derive(Debug)]
pub struct PipelineDescWrapper {
    pub id: TypeId,
    pub name: &'static str,
    pub vertex_assembly: VertexAssembly,
    pub vertices_per_instance: u32,
    pub attributes: AttributesDescWrapper,
}
