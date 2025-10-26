use std::sync::atomic::AtomicBool;

#[cfg(target_os = "android")]
pub mod android;

pub mod winit;
pub mod scene;

static FIRST_RUN: AtomicBool = AtomicBool::new(true);
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: crate::winit::AndroidApp) {
    use crate::winit::run_android;
    if !FIRST_RUN.swap(false, std::sync::atomic::Ordering::SeqCst) {
        std::process::exit(0);
    }
    run_android(app);
    std::process::exit(0);
}