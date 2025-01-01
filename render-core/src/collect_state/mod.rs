pub mod uniform_updates;
pub mod object_updates;
pub mod single_object;
pub mod buffer_updates;

use std::iter;
use crate::object_handles::{ObjectId, UniformResourceId};
use crate::{ObjectUpdate2DCmd, UniformBufferCmd};
use crate::collect_state::uniform_updates::ImageCmd;

pub trait CollectDrawStateUpdates {
    fn collect_updates(&self) -> impl Iterator<Item=GraphicsUpdateCmd>;
    fn clear_updates(&mut self);
}

pub enum GraphicsUpdateCmd<'a> {
    ObjectUpdate2D(ObjectId, ObjectUpdate2DCmd<'a>),
    UniformBufferUpdate(UniformResourceId, UniformBufferCmd<'a>),
    ImageUpdate(UniformResourceId, ImageCmd),
}

impl<'a> GraphicsUpdateCmd<'a> {
    pub fn object_update_2d(id: ObjectId, cmd: ObjectUpdate2DCmd<'a>) -> Self {
        GraphicsUpdateCmd::ObjectUpdate2D(id, cmd)
    }

    pub fn uniform_buffer_update(id: UniformResourceId, cmd: UniformBufferCmd<'a>) -> Self {
        GraphicsUpdateCmd::UniformBufferUpdate(id, cmd)
    }

    pub fn image_update(id: UniformResourceId, cmd: ImageCmd) -> Self {
        GraphicsUpdateCmd::ImageUpdate(id, cmd)
    }
}