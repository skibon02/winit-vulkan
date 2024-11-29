use std::mem::offset_of;
use smallvec::{smallvec, SmallVec};
use crate::layout::{LayoutInfo, MemberMeta};
use crate::layout::types::*;
use crate::object_handles::{UniformResourceId};
use crate::pipelines::{PipelineDesc, UniformBindingType, VertexAssembly};
use crate::state::StateDiff;
use crate::state::uniform_state::{UniformImageState, UniformBufferState};
use crate::uniform_buffers::map_stats::MapStats;
use crate::uniform_buffers::time::Time;
use crate::use_shader;
use crate::vulkan_backend::pipeline::VertexInputDesc;


#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct CircleAttributes {
    pub color: vec4<0>,
    pub pos: vec2<0>,
    pub trig_time: uint<0>,
}
impl LayoutInfo for CircleAttributes {
    const MEMBERS_META: &'static [MemberMeta] = &[
        MemberMeta {
            name: "color",
            range: offset_of!(CircleAttributes, color)..offset_of!(CircleAttributes, pos) ,
            ty: GlslTypeVariant::Vec4,
        },
        MemberMeta {
            name: "pos",
            range: offset_of!(CircleAttributes, pos)..offset_of!(CircleAttributes, trig_time) ,
            ty: GlslTypeVariant::Vec2,
        },
        MemberMeta {
            name: "trig_time",
            range: offset_of!(CircleAttributes, trig_time)..offset_of!(CircleAttributes, trig_time) + size_of::<uint<0>>() ,
            ty: GlslTypeVariant::Uint,
        },
    ];
}

impl StateDiff<CircleAttributes> {
    pub fn set_color(&mut self, color: [f32; 4]) {
        unsafe {
            self.modify_field(|s| {
                s.color = color.into();
                CircleAttributes::MEMBERS_META[0].range.clone()
            });
        }
    }
    pub fn modify_color<F>(&mut self, f: F)
    where F: FnOnce([f32; 4]) -> [f32; 4] {
        unsafe {
            self.modify_field(|s| {
                s.color = f(s.color.into()).into();
                CircleAttributes::MEMBERS_META[0].range.clone()
            });
        }
    }
    pub fn set_pos(&mut self, pos: [f32; 2]) {
        unsafe {
            self.modify_field(|s| {
                s.pos = pos.into();
                CircleAttributes::MEMBERS_META[1].range.clone()
            });
        }
    }
    pub fn modify_pos<F>(&mut self, f: F)
    where F: FnOnce([f32; 2]) -> [f32; 2] {
        unsafe {
            self.modify_field(|s| {
                s.pos = f(s.pos.into()).into();
                CircleAttributes::MEMBERS_META[1].range.clone()
            });
        }
    }   
    pub fn set_trig_time(&mut self, trig_time: u32) {
        unsafe {
            self.modify_field(|s| {
                s.trig_time = trig_time.into();
                CircleAttributes::MEMBERS_META[2].range.clone()
            });
        }
    }
    pub fn modify_trig_time<F>(&mut self, f: F)
    where F: FnOnce(u32) -> u32
    {
        unsafe {
            self.modify_field(|s| {
                s.trig_time = f(s.trig_time.into()).into();
                CircleAttributes::MEMBERS_META[2].range.clone()
            });
        }
    }
}
#[derive(Default)]
pub struct CirclePipleine;

impl PipelineDesc for CirclePipleine {
    type PerInsAttrib = CircleAttributes;
    type Uniforms<'a> = (&'a UniformBufferState<Time>, &'a UniformBufferState<MapStats>, &'a UniformImageState);
    const SHADERS: (&'static [u8], &'static [u8]) = use_shader!("circle");
    fn get_uniform_ids(uniforms: Self::Uniforms<'_>) -> (SmallVec<[(u32, UniformResourceId); 5]>, SmallVec<[(u32, UniformResourceId); 5]>) {
        let (time, map_stats, image) = uniforms;
        (smallvec![(0, time.id()), (1, map_stats.id())], smallvec![(2, image.id())])
    }
    fn get_uniform_bindings() -> SmallVec<[(u32, UniformBindingType); 5]> {
        smallvec![(0, UniformBindingType::UniformBuffer),
            (1, UniformBindingType::UniformBuffer),
            (2, UniformBindingType::CombinedImageSampler)]
    }
    const VERTEX_ASSEMBLY: VertexAssembly = VertexAssembly::TriangleStrip;
    const VERTICES_PER_INSTANCE: usize = 4;
}