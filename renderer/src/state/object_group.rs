use crate::pipelines::circle::{CircleAttributes, CirclePipleine};
use crate::state::single_object::SingleObject;
use crate::state::uniform_state::UniformState;
use crate::uniforms::{MapStats, Time};

pub struct ObjectGroup {
    time: UniformState<Time>,
    map_stats: UniformState<MapStats>,
    circle: SingleObject<CirclePipleine>
}

impl ObjectGroup {
    pub fn new() -> ObjectGroup {
        let time = UniformState::new(Time {
            time: 0
        });

        let map_stats = UniformState::new(MapStats {
            r: 300.0,
            ar: 500.0
        });

        let circle = SingleObject::new(CircleAttributes {
            color: [1.0, 1.0, 1.0, 1.0],
            pos: [0.0, 0.0],
            trig_time: 1000,
        }, (&time, &map_stats).clone());

        Self {
            time,
            map_stats,
            circle
        }
    }

    // pub fn collect_uniform_states(&mut self) -> Vec<UniformState<Time>> {
    //     vec![self.time]
    // }
}