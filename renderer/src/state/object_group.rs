use crate::layout::LayoutInfo;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::circle::{CircleAttributes, CirclePipleine};
use crate::pipelines::{PipelineDescWrapper};
use crate::state::{DrawStateCollect, ObjectStateWrapper};
use crate::state::single_object::SingleObject;
use crate::state::uniform_state::{CollectUniformUpdates, UniformImageState, UniformBufferState, UniformResourceUpdates};
use crate::uniform_buffers::map_stats::MapStats;
use crate::uniform_buffers::time::Time;

pub struct ObjectGroup {
    pub time: UniformBufferState<Time>,
    pub map_stats: UniformBufferState<MapStats>,
    pub circle: SingleObject<CirclePipleine>,
    pub image: UniformImageState,
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

impl CollectUniformUpdates for ObjectGroup {
    fn collect_uniform_updates(&self) -> impl Iterator<Item=(UniformResourceId, UniformResourceUpdates)> {
        self.time.collect_uniform_updates().chain(
            self.map_stats.collect_uniform_updates()
        ).chain(
            self.image.collect_uniform_updates()
        )
    }
    fn clear_uniform_updates(&mut self) {
        self.time.clear_uniform_updates();
        self.map_stats.clear_uniform_updates();
        self.image.clear_uniform_updates();
    }
}
impl DrawStateCollect for ObjectGroup {
    fn collect_object_updates(&self) -> impl Iterator<Item=(ObjectId, ObjectStateWrapper, fn() -> PipelineDescWrapper)> {
        self.circle.collect_object_updates()
    }
    fn clear_state(&mut self) {
        self.circle.clear();
        self.clear_uniform_updates();
    }
}