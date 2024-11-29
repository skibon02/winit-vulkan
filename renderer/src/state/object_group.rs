use crate::layout::LayoutInfo;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::circle::{CircleAttributes, CirclePipleine};
use crate::collect_state::{CollectDrawStateUpdates, StateUpdates, UpdatesDesc};
use crate::collect_state::object_updates::{ObjectUpdatesDesc};
use crate::collect_state::uniform_updates::{UniformBufferUpdatesDesc, UniformImageUpdatesDesc};
use crate::state::single_object::SingleObject;
use crate::state::uniform::{UniformBufferState, UniformImageState};
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


// this thing will be #[derive]'d
impl CollectDrawStateUpdates for ObjectGroup {
    fn collect_uniform_buffer_updates(&self) -> impl Iterator<Item=(UniformResourceId, StateUpdates<UniformBufferUpdatesDesc>)> {
        self.time.collect_uniform_buffer_updates().chain(
            self.map_stats.collect_uniform_buffer_updates()
        )
    }
    fn collect_uniform_image_updates(&self) -> impl Iterator<Item=(<UniformImageUpdatesDesc as UpdatesDesc>::ID, StateUpdates<UniformImageUpdatesDesc>)> {
        self.image.collect_uniform_image_updates()
    }
    fn collect_object_updates(&self) -> impl Iterator<Item=(<ObjectUpdatesDesc as UpdatesDesc>::ID, StateUpdates<ObjectUpdatesDesc>)> {
        self.circle.collect_object_updates()
    }
    fn clear_updates(&mut self) {
        self.time.clear_updates();
        self.map_stats.clear_updates();
        self.image.clear_updates();

        self.circle.clear_updates();
    }
}