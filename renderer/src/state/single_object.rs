use crate::pipelines::Pipeline;

pub struct SingleObject<P: Pipeline> {
    pipeline: P,
    attributes: P::Attributes,
    uniforms: P::Uniforms,
}
impl<P: Pipeline> SingleObject<P> {
    pub fn new(attributes: P::Attributes, uniforms: P::Uniforms) -> Self {
        Self {
            pipeline: P::default(),
            attributes,
            uniforms,
        }
    }

    fn take_state(&mut self) {

    }
}