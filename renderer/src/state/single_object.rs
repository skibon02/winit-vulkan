use std::ops::{Deref, DerefMut};
use smallvec::SmallVec;
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_object_id, ObjectId, UniformResourceId};
use crate::pipelines::{ PipelineDesc, PipelineDescWrapper};
use crate::state::{ObjectStateWrapper, StateDiff};

pub struct SingleObject<P: PipelineDesc> {
    pipeline_desc: P,

    per_ins_attrib: StateDiff<P::PerInsAttrib>,
    image_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    buffer_bindings: SmallVec<[(u32, UniformResourceId); 5]>,
    object_id: ObjectId
}
impl<P: PipelineDesc> SingleObject<P> {
    pub fn new(attributes: P::PerInsAttrib, uniforms: P::Uniforms<'_>) -> Self {
        let (buffer_ids, image_ids) = P::get_uniform_ids(uniforms);
        let object_id = get_new_object_id();
        Self {
            pipeline_desc: P::default(),
            per_ins_attrib: attributes.to_state(),
            image_bindings: image_ids,
            buffer_bindings: buffer_ids,
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
                image_bindings: self.image_bindings.clone(),
                attributes_data: a.0,
                data_offset: a.1,
                buffer_bindings: self.buffer_bindings.clone(),
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


// updates
impl<P: PipelineDesc> CollectObjectUpdates for SingleObject<P> {
    fn collect_object_updates(&self) -> impl Iterator<Item=(ObjectId, ObjectStateWrapper, fn() -> PipelineDescWrapper)> {
        let id = self.id();
        let pipeline_info = self.get_pipeline_info();
        self.modified_state().map(|s|
            (id, s, pipeline_info)
        ).into_iter()
    }
    fn clear(&mut self) {
        self.clear_state();
    }
}