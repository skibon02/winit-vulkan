use crate::collect_state::UpdatesDesc;
use crate::object_handles::UniformResourceId;

pub struct UniformBufferUpdates<'a> {
    pub modified_bytes: &'a [u8],
    pub buffer_offset: usize
}

pub struct UniformBufferUpdatesDesc;
impl UpdatesDesc for UniformBufferUpdatesDesc {
    type ID = UniformResourceId;
    type New<'a> = UniformBufferUpdates<'a>;
    type Update<'a> = UniformBufferUpdates<'a>;
}

pub struct UniformImageUpdatesDesc;
impl UpdatesDesc for UniformImageUpdatesDesc {
    type ID = UniformResourceId;
    type New<'a> = &'a str;
    type Update<'a> = ();
}