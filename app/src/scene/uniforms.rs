use std::mem::offset_of;
use render::define_layout;
use render_core::layout::{LayoutInfo, MemberMeta};
use render_core::layout::types::{float, int, GlslTypeVariant};
use render_core::state::StateUpdatesBytes;

define_layout! {
    pub struct MapStats {
        pub r: float<0>,
        pub ar: float<0>,
        pub aspect: float<0>
    }
}

define_layout! {
    pub struct Time {
        pub time: int<0>
    }
}