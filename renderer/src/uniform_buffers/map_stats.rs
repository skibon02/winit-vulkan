use std::mem::offset_of;
use crate::layout::{LayoutInfo, MemberMeta};
use crate::layout::types::{float, GlslTypeVariant};
use crate::state::StateDiff;

#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct MapStats {
    pub r: float<0>,
    pub ar: float<0>,
}

impl LayoutInfo for MapStats {
    const MEMBERS_META: &'static [MemberMeta] = &[
        MemberMeta {
            name: "r",
            range: offset_of!(MapStats, r)..offset_of!(MapStats, ar),
            ty: GlslTypeVariant::Float,
        },
        MemberMeta {
            name: "ar",
            range: offset_of!(MapStats, ar)..offset_of!(MapStats, ar) + size_of::<float<0>>(),
            ty: GlslTypeVariant::Float,
        },
    ];
}
impl StateDiff<MapStats> {
    fn set_r(&mut self, r: f32) {
        unsafe {
            self.modify_field(|s| {
                s.r = r.into();
                MapStats::MEMBERS_META[0].range.clone()
            });
        }
    }
    fn modify_r<F>(&mut self, f: F)
    where F: FnOnce(f32) -> f32 {
        unsafe {
            self.modify_field(|s| {
                s.r = f(s.r.into()).into();
                MapStats::MEMBERS_META[0].range.clone()
            });
        }
    }
    fn set_ar(&mut self, ar: f32) {
        unsafe {
            self.modify_field(|s| {
                s.ar = ar.into();
                MapStats::MEMBERS_META[1].range.clone()
            });
        }
    }
    fn modify_ar<F>(&mut self, f: F)
    where F: FnOnce(f32) -> f32 {
        unsafe {
            self.modify_field(|s| {
                s.ar = f(s.ar.into()).into();
                MapStats::MEMBERS_META[1].range.clone()
            });
        }
    }
}
