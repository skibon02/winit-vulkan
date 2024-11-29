use smallvec::SmallVec;
use crate::collect_state::UpdatesDesc;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipeline::PipelineDescWrapper;

#[derive(Debug)]
pub struct ObjectStateWrapper<'a> {
    pub buffer_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub image_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub attributes_data: &'a [u8],
    pub data_offset: usize
}

pub struct ObjectUpdatesDesc;

impl UpdatesDesc for ObjectUpdatesDesc {
    type ID = ObjectId;
    type New<'a> = (ObjectStateWrapper<'a>, fn() -> PipelineDescWrapper);
    type Update<'a> = ObjectStateWrapper<'a>;
}