#[cfg(target_os = "android")]
pub mod android;

pub mod winit;
pub mod scene;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: crate::winit::AndroidApp) {
    use crate::winit::run_android;
    run_android(app);
    std::process::exit(0);
}