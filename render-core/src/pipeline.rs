use std::any::TypeId;
use ash::vk;
use ash::vk::{PipelineVertexInputStateCreateInfo, VertexInputAttributeDescription, VertexInputBindingDescription};
use smallvec::{smallvec, SmallVec};
use crate::layout::{LayoutInfo, MemberMeta};
use crate::object_handles::UniformResourceId;

#[derive(Debug, Copy, Clone)]
pub enum VertexAssembly {
    TriangleStrip,
    TriangleList,
}

pub trait PipelineDesc: Default + 'static {
    type PerInsAttrib: LayoutInfo;
    type Uniforms<'a>;
    const SHADERS: (&'static [u8], &'static [u8]);
    fn get_uniform_ids(uniforms: Self::Uniforms<'_>) -> UniformBindingsDesc;
    fn get_uniform_bindings() -> SmallVec<[(u32, UniformBindingType); 5]>;
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

            attributes: Self::PerInsAttrib::get_attributes_configuration(),
            uniform_bindings: Self::get_uniform_bindings(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UniformBindingType {
    UniformBuffer,
    CombinedImageSampler,
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
    pub uniform_bindings: SmallVec<[(u32, UniformBindingType); 5]>,
}

#[derive(Clone, Debug)]
pub struct UniformBindingsDesc {
    pub buffer_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub image_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
}



// Depend on ash for now
#[derive(Debug, Clone)]
pub struct VertexInputDesc {
    attrib_desc: SmallVec<[VertexInputAttributeDescription; 1]>,
    binding_desc: SmallVec<[VertexInputBindingDescription; 1]>,
}

/// Use single binding for now
impl VertexInputDesc {
    pub fn new(members_meta: &'static [MemberMeta], size: usize) -> Self {

        let binding_desc = smallvec![VertexInputBindingDescription::default()
                .binding(0)
                .input_rate(vk::VertexInputRate::INSTANCE)
                .stride(size as u32)];

        let attrib_desc = members_meta.iter().enumerate().map(|(i, member)| {
            VertexInputAttributeDescription::default()
                .binding(0)
                .format(member.ty.format())
                .offset(member.range.start as u32)
                .location(i as u32)
        }).collect::<SmallVec<_>>();
        Self {
            attrib_desc,
            binding_desc,
        }
    }

    pub fn get_input_state_create_info(&mut self) -> PipelineVertexInputStateCreateInfo {
        PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&self.attrib_desc)
            .vertex_binding_descriptions(&self.binding_desc)
    }
}


#[macro_export]
macro_rules! use_shader {
    ($name:expr) => {
        (
            include_bytes!(concat!("../../shaders/compiled/", $name, "_vert.spv")),
            include_bytes!(concat!("../../shaders/compiled/", $name, "_frag.spv"))
        )
    };
}