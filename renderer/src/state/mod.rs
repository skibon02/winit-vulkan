use smallvec::SmallVec;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::PipelineDescWrapper;

pub mod single_object;
pub mod uniform_state;
pub mod object_group;


#[derive(Debug)]
pub struct ObjectStateWrapper<'a> {
    pub uniform_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub new_attributes: &'a [u8]
}

pub trait DrawStateCollect {
    fn collect_uniform_states(&mut self) -> impl Iterator<Item=(UniformResourceId, Vec<u8>)>;
    fn collect_object_states(&mut self) -> impl Iterator<Item=(ObjectId, ObjectStateWrapper, fn() -> PipelineDescWrapper)>;
    fn clear_state(&mut self);
}