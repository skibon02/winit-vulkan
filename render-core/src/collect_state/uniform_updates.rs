use crate::collect_state::buffer_updates::{BufferUpdateCmd, BufferUpdateData};

pub enum UniformBufferCmd<'a> {
    Create(BufferUpdateData<'a>),
    Update(BufferUpdateCmd<'a>),
    Destroy
}

pub enum ImageCmd {
    Create(String),
    Destroy
}