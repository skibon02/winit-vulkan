use render::CollectDrawStateUpdates;
use render_core::collect_state::ordered_object_pool::OrderedObjectPool;
use render_core::collect_state::single_object::SingleObject;
use render_core::layout::LayoutInfo;
use render_core::state::uniform::{UniformBufferState, UniformImageState};
use crate::scene::circle::{CircleAttributes, CirclePipleine};
use crate::scene::uniforms::{MapStats, Time};

pub mod uniforms;
pub mod circle;

#[derive(CollectDrawStateUpdates)]
pub struct Scene {
    // uniforms
    pub time: UniformBufferState<Time>,
    pub map_stats: UniformBufferState<MapStats>,
    pub image: UniformImageState,


    // objects
    pub mirror_lamp: SingleObject<CirclePipleine>,
    pub trail: OrderedObjectPool<CirclePipleine, u64>,
}

impl Scene {
    pub fn new(aspect: f32) -> Scene {
        let time = Time {
            time: 0.into()
        }.to_new_uniform();

        let map_stats = MapStats {
            r: 0.2.into(),
            aspect: aspect.into(),
            ar: 1_500.0.into()
        }.to_new_uniform();

        let image = UniformImageState::new("bulb.jpg".to_string());

        let lamp2 = SingleObject::new(CircleAttributes {
            color: [0.6, 0.1, 0.8, 1.0].into(),
            pos: [0.0, 0.0].into(),
            trig_time: i32::MAX.into(),
        }, (&time, &map_stats, &image));
        
        let trail = OrderedObjectPool::new((&time, &map_stats, &image));
        
        Self {
            time,
            map_stats,
            mirror_lamp: lamp2,
            image,
            trail
        }
    }
}
