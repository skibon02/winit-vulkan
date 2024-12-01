pub mod uniform_updates;
pub mod object_updates;
pub mod single_object;
pub mod buffer_updates;

use std::iter;
use crate::collect_state::object_updates::ObjectUpdatesDesc;
use crate::collect_state::uniform_updates::{UniformBufferUpdatesDesc, UniformImageUpdatesDesc};

pub trait UpdatesDesc {
    type ID;
    type New<'a>;
    type Update<'a>;
}

pub enum StateUpdates<'a, T: UpdatesDesc> {
    New(T::New<'a>),
    Update(T::Update<'a>),
    Destroy
}

pub trait CollectDrawStateUpdates {
    fn collect_uniform_buffer_updates(&self) -> impl Iterator<Item=(<UniformBufferUpdatesDesc as UpdatesDesc>::ID, StateUpdates<UniformBufferUpdatesDesc>)>{
        iter::empty()
    }
    fn collect_uniform_image_updates(&self) -> impl Iterator<Item=(<UniformImageUpdatesDesc as UpdatesDesc>::ID, StateUpdates<UniformImageUpdatesDesc>)>{
        iter::empty()
    }
    fn collect_object_updates(&self) -> impl Iterator<Item=(<ObjectUpdatesDesc as UpdatesDesc>::ID, StateUpdates<ObjectUpdatesDesc>)> {
        iter::empty()
    }
    fn clear_updates(&mut self);
}