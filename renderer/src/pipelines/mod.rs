use std::any::TypeId;
use ash::vk::PipelineInputAssemblyStateCreateInfo;
use smallvec::SmallVec;
use crate::object_handles::UniformResourceId;
use crate::vulkan_backend::pipeline::VertexInputDesc;

pub mod circle;

#[derive(Debug, Copy, Clone)]
pub enum VertexAssembly {
    TriangleStrip,
    TriangleList,
}

impl VertexAssembly {
    pub fn get_assembly_create_info(&self) -> PipelineInputAssemblyStateCreateInfo {
        match self {
            VertexAssembly::TriangleStrip => PipelineInputAssemblyStateCreateInfo {
                topology: ash::vk::PrimitiveTopology::TRIANGLE_STRIP,
                primitive_restart_enable: ash::vk::FALSE,
                ..Default::default()
            },
            VertexAssembly::TriangleList => PipelineInputAssemblyStateCreateInfo {
                topology: ash::vk::PrimitiveTopology::TRIANGLE_LIST,
                primitive_restart_enable: ash::vk::FALSE,
                ..Default::default()
            },
        }
    }
}


pub trait AttributesDesc: Sized {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                size_of::<Self>(),
            )
        }
    }
    fn get_attributes_configuration() -> VertexInputDesc;
}

pub trait PipelineDesc: Default + 'static {
    type AttributesPerIns: AttributesDesc;
    type Uniforms;
    const SHADERS: (&'static [u8], &'static [u8]);
    fn get_uniform_ids(uniforms: Self::Uniforms) -> SmallVec<[(u32, UniformResourceId); 5]>;
    fn get_uniform_bindings() -> SmallVec<[u32; 5]>;
    const VERTEX_ASSEMBLY: VertexAssembly;
    const VERTICES_PER_INSTANCE: usize;

    fn get_id() -> TypeId {
        TypeId::of::<Self>()
    }
    fn collect() -> PipelineDescWrapper {
        PipelineDescWrapper {
            id: Self::get_id(),
            name: std::any::type_name::<Self>(),
            vertex_assembly: Self::VERTEX_ASSEMBLY,
            vertices_per_instance: Self::VERTICES_PER_INSTANCE,
            vertex_shader: Self::SHADERS.0,
            fragment_shader: Self::SHADERS.1,

            attributes: Self::AttributesPerIns::get_attributes_configuration(),
            uniform_bindings: Self::get_uniform_bindings(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineDescWrapper {
    pub id: TypeId,
    pub name: &'static str,
    pub vertex_assembly: VertexAssembly,
    pub vertices_per_instance: usize,
    pub vertex_shader: &'static [u8],
    pub fragment_shader: &'static [u8],

    pub attributes: VertexInputDesc,
    pub uniform_bindings: SmallVec<[u32; 5]>,
}
