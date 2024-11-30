use std::sync::{Arc, Mutex, OnceLock};
use jni::JavaVM;
use jni::objects::GlobalRef;

pub static VM: OnceLock<Arc<Mutex<Option<jni::JavaVM>>>> = OnceLock::new();
pub static ACTIVITY: OnceLock<Arc<Mutex<Option<jni::objects::GlobalRef>>>> = OnceLock::new();

pub fn set_android_context(vm: Arc<Mutex<Option<GlobalRef>>>, activity: Arc<Mutex<Option<JavaVM>>>) {
    ACTIVITY.set(vm).unwrap();
    VM.set(activity).unwrap();
}