use glsl_layout::Uniform;
use crate::object_handles::{get_new_uniform_id, UniformResourceId};

pub trait UniformDesc: Sized + Uniform {
    fn get_buffer_size(&self) -> usize {
        size_of::<Self>()
    }
}


pub struct UniformState<U: UniformDesc> {
    new_state: Option<U>,
    id: UniformResourceId
}

impl<U: UniformDesc> UniformState<U> {
    pub fn new(u: U) -> Self {
        let uniform_resource_id = get_new_uniform_id();
        Self {
            new_state: Some(u),
            id: uniform_resource_id,
        }
    }

    pub fn update(&mut self, u: U) {
        self.new_state = Some(u);
    }

    pub fn take_state(&mut self) -> Option<U::Std140> {
        self.new_state.take().map(|s| s.std140())
    }

    pub fn id(&self) -> UniformResourceId {
        self.id
    }
}

