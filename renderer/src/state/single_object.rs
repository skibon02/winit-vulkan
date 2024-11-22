use smallvec::SmallVec;
use crate::object_handles::{get_new_object_id, ObjectId, UniformResourceId};
use crate::pipelines::{AttributesDesc, PipelineDesc, PipelineDescWrapper};
use crate::state::ObjectStateWrapper;

pub struct SingleObject<P: PipelineDesc> {
    pipeline_desc: P,
    new_attributes_per_ins: Option<P::AttributesPerIns>,
    uniform_ids: SmallVec<[(u32, UniformResourceId); 5]>,
    object_id: ObjectId
}
impl<P: PipelineDesc> SingleObject<P> {
    pub fn new(attributes: P::AttributesPerIns, uniforms: P::Uniforms) -> Self {
        let uniform_ids = P::get_uniform_ids(uniforms);
        let object_id = get_new_object_id();
        Self {
            pipeline_desc: P::default(),
            new_attributes_per_ins: Some(attributes),
            uniform_ids,
            object_id
        }
    }

    pub fn id(&self) -> ObjectId {
        self.object_id
    }

    pub fn update(&mut self, s: P::AttributesPerIns) {
        self.new_attributes_per_ins = Some(s);
    }

    pub fn get_pipeline_info(&self) -> fn() -> PipelineDescWrapper {
        P::collect
    }

    pub fn take_state(&mut self) -> Option<ObjectStateWrapper> {
        self.new_attributes_per_ins.as_ref().map(|a| {
            ObjectStateWrapper {
                uniform_bindings: self.uniform_ids.clone(),
                new_attributes: a.as_bytes()
            }
        })
    }
    pub fn clear(&mut self) {
        self.new_attributes_per_ins = None;
    }
}