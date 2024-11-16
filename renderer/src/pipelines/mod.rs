pub mod circle;

pub enum VertexAssembly {
    TriangleStrip,
    TriangleList,
    LineStrip,
    LineList,
}
pub trait Pipeline: Default {
    type Attributes;
    type Uniforms;
    const VERTEX_ASSEMBLY: VertexAssembly;
    const VERTICES_PER_INSTANCE: u32;
}