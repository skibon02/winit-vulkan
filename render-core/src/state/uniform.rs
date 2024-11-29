use std::ops::{Deref, DerefMut};
use crate::collect_state::{CollectDrawStateUpdates, StateUpdates};
use crate::collect_state::uniform_updates::{UniformBufferUpdates, UniformBufferUpdatesDesc, UniformImageUpdatesDesc};
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_uniform_id, UniformResourceId};
use crate::state::StateUpdatesBytes;

pub struct UniformBufferState<L: LayoutInfo> {
    state: StateUpdatesBytes<L>,
    id: UniformResourceId,
    is_first: bool,
}

impl<L: LayoutInfo> Deref for UniformBufferState<L> {
    type Target = StateUpdatesBytes<L>;

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
            state: StateUpdatesBytes::new(v),
            id: uniform_resource_id,
            is_first: true
        }
    }

    pub fn id(&self) -> UniformResourceId {
        self.id
    }
}


pub struct UniformImageState {
    pub id: UniformResourceId,
    pub new_image_path: Option<String>,
    is_first: bool,
}


impl UniformImageState {
    pub fn new(path: String) -> Self {
        let uniform_resource_id = get_new_uniform_id();
        Self {
            id: uniform_resource_id,
            new_image_path: Some(path),
            is_first: true
        }
    }
    pub fn id(&self) -> UniformResourceId {
        self.id
    }
}

// updates

impl<L: LayoutInfo> CollectDrawStateUpdates for UniformBufferState<L> {
    fn collect_uniform_buffer_updates(&self) -> impl Iterator<Item=(UniformResourceId, StateUpdates<UniformBufferUpdatesDesc>)> {
        if self.is_first {
            let r = self.modified_range().unwrap();
            Some((self.id(), StateUpdates::New(UniformBufferUpdates {
                modified_bytes: r.0,
                buffer_offset: r.1
            }))).into_iter()
        }
        else {
            self.modified_range().map(|r| {
                (self.id(), StateUpdates::Update(UniformBufferUpdates {
                    modified_bytes: r.0,
                    buffer_offset: r.1
                }))
            }).into_iter()
        }
    }

    fn clear_updates(&mut self) {
        self.clear_modified();
        self.is_first = false;
    }
}

impl CollectDrawStateUpdates for UniformImageState {
    fn collect_uniform_image_updates(&self) -> impl Iterator<Item=(UniformResourceId, StateUpdates<UniformImageUpdatesDesc>)> {
        if self.is_first {
            let path = self.new_image_path.as_ref().unwrap().as_str();
            Some((self.id, StateUpdates::New(path))).into_iter()
        }
        else {
            None.into_iter()
        }
    }

    fn clear_updates(&mut self) {
        self.new_image_path = None;
        self.is_first = false;
    }
}