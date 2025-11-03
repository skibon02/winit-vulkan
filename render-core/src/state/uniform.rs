use std::ops::{Deref, DerefMut};
use crate::collect_state::{CollectDrawStateUpdates, GraphicsUpdateCmd};
use crate::collect_state::uniform_updates::ImageCmd;
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_uniform_id, UniformResourceId};
use crate::state::StateUpdatesBytes;
use crate::{BufferUpdateCmd, UniformBufferCmd};

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
    fn collect_updates(&self) -> impl Iterator<Item=GraphicsUpdateCmd> {
        if self.is_first {
            let r = self.modified_bytes().unwrap();
            Some(GraphicsUpdateCmd::uniform_buffer_update(self.id, UniformBufferCmd::Create(r))).into_iter()
        }
        else {
            self.modified_bytes().map(|r| {
                GraphicsUpdateCmd::uniform_buffer_update(self.id, UniformBufferCmd::Update(
                    BufferUpdateCmd::Update(r)
                ))
            }).into_iter()
        }
    }

    fn clear_updates(&mut self) {
        self.clear_modified();
        self.is_first = false;
    }
}

impl CollectDrawStateUpdates for UniformImageState {
    fn collect_updates(&self) -> impl Iterator<Item=GraphicsUpdateCmd> {
        if self.is_first {
            let path = self.new_image_path.as_ref().unwrap().as_str();
            Some(GraphicsUpdateCmd::Image(self.id(), ImageCmd::Create(path.to_string()))).into_iter()
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