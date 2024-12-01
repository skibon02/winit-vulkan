use std::mem::offset_of;
use smallvec::{smallvec, SmallVec};
use render::define_layout;
use render_core::layout::{LayoutInfo, MemberMeta};
use render_core::layout::types::*;
use render_core::object_handles::UniformResourceId;
use render_core::pipeline::{PipelineDesc, UniformBindingType, UniformBindingsDesc, VertexAssembly};
use render_core::state::StateUpdatesBytes;
use render_core::state::uniform::{UniformBufferState, UniformImageState};
use render_core::use_shader;
use crate::scene::uniforms::{MapStats, Time};

define_layout! {
    pub struct CircleAttributes {
        pub color: vec4<0>,
        pub pos: vec2<0>,
        pub trig_time: uint<0>,
    }
}

#[derive(Default)]
pub struct CirclePipleine;

impl PipelineDesc for CirclePipleine {
    type PerInsAttrib = CircleAttributes;
    type Uniforms<'a> = (&'a UniformBufferState<Time>, &'a UniformBufferState<MapStats>, &'a UniformImageState);
    const SHADERS: (&'static [u8], &'static [u8]) = use_shader!("circle");
    fn get_uniform_ids(uniforms: Self::Uniforms<'_>) -> UniformBindingsDesc {
        let (time, map_stats, image) = uniforms;
        UniformBindingsDesc {
            image_bindings: smallvec![(2, image.id())],
            buffer_bindings: smallvec![(0, time.id()), (1, map_stats.id())],
        }
    }
    fn get_uniform_bindings() -> SmallVec<[(u32, UniformBindingType); 5]> {
        smallvec![(0, UniformBindingType::UniformBuffer),
            (1, UniformBindingType::UniformBuffer),
            (2, UniformBindingType::CombinedImageSampler)]
    }
    const VERTEX_ASSEMBLY: VertexAssembly = VertexAssembly::TriangleStrip;
    const VERTICES_PER_INSTANCE: usize = 4;
}