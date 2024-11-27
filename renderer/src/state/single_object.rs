use std::ops::{Deref, DerefMut};
use smallvec::SmallVec;
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_object_id, ObjectId, UniformResourceId};
use crate::pipelines::{ PipelineDesc, PipelineDescWrapper};
use crate::state::{ObjectStateWrapper, StateDiff};

pub struct SingleObject<P: PipelineDesc> {
    pipeline_desc: P,

    per_ins_attrib: StateDiff<P::PerInsAttrib>,
    uniform_ids: SmallVec<[(u32, UniformResourceId); 5]>,
    object_id: ObjectId
}
impl<P: PipelineDesc> SingleObject<P> {
    pub fn new(attributes: P::PerInsAttrib, uniforms: P::Uniforms) -> Self {
        let uniform_ids = P::get_uniform_ids(uniforms);
        let object_id = get_new_object_id();
        Self {
            pipeline_desc: P::default(),
            per_ins_attrib: attributes.to_state(),
            uniform_ids,
            object_id
        }
    }

    pub fn id(&self) -> ObjectId {
        self.object_id
    }

    pub fn get_pipeline_info(&self) -> fn() -> PipelineDescWrapper {
        P::collect
    }

    pub fn modified_state(&self) -> Option<ObjectStateWrapper> {
        self.per_ins_attrib.modified_range().map(|a| {
            ObjectStateWrapper {
                uniform_bindings: self.uniform_ids.clone(),
                attributes_data: a.0,
                data_offset: a.1
            }
        })
    }
}

impl<P: PipelineDesc> Deref for SingleObject<P> {
    type Target = StateDiff<P::PerInsAttrib>;

    fn deref(&self) -> &Self::Target {
        &self.per_ins_attrib
    }
}

impl<P: PipelineDesc> DerefMut for SingleObject<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.per_ins_attrib
    }
}