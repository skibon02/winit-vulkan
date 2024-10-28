pub mod app;

extern crate winit_vulkan;

use crate::app::App;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit_vulkan::AndroidApp) {
    use winit_vulkan::{run_android};
    run_android::<App>(app);
}