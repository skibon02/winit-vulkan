use std::ops::Range;
use smallvec::SmallVec;
use crate::layout::LayoutInfo;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::pipelines::PipelineDescWrapper;

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
    pub uniform_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    pub attributes_data: &'a [u8],
    pub data_offset: usize
}

pub trait DrawStateCollect {
    fn collect_uniform_updates(&self) -> impl Iterator<Item=(UniformResourceId, &[u8], usize)>;
    fn collect_object_updates(&self) -> impl Iterator<Item=(ObjectId, ObjectStateWrapper, fn() -> PipelineDescWrapper)>;
    fn clear_state(&mut self);
}