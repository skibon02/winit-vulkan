pub mod uniform;

use std::ops::Range;
use crate::layout::LayoutInfo;

pub struct StateUpdatesBytes<T: LayoutInfo> {
    inner: T,
    modified: Option<Range<usize>>
}
impl<T: LayoutInfo> StateUpdatesBytes<T> {
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


    /// safety: Not intended to use from user code!
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

    pub fn clear_modified(&mut self) {
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