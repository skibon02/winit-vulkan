pub mod app;
pub mod vulkan_backend;
pub mod util;
pub mod renderer;

#[cfg(target_os = "android")]
mod android;

use std::fs;
use log::{error, info, warn};
use sparkles_macro::{instant_event, range_event_start};
use std::time::Instant;
use sparkles::FinalizeGuard;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoopBuilder};
use winit::keyboard::NamedKey;
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};
use winit::{event::WindowEvent, event_loop::EventLoop, keyboard};
use crate::vulkan_backend::VulkanBackend;
use crate::app::AppTrait;

#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
pub fn run_android<A: AppTrait>(app: AndroidApp) {
    let event_loop = android::android_main(app);
    let mut winit_app: WinitApp<A> = WinitApp::new();
    event_loop.run_app(&mut winit_app).unwrap();
}

#[cfg(not(target_os = "android"))]
pub fn run<A: AppTrait>() {
    let event_loop = EventLoop::new().unwrap();
    let mut winit_app: WinitApp<A> = WinitApp::new();
    event_loop.run_app(&mut winit_app).unwrap();
}

struct WinitApp<A> {
    app_state: Option<AppState<A>>,
    g: FinalizeGuard,
}

impl<A: AppTrait> WinitApp<A> {
    fn new() -> Self {
        let g = sparkles::init_default();
        Self { app_state: None, g }
    }
}

impl<A: AppTrait> ApplicationHandler for WinitApp<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let g = range_event_start!("[WINIT] resumed");
        info!("\t\t*** APP RESUMED ***");
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("DAMNDASHIE"))
            .unwrap();

        let app_state = AppState::new_winit(window);
        self.app_state = Some(app_state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let g = range_event_start!("[WINIT] window event");
        if self.app_state.as_mut().unwrap().is_finished() {
            info!("Exit requested!");
            event_loop.exit();
        }
        if let Err(e) = self.app_state.as_mut().unwrap().handle_event(event_loop, event) {
            error!("Error handling event: {:?}", e);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let g = range_event_start!("[WINIT] Exiting");
        info!("\t\t*** APP EXITING ***");
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

pub struct AppState<A> {
    user_app: A,

    app_finished: bool,
    prev_touch_event_time: Instant,

    vulkan_backend: VulkanBackend,
    window: Window,

    frame_cnt: i32,
    last_sec: Instant,

    rendering_active: bool,
}

pub enum AppResult {
    Idle,
    Exit,
}

impl<A: AppTrait> AppState<A> {
    pub fn new_winit(window: Window) -> AppState<A> {
        let user_app = A::new();

        let vulkan_backend = VulkanBackend::new_for_window(&window, &user_app).unwrap();

        Self {
            user_app,

            app_finished: false,
            prev_touch_event_time: Instant::now(),

            vulkan_backend,
            window,

            last_sec: Instant::now(),
            frame_cnt: 0,

            rendering_active: true,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.app_finished
    }

    pub fn handle_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        evt: WindowEvent,
    ) -> anyhow::Result<()> {
        match &evt {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        logical_key: keyboard::Key::Named(NamedKey::GoBack | NamedKey::BrowserBack),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                let g = range_event_start!("[APP] Close requested");
                info!("Close requested...");
                self.vulkan_backend.wait_idle();
                self.app_finished = true;
            }

            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        logical_key: keyboard::Key::Named(NamedKey::F11),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if self.window.fullscreen().is_none() {
                    let g = range_event_start!("[APP] Enable fullscreen");
                    let monitor = self.window.current_monitor().unwrap();
                    // find max by width and refresh rate
                    let mode = monitor
                        .video_modes()
                        .max_by_key(|m| m.refresh_rate_millihertz() * m.size().width)
                        .unwrap();
                    info!("Entering fullscreen mode {:?}", mode);
                    self.window
                        .set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                } else {
                    let g = range_event_start!("[APP] Exit fullscreen mode");
                    self.window.set_fullscreen(None);
                }
            }

            WindowEvent::Touch(t) => {
                let g = range_event_start!("[APP] Touch event");
                info!("Touch event: {:?}", t);
                let now = Instant::now();
                let prev = self.prev_touch_event_time;
                let elapsed = now.duration_since(prev);
                self.prev_touch_event_time = now;
                info!("Elapsed: {:?}", elapsed);
            }

            WindowEvent::RedrawRequested => {
                let g = range_event_start!("[APP] Redraw requested");
                if !self.app_finished && self.rendering_active {
                    self.vulkan_backend.render(&mut self.user_app)?;

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
                info!("Resized to {}x{}", size.width, size.height);
                if size.width == 0 || size.height == 0 {
                    warn!("One of dimensions is 0! Suspending rendering...");
                    self.rendering_active = false;
                } else {
                    if !self.rendering_active {
                        info!("Continue rendering...");
                    }
                    self.vulkan_backend.recreate_resize(*size);
                    self.rendering_active = true;
                }
            }
            // _ => info!("new window event: {:?}", evt),
            _ => {}
        }

        Ok(())
    }
}
