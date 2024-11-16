use std::sync::atomic::AtomicUsize;

pub type ObjectId = usize;
static LAST_RC_ID: AtomicUsize = AtomicUsize::new(0);

pub fn get_new_object_id() -> ObjectId {
    LAST_RC_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
