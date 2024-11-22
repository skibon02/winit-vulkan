use glsl_layout::Std140;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::circle::{CircleAttributes, CirclePipleine};
use crate::pipelines::{PipelineDescWrapper};
use crate::state::{DrawStateCollect, ObjectStateWrapper};
use crate::state::single_object::SingleObject;
use crate::state::uniform_state::UniformState;
use crate::uniforms::{MapStats, Time};

pub struct ObjectGroup {
    pub time: UniformState<Time>,
    pub map_stats: UniformState<MapStats>,
    pub circle: SingleObject<CirclePipleine>
}

impl ObjectGroup {
    pub fn new() -> ObjectGroup {
        let time = UniformState::new(Time {
            time: 0
        });

        let map_stats = UniformState::new(MapStats {
            r: 0.5,
            ar: 500.0
        });

        let circle = SingleObject::new(CircleAttributes {
            color: [1.0, 1.0, 1.0, 1.0].into(),
            pos: [0.0, 0.0].into(),
            trig_time: 1000,
        }, (time.id(), map_stats.id()));

        Self {
            time,
            map_stats,
            circle
        }
    }
}

impl DrawStateCollect for ObjectGroup {
    fn collect_uniform_updates(&mut self) -> impl Iterator<Item=(UniformResourceId, Vec<u8>)> {
        self.time.take_state().map(|s| (self.time.id().id, s.as_raw().to_vec())).into_iter().chain(
            self.map_stats.take_state().map(|s| (self.map_stats.id().id, s.as_raw().to_vec())).into_iter()
        )
    }

    fn collect_object_updates(&mut self) -> impl Iterator<Item=(ObjectId, ObjectStateWrapper, fn() -> PipelineDescWrapper)> {
        let id = self.circle.id();
        let pipeline_info = self.circle.get_pipeline_info();
        self.circle.take_state().map(|s|
            (id, s, pipeline_info)
        ).into_iter()
    }
    fn clear_state(&mut self) {
        self.circle.clear();
        self.map_stats.clear();
        self.time.clear();
    }
}