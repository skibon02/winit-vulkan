use std::ops::{Deref, DerefMut};
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_uniform_id, TypedUniformResourceId, UniformResourceId};
use crate::state::{StateDiff};


pub struct UniformResource<L: LayoutInfo> {
    state: StateDiff<L>,
    id: UniformResourceId
}

impl<L: LayoutInfo> Deref for UniformResource<L> {
    type Target = StateDiff<L>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<L: LayoutInfo> DerefMut for UniformResource<L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<L: LayoutInfo> UniformResource<L> {
    pub fn new(v: L) -> Self {
        let uniform_resource_id = get_new_uniform_id();
        Self {
            state: StateDiff::new(v),
            id: uniform_resource_id,
        }
    }

    pub fn id(&self) -> TypedUniformResourceId<L> {
        TypedUniformResourceId {
            id: self.id,
            _p: std::marker::PhantomData
        }
    }
}

