use std::collections::BTreeMap;
use crate::object_handles::UniformResourceId;
use crate::pipelines::PipelineDesc;

pub struct SingleObject<P: PipelineDesc> {
    pipeline: P,
    new_attributes: Option<P::Attributes>,
    uniform_ids: BTreeMap<u32, UniformResourceId>,
}
impl<P: PipelineDesc> SingleObject<P> {
    pub fn new(attributes: P::Attributes, uniforms: P::Uniforms) -> Self {
        let uniform_ids = P::get_uniform_ids(uniforms);
        Self {
            pipeline: P::default(),
            new_attributes: Some(attributes),
            uniform_ids,
        }
    }

    fn take_state(&mut self) -> Option<P::Attributes> {
        self.new_attributes.take()
    }
}