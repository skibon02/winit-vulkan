use std::ops::{Deref, DerefMut};
use crate::collect_state::{CollectDrawStateUpdates, GraphicsUpdateCmd};
use crate::layout::LayoutInfo;
use crate::object_handles::{get_new_object_id, ObjectId};
use crate::{BufferUpdateCmd, ObjectUpdate2DCmd};
use crate::pipeline::{PipelineDesc, PipelineDescWrapper, UniformBindingsDesc};
use crate::state::StateUpdatesBytes;

pub struct SingleObject<P: PipelineDesc> {
    pipeline_desc: P,

    per_ins_attrib: StateUpdatesBytes<P::PerInsAttrib>,
    uniform_bindings: UniformBindingsDesc,
    object_id: ObjectId,

    is_first: bool
}
impl<P: PipelineDesc> SingleObject<P> {
    pub fn new(attributes: P::PerInsAttrib, uniforms: P::Uniforms<'_>) -> Self {
        let uniform_bindings = P::get_uniform_ids(uniforms);
        let object_id = get_new_object_id();
        Self {
            pipeline_desc: P::default(),
            per_ins_attrib: attributes.to_state(),
            uniform_bindings,
            object_id,

            is_first: true
        }
    }

    pub fn id(&self) -> ObjectId {
        self.object_id
    }

    pub fn get_pipeline_info(&self) -> fn() -> PipelineDescWrapper {
        P::collect
    }
}

impl<P: PipelineDesc> Deref for SingleObject<P> {
    type Target = StateUpdatesBytes<P::PerInsAttrib>;

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
impl<P: PipelineDesc> CollectDrawStateUpdates for SingleObject<P> {
    fn collect_updates(&self) -> impl Iterator<Item=GraphicsUpdateCmd> {
        let id = self.id();

        if self.is_first {
            let pipeline_info = self.get_pipeline_info();
            let s = self.per_ins_attrib.modified_bytes().unwrap();
            Some(GraphicsUpdateCmd::object_update_2d(id, ObjectUpdate2DCmd::Create {
                pipeline_desc: pipeline_info,
                uniform_bindings_desc: self.uniform_bindings.clone(),
                initial_state: s
            })).into_iter()
        }
        else {
            self.per_ins_attrib.modified_bytes().map(|s|
                GraphicsUpdateCmd::object_update_2d(id, ObjectUpdate2DCmd::AttribUpdate(BufferUpdateCmd::Update(s)))
            ).into_iter()
        }
    }
    fn clear_updates(&mut self) {
        self.clear_modified();
        self.is_first = false;
    }
}