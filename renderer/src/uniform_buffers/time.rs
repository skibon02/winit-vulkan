use std::mem::offset_of;
use crate::layout::{LayoutInfo, MemberMeta};
use crate::layout::types::{uint, GlslTypeVariant};
use crate::state::StateUpdatesBytes;

#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct Time {
    pub time: uint<0>
}

impl LayoutInfo for Time {
    const MEMBERS_META: &'static [MemberMeta] = &[
        MemberMeta {
            name: "time",
            range: offset_of!(Time, time)..offset_of!(Time, time) + size_of::<uint<0>>(),
            ty: GlslTypeVariant::Uint,
        },
    ];
}
impl StateUpdatesBytes<Time> {
    fn set_time(&mut self, time: u32) {
        unsafe {
            self.modify_field(|s| {
                s.time = time.into();
                Time::MEMBERS_META[0].range.clone()
            });
        }
    }
    fn modify_time<F>(&mut self, f: F)
    where F: FnOnce(u32) -> u32 {
        unsafe {
            self.modify_field(|s| {
                s.time = f(s.time.into()).into();
                Time::MEMBERS_META[0].range.clone()
            });
        }
    }
}