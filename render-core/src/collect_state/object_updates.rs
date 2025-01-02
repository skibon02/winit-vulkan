use crate::BufferUpdateCmd;
use crate::collect_state::buffer_updates::BufferUpdateData;
use crate::pipeline::{PipelineDescWrapper, UniformBindingsDesc};

pub enum ObjectUpdate2DCmd<'a> {
    Create {
        pipeline_desc: fn() -> PipelineDescWrapper,
        uniform_bindings_desc: UniformBindingsDesc,
        initial_state: BufferUpdateData<'a>
    },
    AttribUpdate(BufferUpdateCmd<'a>),
    Destroy
}