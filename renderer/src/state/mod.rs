use std::ops::Range;
use smallvec::SmallVec;
use crate::layout::LayoutInfo;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::PipelineDescWrapper;
use crate::state::uniform_state::{CollectUniformUpdates, UniformResourceUpdates};

pub mod single_object;
pub mod object_group;
pub mod uniform_state;

pub struct StateDiff<T: LayoutInfo> {
    inner: T,
    modified: Option<Range<usize>>
}
impl<T: LayoutInfo> StateDiff<T> {
    pub fn new(v: T) -> Self {
        Self {
            inner: v,
            modified: Some(0..T::SIZE)
        }
    }
    pub fn set(&mut self, v: T) {
        self.inner = v;
        self.modified = Some(0..T::SIZE);
    }
    pub fn modify<F>(&mut self, f: F)
    where F: FnOnce(&mut T) {
        f(&mut self.inner);
        self.modified = Some(0..T::SIZE);
    }

    pub unsafe fn modify_field<F>(&mut self, f: F)
    where F: FnOnce(&mut T) -> Range<usize> {
        let range = f(&mut self.inner);
        self.merge_range(range);
    }
    
    fn merge_range(&mut self, r: Range<usize>) {
        self.modified = merge_ranges(self.modified.clone(), r);
    }

    pub fn modified_range(&self) -> Option<(&[u8], usize)> {
       self.modified.as_ref()
            .map(|r| (&self.inner.as_bytes()[r.clone()], r.start))
    }

    pub fn clear(&mut self) {
        self.modified = None;
    }
}


pub fn merge_ranges(r1: Option<Range<usize>>, r2: Range<usize>) -> Option<Range<usize>> {
    match r1 {
        Some(r) => {
            let start = r.start.min(r2.start);
            let end = r.end.max(r2.end);
            Some(start..end)
        },
        None => Some(r2)
    }
}

#[derive(Debug)]
pub struct ObjectStateWrapper<'a> {
    pub buffer_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub image_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub attributes_data: &'a [u8],
    pub data_offset: usize
}

impl ObjectStateTrait for ObjectStateWrapper<'_> {
    type FullState = ObjectStateWrapper<'static>;
    type Updates = ObjectStateWrapper<'static>;
}

pub trait ObjectStateTrait {
    type FullState;
    type Updates;
}
pub enum ResourceOperation<S: ObjectStateTrait> {
    Create(S::FullState),
    Update(S::Updates),
    Remove
}

pub trait CollectObjectUpdates {
    fn collect_object_updates(&self) -> impl Iterator<Item=(ObjectId, ResourceOperation<ObjectStateWrapper>, fn() -> PipelineDescWrapper)>;
    fn clear_object_updates(&mut self);
}

pub trait CollectUniformUpdates {
    fn collect_uniform_updates(&self) -> impl Iterator<Item=(UniformResourceId, UniformResourceUpdates)>;
    fn clear_uniform_updates(&mut self);
}
pub trait DrawStateCollect: CollectUniformUpdates + CollectObjectUpdates {
    fn clear_updates(&mut self) {
        self.clear_uniform_updates();
        self.clear_object_updates();
    }
}