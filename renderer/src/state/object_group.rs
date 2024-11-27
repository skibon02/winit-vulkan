use crate::layout::LayoutInfo;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::circle::{CircleAttributes, CirclePipleine};
use crate::pipelines::{PipelineDescWrapper};
use crate::state::{DrawStateCollect, ObjectStateWrapper};
use crate::state::single_object::SingleObject;
use crate::state::uniform_state::UniformResource;
use crate::uniforms::{MapStats, Time};

pub struct ObjectGroup {
    pub time: UniformResource<Time>,
    pub map_stats: UniformResource<MapStats>,
    pub circle: SingleObject<CirclePipleine>
}

impl ObjectGroup {
    pub fn new() -> ObjectGroup {
        let time = Time {
            time: 0.into()
        }.to_new_uniform();

        let map_stats = MapStats {
            r: 0.2.into(),
            ar: 500.0.into()
        }.to_new_uniform();

        let circle = SingleObject::new(CircleAttributes {
            color: [1.0, 1.0, 1.0, 1.0].into(),
            pos: [0.0, 0.0].into(),
            trig_time: 1000.into(),
        }, (time.id(), map_stats.id()));

        Self {
            time,
            map_stats,
            circle
        }
    }
}

impl DrawStateCollect for ObjectGroup {
    fn collect_uniform_updates(&self) -> impl Iterator<Item=(UniformResourceId, &[u8], usize)> {
        self.time.modified_range().map(|s| (self.time.id().id, s.0, s.1)).into_iter().chain(
            self.map_stats.modified_range().map(|s| (self.map_stats.id().id, s.0, s.1)).into_iter()
        )
    }

    fn collect_object_updates(&self) -> impl Iterator<Item=(ObjectId, ObjectStateWrapper, fn() -> PipelineDescWrapper)> {
        let id = self.circle.id();
        let pipeline_info = self.circle.get_pipeline_info();
        self.circle.modified_state().map(|s|
            (id, s, pipeline_info)
        ).into_iter()
    }
    fn clear_state(&mut self) {
        self.circle.clear();
        self.map_stats.clear();
        self.time.clear();
    }
}