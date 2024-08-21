pub mod vulkan_backend;

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use anyhow::Context;
use jni::JavaVM;
use jni::objects::{JObject, JObjectArray, JValue};
use log::{error, info, warn};
use winit::{event::{Event, WindowEvent}, event_loop, event_loop::EventLoop, keyboard};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId};
use winit::event_loop::{ActiveEventLoop, EventLoopBuilder};
use winit::keyboard::NamedKey;
use winit::window::{Window, WindowAttributes, WindowId};

#[cfg(target_os = "android")]
use winit::platform::android::activity::*;
use crate::vulkan_backend::VulkanBackend;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

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
        info!("\t\t*** APP RESUMED ***");
        let window = event_loop.create_window(WindowAttributes::default().with_title("Winit hello!")).unwrap();

        let mut app = App::new_winit(window);
        app.send_resumed();
        self.app = Some(app);
    }


    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if self.app.as_mut().unwrap().is_finished() {
            info!("Exit requested!");
            event_loop.exit();
        }
        if let Err(e) = self.app.as_mut().unwrap().handle_event(event_loop, event) {
            error!("Error handling event: {:?}", e);
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        info!("\t\t*** APP EXITING ***");
    }
    //
    // fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    //     info!("\t\t*** APP ABOUT TO WAIT ***");
    // }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        info!("\t\t*** APP MEMORY WARNING ***");

    }
}


pub struct App {
    jh: Option<thread::JoinHandle<()>>,
    is_exiting: Arc<AtomicBool>,
    event_sender: Sender<RendererMessage>,
    app_finished: bool,
    prev_touch_event_time: Instant,
}

pub enum AppResult {
    Idle,
    Exit
}

#[derive(Debug)]
enum RendererMessage {
    Resumed,
    RedrawRequested,
    Exiting
}

impl App {
    pub fn new_winit(window: Window) -> App {

        let is_exiting = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();

        let is_exiting_clone = is_exiting.clone();

        let mut vulkan_backend = VulkanBackend::new(&window).unwrap();
        let jh = thread::Builder::new().name("vulkan_thread".to_string()).spawn(move || {
            info!("Thread started!");
            #[cfg(target_os = "android")]
            {
                info!("Waiting for Resumed android event...");
                loop {
                    let msg = rx.recv().unwrap();
                    match msg {
                        RendererMessage::Resumed => {
                            info!("Received RESUMED signal!");
                            break;
                        }
                        _ => {}
                    }
                }
            }

            let mut frame_cnt = 0;
            let mut last_sec = Instant::now();
            loop {
                let message = rx.recv().unwrap();
                // info!("Received message: {:?}", message);
                // println!("On thread {:?}", std::thread::current().id());

                match message {
                    RendererMessage::RedrawRequested => {
                        vulkan_backend.render().unwrap();

                        frame_cnt += 1;
                        if last_sec.elapsed().as_secs() >= 1 {
                            info!("FPS: {frame_cnt}");
                            frame_cnt = 0;
                            last_sec = Instant::now();
                        }
                        window.request_redraw();
                    }
                    _ => {

                    }
                }

                if is_exiting.load(Ordering::Relaxed) {
                    info!("[app] exit requested...");
                    // thread::sleep(Duration::from_secs(1));
                    break;
                }
            }

            vulkan_backend.wait_idle();
        }).unwrap();

        Self {
            jh: Some(jh),
            is_exiting: is_exiting_clone,
            event_sender: tx,
            app_finished: false,
            prev_touch_event_time: Instant::now(),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.app_finished
    }

    pub fn handle_event(&mut self, event_loop: &ActiveEventLoop, evt: WindowEvent) -> anyhow::Result<()> {
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
                info!("Close requested...");
                self.is_exiting.store(true, Ordering::Relaxed);
                self.event_sender.send(RendererMessage::Exiting).unwrap();
                self.jh.take().unwrap().join().unwrap();
                info!("Main thread joined!");
                self.app_finished = true;
            },

            WindowEvent::Touch(t) => {
                info!("Touch event: {:?}", t);
                let now = Instant::now();
                let prev = self.prev_touch_event_time;
                let elapsed = now.duration_since(prev);
                self.prev_touch_event_time = now;
                info!("Elapsed: {:?}", elapsed);
            },

            WindowEvent::RedrawRequested => {
                if !self.app_finished {
                    self.event_sender.send(RendererMessage::RedrawRequested).unwrap();
                }
            }

            _ => info!("new window event: {:?}", evt),
        }

        Ok(())
    }

    pub fn send_resumed(&mut self) {
        self.event_sender.send(RendererMessage::Resumed).unwrap();
    }
}