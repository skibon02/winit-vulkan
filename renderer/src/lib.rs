use std::sync::{Arc, Mutex, OnceLock};


pub mod vulkan_backend;
pub mod util;
pub mod pipelines;
pub mod state;
mod object_handles;
mod uniforms;

#[cfg(target_os = "android")]
pub static VM: OnceLock<Arc<Mutex<Option<jni::JavaVM>>>> = OnceLock::new();
#[cfg(target_os = "android")]
pub static ACTIVITY: OnceLock<Arc<Mutex<Option<jni::objects::GlobalRef>>>> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_context(vm: Arc<Mutex<Option<GlobalRef>>>, activity: Arc<Mutex<Option<JavaVM>>>) {
    ACTIVITY.set(vm).unwrap();
    VM.set(activity).unwrap();
}