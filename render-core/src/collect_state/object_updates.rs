use crate::collect_state::buffer_updates::StaticBufferUpdates;
use crate::collect_state::UpdatesDesc;
use crate::object_handles::{ObjectId,};
use crate::pipeline::{PipelineDescWrapper, UniformBindingsDesc};

pub struct ObjectUpdatesDesc;

impl UpdatesDesc for ObjectUpdatesDesc {
    type ID = ObjectId;
    type New<'a> = (StaticBufferUpdates<'a>, UniformBindingsDesc, fn() -> PipelineDescWrapper);
    type Update<'a> = StaticBufferUpdates<'a>;
}