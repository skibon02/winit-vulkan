use crate::collect_state::buffer_updates::StaticBufferUpdates;
use crate::collect_state::UpdatesDesc;
use crate::object_handles::UniformResourceId;

pub struct UniformBufferUpdatesDesc;
impl UpdatesDesc for UniformBufferUpdatesDesc {
    type ID = UniformResourceId;
    type New<'a> = StaticBufferUpdates<'a>;
    type Update<'a> = StaticBufferUpdates<'a>;
}

pub struct UniformImageUpdatesDesc;
impl UpdatesDesc for UniformImageUpdatesDesc {
    type ID = UniformResourceId;
    type New<'a> = &'a str;
    type Update<'a> = ();
}