pub mod vulkan_backend;
pub mod app;

use std::time::Instant;
use log::{error, info};
use sparkles_macro::{instant_event, range_event_start};
use winit::{event::WindowEvent, event_loop::EventLoop, keyboard};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoopBuilder};
use winit::keyboard::NamedKey;
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

#[cfg(target_os = "android")]
use winit::platform::android::activity::*;
use crate::vulkan_backend::VulkanBackend;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use jni::JavaVM;
    use jni::objects::{JObject, JObjectArray, JValue};
    use winit::platform::android::EventLoopBuilderExtAndroid;
    
    let g = range_event_start!("android_main init");

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _) }.unwrap();
    let mut env = vm.get_env().unwrap();

    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };

    let windowmanager = env.call_method(&activity, "getWindowManager", "()Landroid/view/WindowManager;", &[]).unwrap().l().unwrap();
    let display = env.call_method(&windowmanager, "getDefaultDisplay", "()Landroid/view/Display;", &[]).unwrap().l().unwrap();
    let supported_modes = env.call_method(&display, "getSupportedModes", "()[Landroid/view/Display$Mode;", &[]).unwrap().l().unwrap();
    let supported_modes = JObjectArray::from(supported_modes);
    let length = env.get_array_length(&supported_modes).unwrap();
    info!("Found {} supported modes", length);
    let mut modes = Vec::new();
    for i in 0..length {
        let mode = env.get_object_array_element(&supported_modes, i).unwrap();
        let height = env.call_method(&mode, "getPhysicalHeight", "()I", &[]).unwrap().i().unwrap();
        let width = env.call_method(&mode, "getPhysicalWidth", "()I", &[]).unwrap().i().unwrap();
        let refresh_rate = env.call_method(&mode, "getRefreshRate", "()F", &[]).unwrap().f().unwrap();
        let index = env.call_method(&mode, "getModeId", "()I", &[]).unwrap().i().unwrap();
        modes.push((index, refresh_rate));
        info!("Mode {}: {}x{}@{}", index, width, height, refresh_rate);
    }

    let mut max_framerate_mode = modes.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    info!("Max framerate: {}", max_framerate_mode.1);

    let preferred_id = 1;

    let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l().unwrap();

    let layout_params_class = env.find_class("android/view/WindowManager$LayoutParams").unwrap();
    let layout_params = env.call_method(window, "getAttributes", "()Landroid/view/WindowManager$LayoutParams;", &[]).unwrap().l().unwrap();

    let preferred_display_mode_id_field_id = env.get_field_id(layout_params_class, "preferredDisplayModeId", "I").unwrap();
    env.set_field_unchecked(&layout_params, preferred_display_mode_id_field_id, JValue::from(preferred_id)).unwrap();

    let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l().unwrap();
    env.call_method(window, "setAttributes", "(Landroid/view/WindowManager$LayoutParams;)V", &[(&layout_params).into()]).unwrap();


    let event_loop = EventLoopBuilder::default().with_android_app(app).build().unwrap();
    drop(g);
    run(event_loop);
}

pub fn run(event_loop: EventLoop<()>) {
    let mut winit_app = WinitApp::new();
    event_loop.run_app(&mut winit_app).unwrap();
}

struct WinitApp {
    app: Option<App>
}

impl WinitApp {
    fn new() -> Self {
        Self {
            app: None
        }
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let g = range_event_start!("[WINIT] resumed");
        info!("\t\t*** APP RESUMED ***");
        let window = event_loop.create_window(WindowAttributes::default().with_title("Crazy triangle")).unwrap();

        let app = App::new_winit(window);
        self.app = Some(app);
    }


    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let g = range_event_start!("[WINIT] window event");
        if self.app.as_mut().unwrap().is_finished() {
            info!("Exit requested!");
            event_loop.exit();
        }
        if let Err(e) = self.app.as_mut().unwrap().handle_event(event_loop, event) {
            error!("Error handling event: {:?}", e);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let g = range_event_start!("[WINIT] Exiting");
        info!("\t\t*** APP EXITING ***");
        sparkles::finalize();
    }
    //
    // fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    //     info!("\t\t*** APP ABOUT TO WAIT ***");
    // }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let g = range_event_start!("[WINIT] Memory warning");
        info!("\t\t*** APP MEMORY WARNING ***");

    }
}


pub struct App {
    app_finished: bool,
    prev_touch_event_time: Instant,

    vulkan_backend: VulkanBackend,
    window: Window,

    frame_cnt: i32,
    last_sec: Instant,
}

pub enum AppResult {
    Idle,
    Exit
}

impl App {
    pub fn new_winit(window: Window) -> App {

        let vulkan_backend = VulkanBackend::new_for_window(&window, app::App::new()).unwrap();

        Self {
            app_finished: false,
            prev_touch_event_time: Instant::now(),

            vulkan_backend,
            window,

            last_sec: Instant::now(),
            frame_cnt: 0
        }
    }

    pub fn is_finished(&self) -> bool {
        self.app_finished
    }

    pub fn handle_event(&mut self, _event_loop: &ActiveEventLoop, evt: WindowEvent) -> anyhow::Result<()> {
        match &evt {
            WindowEvent::CloseRequested |
            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::GoBack | NamedKey::BrowserBack),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
                ..
            }=> {
                let g = range_event_start!("[APP] Close requested");
                info!("Close requested...");
                self.vulkan_backend.wait_idle();
                self.app_finished = true;
            },

            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::F11),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
                ..
            }=> {
                if self.window.fullscreen().is_none() {
                    let g = range_event_start!("[APP] Enable fullscreen");
                    let monitor = self.window.current_monitor().unwrap();
                    // find max by width and refresh rate
                    let mode = monitor.video_modes().max_by_key(|m| m.refresh_rate_millihertz() * m.size().width).unwrap();
                    info!("Entering fullscreen mode {:?}", mode);
                    self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                }
                else {
                    let g = range_event_start!("[APP] Exit fullscreen mode");
                    self.window.set_fullscreen(None);
                }
            },

            WindowEvent::Touch(t) => {
                let g = range_event_start!("[APP] Touch event");
                info!("Touch event: {:?}", t);
                let now = Instant::now();
                let prev = self.prev_touch_event_time;
                let elapsed = now.duration_since(prev);
                self.prev_touch_event_time = now;
                info!("Elapsed: {:?}", elapsed);
            },

            WindowEvent::RedrawRequested => {
                let g = range_event_start!("[APP] Redraw requested");
                if !self.app_finished {
                    self.vulkan_backend.render()?;

                    self.frame_cnt += 1;
                    if self.last_sec.elapsed().as_secs() >= 1 {
                        instant_event!("[APP] New sec!");
                        sparkles::flush_thread_local();

                        info!("FPS: {}", self.frame_cnt);
                        self.frame_cnt = 0;
                        self.last_sec = Instant::now();
                    }
                    let g = range_event_start!("[APP] window.request_redraw call");
                    self.window.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                self.vulkan_backend.recreate_resize(*size);
            }
            // _ => info!("new window event: {:?}", evt),
            _ => {}
        }

        Ok(())
    }

}