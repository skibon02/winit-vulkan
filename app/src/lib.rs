
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit_vulkan::AndroidApp) {
    use framework::{run_android};
    run_android(app);
}