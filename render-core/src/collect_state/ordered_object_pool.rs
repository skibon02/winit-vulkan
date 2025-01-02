use std::collections::BTreeMap;
use std::fmt::Display;
use std::mem;
use crate::collect_state::CollectDrawStateUpdates;
use crate::{BufferUpdateCmd, GraphicsUpdateCmd, ObjectUpdate2DCmd};
use crate::object_handles::{get_new_object_id, ObjectId};
use crate::pipeline::{PipelineDesc, PipelineDescWrapper, UniformBindingsDesc};
use crate::state::StateUpdatesBytes;

pub struct OrderedObjectPool<P: PipelineDesc, K: Ord> {
    pipeline_desc: P, 
    
    objects_per_ins_attrib: BTreeMap<K, (ObjectId, StateUpdatesBytes<P::PerInsAttrib>, bool)>,
    uniform_bindings: UniformBindingsDesc,
    
    removed_ids: Vec<ObjectId>,
}

impl<P: PipelineDesc, K: Ord> OrderedObjectPool<P, K>
    where P::PerInsAttrib: Default {
    /// Create new empty object pool
    pub fn new(uniforms: P::Uniforms<'_>) -> Self {
        Self {
            pipeline_desc: P::default(),
            uniform_bindings: P::get_uniform_ids(uniforms),
            objects_per_ins_attrib: BTreeMap::new(),
            removed_ids: Vec::new(),
        }
    }
    
    /// Get per instance attributes for object with given key
    /// 
    /// If object with given key does not exist, it will be created with default attributes
    pub fn entry(&mut self, key: K) -> &mut StateUpdatesBytes<P::PerInsAttrib> {
        &mut self.objects_per_ins_attrib.entry(key).or_insert_with(|| {
            let object_id = get_new_object_id();
            (object_id, StateUpdatesBytes::default(), true)
        }).1
    }
    
    /// Create new object with given key and attributes
    /// 
    /// If object with given key already exists, it will be not be modified
    pub fn create(&mut self, key: K, attrib: P::PerInsAttrib) {
        if self.objects_per_ins_attrib.contains_key(&key) {
            return;
        }
        let object_id = get_new_object_id();
        self.objects_per_ins_attrib.insert(key, (object_id, StateUpdatesBytes::new(attrib), true));
    }
    
    /// Remove object with given key
    pub fn remove(&mut self, key: &K) -> bool {
        if let Some(removed) = self.objects_per_ins_attrib.remove(key) {
            self.removed_ids.push(removed.0);
            true
        }
        else {
            false
        }
    }
    
    
    /// Remove all objects with key less than given threshold
    pub fn auto_remove(&mut self, key_threshold: K) where K: Display {
        if self.objects_per_ins_attrib.iter().any(|(key, _)| key < &key_threshold) {
            let retained = self.objects_per_ins_attrib.split_off(&key_threshold);
            for (_, (id, _, is_new)) in mem::take(&mut self.objects_per_ins_attrib) {
                if !is_new {
                    self.removed_ids.push(id);
                }
            }
            self.objects_per_ins_attrib = retained;
        }
    }

    pub fn get_pipeline_info(&self) -> fn() -> PipelineDescWrapper {
        P::collect
    }
}

// updates
impl<K: Ord, P: PipelineDesc> CollectDrawStateUpdates for OrderedObjectPool<P, K>
    where P::PerInsAttrib: Default {
    fn collect_updates(&self) -> impl Iterator<Item=GraphicsUpdateCmd> {
        let removed = self.removed_ids.iter().map(|id| GraphicsUpdateCmd::object_update_2d(*id, ObjectUpdate2DCmd::Destroy));
        
        let updated = self.objects_per_ins_attrib.iter().filter_map(|(_, (id, attrib, is_new))| {
            if *is_new {
                let pipeline_info = self.get_pipeline_info();
                let s = attrib.modified_bytes().unwrap();
                Some(GraphicsUpdateCmd::object_update_2d(*id, ObjectUpdate2DCmd::Create {
                    pipeline_desc: pipeline_info,
                    uniform_bindings_desc: self.uniform_bindings.clone(),
                    initial_state: s
                }))
            }
            else {
                attrib.modified_bytes().map(|s| 
                    GraphicsUpdateCmd::object_update_2d(*id, ObjectUpdate2DCmd::AttribUpdate(BufferUpdateCmd::Update(s))))
            }
        });
        
        removed.chain(updated)
    }

    fn clear_updates(&mut self) {
        for (_, (_, attrib, is_new)) in self.objects_per_ins_attrib.iter_mut() {
            attrib.clear_modified();
            *is_new = false;
        }
        self.removed_ids.clear();
    }
}