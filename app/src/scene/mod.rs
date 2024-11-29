use render::CollectDrawStateUpdates;
use render_core::collect_state::{UpdatesDesc};
use render_core::collect_state::single_object::SingleObject;
use render_core::layout::LayoutInfo;
use render_core::state::uniform::{UniformBufferState, UniformImageState};
use crate::scene::circle::{CircleAttributes, CirclePipleine};
use crate::scene::uniforms::{MapStats, Time};

pub mod uniforms;
pub mod circle;

#[derive(CollectDrawStateUpdates)]
pub struct Scene {
    pub time: UniformBufferState<Time>,
    pub map_stats: UniformBufferState<MapStats>,
    pub circle: SingleObject<CirclePipleine>,
    pub image: UniformImageState,
}

impl Scene {
    pub fn new() -> Scene {
        let time = Time {
            time: 0.into()
        }.to_new_uniform();

        let map_stats = MapStats {
            r: 0.2.into(),
            ar: 500.0.into()
        }.to_new_uniform();

        let image = UniformImageState::new("bulb.jpg".to_string());

        let circle = SingleObject::new(CircleAttributes {
            color: [1.0, 1.0, 1.0, 1.0].into(),
            pos: [0.0, 0.0].into(),
            trig_time: 1000.into(),
        }, (&time, &map_stats, &image));

        Self {
            time,
            map_stats,
            circle,
            image
        }
    }
}
