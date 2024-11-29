use std::ops::{Deref, DerefMut};
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_uniform_id, UniformResourceId};
use crate::state::{CollectUniformUpdates, StateDiff};


pub struct UniformBufferState<L: LayoutInfo> {
    state: StateDiff<L>,
    id: UniformResourceId
}

impl<L: LayoutInfo> Deref for UniformBufferState<L> {
    type Target = StateDiff<L>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<L: LayoutInfo> DerefMut for UniformBufferState<L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<L: LayoutInfo> UniformBufferState<L> {
    pub fn new(v: L) -> Self {
        let uniform_resource_id = get_new_uniform_id();
        Self {
            state: StateDiff::new(v),
            id: uniform_resource_id,
        }
    }

    pub fn id(&self) -> UniformResourceId {
        self.id
    }
}


pub struct UniformImageState {
    pub id: UniformResourceId,
    pub new_image_path: Option<String>,
}


impl UniformImageState {
    pub fn new(path: String) -> Self {
        let uniform_resource_id = get_new_uniform_id();
        Self {
            id: uniform_resource_id,
            new_image_path: Some(path),
        }
    }
    pub fn id(&self) -> UniformResourceId {
        self.id
    }
}

// updates
pub enum UniformResourceUpdates<'a> {
    ImageResource {
        new_path: Option<&'a str>
    },
    BufferResource {
        modified_bytes: &'a [u8],
        buffer_offset: usize
    }
}

impl<L: LayoutInfo> CollectUniformUpdates for UniformBufferState<L> {
    fn collect_uniform_updates(&self) -> impl Iterator<Item=(UniformResourceId, UniformResourceUpdates)> {
        self.modified_range().map(|s| (self.id(), UniformResourceUpdates::BufferResource {
            modified_bytes: s.0,
            buffer_offset: s.1
        })).into_iter()
    }

    fn clear_uniform_updates(&mut self) {
        self.clear();
    }
}

impl CollectUniformUpdates for UniformImageState {
    fn collect_uniform_updates(&self) -> impl Iterator<Item=(UniformResourceId, UniformResourceUpdates)> {
        self.new_image_path.iter().map(|path| (self.id, UniformResourceUpdates::ImageResource {
            new_path: Some(path)
        }))
    }

    fn clear_uniform_updates(&mut self) {
        self.new_image_path = None;
    }
}