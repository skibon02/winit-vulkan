use std::sync::atomic::{AtomicUsize, Ordering};

pub type ObjectId = usize;
static LAST_RC_ID: AtomicUsize = AtomicUsize::new(0);

pub fn get_new_object_id() -> ObjectId {
    LAST_RC_ID.fetch_add(1, Ordering::SeqCst)
}

pub type UniformResourceId = usize;


static LAST_UNIFORM_RESOURCE_ID: AtomicUsize = AtomicUsize::new(0);
pub fn get_new_uniform_id() -> UniformResourceId {
    LAST_UNIFORM_RESOURCE_ID.fetch_add(1, Ordering::SeqCst)
}