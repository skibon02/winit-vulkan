use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, RwLock};
use jni::{JNIEnv, JavaVM};
use jni::objects::GlobalRef;
use log::info;
use sparkles_macro::range_event_start;
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::platform::android::activity::*;
pub(crate) fn android_main(app: AndroidApp) -> EventLoop<()> {
    use jni::objects::{JObject, JObjectArray, JValue};
    use jni::JavaVM;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let g = range_event_start!("android_main init");


    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _) }.unwrap();
    let mut env = vm.get_env().unwrap();

    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
    let activity = env.new_global_ref(activity).unwrap();


    let windowmanager = env
        .call_method(
            &activity,
            "getWindowManager",
            "()Landroid/view/WindowManager;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let display = env
        .call_method(
            &windowmanager,
            "getDefaultDisplay",
            "()Landroid/view/Display;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let supported_modes = env
        .call_method(
            &display,
            "getSupportedModes",
            "()[Landroid/view/Display$Mode;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let supported_modes = JObjectArray::from(supported_modes);
    let length = env.get_array_length(&supported_modes).unwrap();
    info!("Found {} supported modes", length);
    let mut modes = Vec::new();
    for i in 0..length {
        let mode = env.get_object_array_element(&supported_modes, i).unwrap();
        let height = env
            .call_method(&mode, "getPhysicalHeight", "()I", &[])
            .unwrap()
            .i()
            .unwrap();
        let width = env
            .call_method(&mode, "getPhysicalWidth", "()I", &[])
            .unwrap()
            .i()
            .unwrap();
        let refresh_rate = env
            .call_method(&mode, "getRefreshRate", "()F", &[])
            .unwrap()
            .f()
            .unwrap();
        let index = env
            .call_method(&mode, "getModeId", "()I", &[])
            .unwrap()
            .i()
            .unwrap();
        modes.push((index, refresh_rate));
        info!("Mode {}: {}x{}@{}", index, width, height, refresh_rate);
    }

    let mut max_framerate_mode = modes
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    info!("Max framerate: {}", max_framerate_mode.1);

    let preferred_id = 1;

    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .unwrap()
        .l()
        .unwrap();

    let layout_params_class = env
        .find_class("android/view/WindowManager$LayoutParams")
        .unwrap();
    let layout_params = env
        .call_method(
            window,
            "getAttributes",
            "()Landroid/view/WindowManager$LayoutParams;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();

    let preferred_display_mode_id_field_id = env
        .get_field_id(layout_params_class, "preferredDisplayModeId", "I")
        .unwrap();
    env.set_field_unchecked(
        &layout_params,
        preferred_display_mode_id_field_id,
        JValue::from(preferred_id),
    )
        .unwrap();

    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .unwrap()
        .l()
        .unwrap();
    env.call_method(
        window,
        "setAttributes",
        "(Landroid/view/WindowManager$LayoutParams;)V",
        &[(&layout_params).into()],
    )
        .unwrap();

    drop(g);

    *VM.lock().unwrap() = Some(vm);
    *ACTIVITY.lock().unwrap() = Some(activity);
    let event_loop = EventLoopBuilder::default().with_android_app(app).build().unwrap();
    event_loop
}

pub static VM: Mutex<Option<JavaVM>> = Mutex::new(None);
pub static ACTIVITY: Mutex<Option<GlobalRef>> = Mutex::new(None);