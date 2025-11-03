use std::{fs, thread};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use parking_lot::Mutex;
use log::{error, info, warn};
use std::time::Instant;
use sparkles::config::SparklesConfig;
use sparkles::{instant_event, range_event_start, FinalizeGuard};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoopBuilder};
use winit::keyboard::NamedKey;
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};
use winit::{event::WindowEvent, event_loop::EventLoop, keyboard};
use winit::event::{ElementState, MouseButton};
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;
use winit::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use render::vulkan_backend::VulkanBackend;

use render::vulkan_backend::config::{InFlightFrames, VulkanRenderConfig};
use crate::scene::circle::{CircleAttributes, CircleAttributesExt};
use crate::scene::Scene;
use crate::scene::uniforms::Time;

enum RenderMessage {
    Redraw { bg_color: [f32; 3], done: Arc<AtomicBool> },
    Resize { width: u32, height: u32 },
    Exit,
}


fn sparkles_init() -> FinalizeGuard{
    sparkles::init(SparklesConfig::default()
        .with_udp_multicast_default())
}

#[cfg(target_os = "android")]
pub fn run_android(app: AndroidApp) {
    use crate::android::android_main;
    
    let g = sparkles_init();
    let event_loop = android_main(app);
    let mut winit_app: WinitApp = WinitApp::new(g);
    event_loop.run_app(&mut winit_app).unwrap();
    info!("Winit application exited without error!");
}

#[cfg(not(target_os = "android"))]
pub fn run() {
    let g = sparkles_init();
    let event_loop = EventLoop::new().unwrap();
    let mut winit_app: WinitApp = WinitApp::new(g);
    event_loop.run_app(&mut winit_app).unwrap();
}

struct WinitApp {
    app_state: Option<AppState>,
    g: FinalizeGuard,
}

impl WinitApp {
    fn new(g: FinalizeGuard) -> Self {
        
        Self { app_state: None, g }
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let g = range_event_start!("[WINIT] resumed");
        info!("\t\t*** APP RESUMED ***");
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("shades of pink"))
            .unwrap();

        window.request_redraw();

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

pub struct AppState {
    app_finished: bool,
    prev_touch_event_time: Instant,
    start_time: Instant,

    window: Window,

    frame_cnt: i32,
    last_sec: Instant,

    rendering_active: bool,

    scene: Arc<Mutex<Scene>>,
    render_tx: mpsc::Sender<RenderMessage>,
    render_thread: Option<JoinHandle<()>>,
    render_ready: Arc<AtomicBool>,

    bg_color: [f32; 3],
    last_touch_pos: [f32; 2],
    last_frame_time: Instant,

    trail_last_update: Instant,
}

pub enum AppResult {
    Idle,
    Exit,
}

impl AppState {
    pub fn new_winit(window: Window) -> AppState {
        let raw_window_handle = window.raw_window_handle().unwrap();
        let raw_display_handle = window.raw_display_handle().unwrap();
        let inner_size = window.inner_size();
        let config = VulkanRenderConfig {
            msaa_samples: None,
            in_flight_frames: InFlightFrames::One
        };
        let mut vulkan_backend = VulkanBackend::new_for_window(
            raw_window_handle,
            raw_display_handle,
            (inner_size.width, inner_size.height),
            config
        ).unwrap();

        let aspect = inner_size.width as f32 / inner_size.height as f32;
        let scene = Arc::new(Mutex::new(Scene::new(aspect)));
        let scene_clone = Arc::clone(&scene);

        let (tx, rx) = mpsc::channel::<RenderMessage>();

        let render_thread = thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(RenderMessage::Redraw { bg_color, done }) => {
                        let g = range_event_start!("Render");
                        let mut scene_guard = scene_clone.lock();
                        if let Err(e) = vulkan_backend.render(&mut *scene_guard, bg_color) {
                            error!("Render error: {:?}", e);
                        }
                        done.store(true, Ordering::Release);
                    }
                    Ok(RenderMessage::Resize { width, height }) => {
                        let g = range_event_start!("Recreate Resize");
                        vulkan_backend.recreate_resize((width, height));
                    }
                    Ok(RenderMessage::Exit) | Err(_) => {
                        info!("Render thread exiting");
                        break;
                    }
                }
            }
        });

        Self {
            scene,
            render_tx: tx,
            render_thread: Some(render_thread),
            render_ready: Arc::new(AtomicBool::new(true)),
            app_finished: false,
            prev_touch_event_time: Instant::now(),
            window,
            last_sec: Instant::now(),
            frame_cnt: 0,
            rendering_active: true,
            start_time: Instant::now(),
            bg_color: [0.0, 0.0, 0.0],
            last_touch_pos: [0.0, 0.0],
            last_frame_time: Instant::now(),
            trail_last_update: Instant::now(),
        }
    }
    
    fn calculate_aspect(&self) -> f32 {
        let inner_size = self.window.inner_size();
        inner_size.width as f32 / inner_size.height as f32
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
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => {
                let g = range_event_start!("[APP] Close requested");
                info!("Close requested...");
                self.app_finished = true;
            }

            WindowEvent::KeyboardInput {
                event:
                winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::F11),
                    state: ElementState::Pressed,
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
                        .map(|m| (m.size().width, m.refresh_rate_millihertz(), m))
                        .max_by_key(|(w, hz, m)| w * 5000 + * hz)
                        .map(|(_, _, m)| m)
                        .unwrap();
                    info!("Entering fullscreen mode {:?}, refresh rate: {}", mode.size(), mode.refresh_rate_millihertz() as f32 / 1000.0);
                    self.window
                        .set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                } else {
                    let g = range_event_start!("[APP] Exit fullscreen mode");
                    self.window.set_fullscreen(None);
                }
            }
            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::ArrowLeft),
                    state: ElementState::Released,
                    ..
                },
                ..
            } => {
                self.scene.lock().mirror_lamp.modify_pos(|mut pos| {
                    pos[0] += 0.1;
                    pos
                });
                self.last_touch_pos[0] -= 0.1;
            }

            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::ArrowRight),
                    state: ElementState::Released,
                    ..
                },
                ..
            } => {
                self.scene.lock().mirror_lamp.modify_pos(|mut pos| {
                    pos[0] -= 0.1;
                    pos
                });
                self.last_touch_pos[0] += 0.1;
            }

            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::ArrowUp),
                    state: ElementState::Released,
                    ..
                },
                ..
            } => {
                self.scene.lock().mirror_lamp.modify_pos(|mut pos| {
                    pos[1] += 0.1;
                    pos
                });
                self.last_touch_pos[1] -= 0.1;
            }

            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    logical_key: keyboard::Key::Named(NamedKey::ArrowDown),
                    state: ElementState::Released,
                    ..
                },
                ..
            } => {
                self.scene.lock().mirror_lamp.modify_pos(|mut pos| {
                    pos[1] -= 0.1;
                    pos
                });
                self.last_touch_pos[1] += 0.1;
            }

            WindowEvent::Touch(t) => {
                let g = range_event_start!("[APP] Touch event");
                info!("Touch event: {:?}", t);
                let now = Instant::now();
                let prev = self.prev_touch_event_time;
                let elapsed = now.duration_since(prev);
                self.prev_touch_event_time = now;
                info!("Elapsed: {:?}", elapsed);

                let pos = [
                    (t.location.x as f32 / self.window.inner_size().width as f32) * 2.0 - 1.0,
                    (t.location.y as f32 / self.window.inner_size().height as f32) * 2.0 - 1.0,
                ];
                self.last_touch_pos = pos;

                let mut scene = self.scene.lock();
                scene.mirror_lamp.set_pos([-pos[0], -pos[1]]);
                scene.trail.create(self.start_time.elapsed().as_millis() as u64  + 10_000, CircleAttributes {
                    pos: pos.into(),
                    color: [1.0, 0.2, 0.4, 1.0].into(),
                    trig_time: i32::MAX.into(),
                });
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                info!("Mouse left button pressed!");
                self.last_touch_pos = [0.0, 0.0];

                let mut scene = self.scene.lock();
                scene.mirror_lamp.set_pos([0.0, 0.0]);
                scene.trail.create(self.start_time.elapsed().as_millis() as u64  + 10_000, CircleAttributes {
                    pos: [rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0)].into(),
                    color: [1.0, 0.2, 0.4, 1.0].into(),
                    trig_time: i32::MAX.into(),
                });
            }

            WindowEvent::RedrawRequested => {
                let now = self.start_time.elapsed().as_millis() as f32;
                let g = range_event_start!("[APP] Redraw requested");
                if !self.app_finished && self.rendering_active {
                    let normalized_touch_pos = [
                        (self.last_touch_pos[0] + 1.0) / 2.0,
                        (self.last_touch_pos[1] + 1.0) / 2.0,
                    ];

                    let new_color = [
                        normalized_touch_pos[0] * 0.6 + normalized_touch_pos[1] * 0.3 + (now / 600.0).sin() * 0.05,
                        normalized_touch_pos[0] * 0.3 + normalized_touch_pos[1] * 0.3 + (now / 600.0 + 1.0).sin() * 0.05,
                        normalized_touch_pos[1] * 0.6 + normalized_touch_pos[0] * 0.3 + (now / 600.0 + 2.0).sin() * 0.05,
                    ];

                    let color_dir = [
                        new_color[0] - self.bg_color[0],
                        new_color[1] - self.bg_color[1],
                        new_color[2] - self.bg_color[2],
                    ];

                    let elapsed = self.last_frame_time.elapsed().as_secs_f32();
                    let color_dist = (color_dir[0].powi(2) + color_dir[1].powi(2) + color_dir[2].powi(2)).sqrt();
                    let color_dist = (color_dist + 0.5) * elapsed * 20.0;
                    let color_change = [
                        color_dir[0] * color_dist,
                        color_dir[1] * color_dist,
                        color_dir[2] * color_dist,
                    ];

                    self.bg_color[0] += color_change[0];
                    self.bg_color[1] += color_change[1];
                    self.bg_color[2] += color_change[2];

                    if self.trail_last_update.elapsed().as_secs_f32() > 0.2 {
                        let trail_id = self.trail_last_update.duration_since(self.start_time).as_millis() as u64;

                        let mut scene = self.scene.lock();
                        scene.trail.create(trail_id, CircleAttributes {
                            pos: [self.last_touch_pos[0], self.last_touch_pos[1]].into(),
                            color: [1.0, 0.7, 1.0, 1.0].into(),
                            trig_time: (trail_id as i32 + 1_500).into(),
                        });
                        scene.trail.auto_remove(trail_id.saturating_sub(2_000));

                        self.trail_last_update = Instant::now();
                    }

                    // Only send redraw if render thread is ready (non-blocking check)
                    if self.render_ready.load(Ordering::Acquire) {
                        self.render_ready.store(false, Ordering::Release);
                        let _ = self.render_tx.send(RenderMessage::Redraw {
                            bg_color: self.bg_color,
                            done: Arc::clone(&self.render_ready),
                        });
                        self.frame_cnt += 1;
                    } else {
                        // Render thread is still busy, skip this frame
                    }
                    if self.last_sec.elapsed().as_secs() >= 1 {
                        info!("FPS: {}", self.frame_cnt);
                        self.frame_cnt = 0;
                        self.last_sec = Instant::now();
                    }
                    let g = range_event_start!("[APP] window.request_redraw call");
                    self.window.request_redraw();
                }
                self.last_frame_time = Instant::now();
            }
            WindowEvent::Resized(size) => {
                static FIRST_RESIZE: AtomicBool = AtomicBool::new(true);
                if FIRST_RESIZE.swap(false, Ordering::Relaxed) {
                    return Ok(());
                }

                info!("Resized to {}x{}", size.width, size.height);
                if size.width == 0 || size.height == 0 {
                    warn!("One of dimensions is 0! Suspending rendering...");
                    self.rendering_active = false;
                } else {
                    if !self.rendering_active {
                        info!("Continue rendering...");
                    } else {
                        let aspect = self.calculate_aspect();
                        self.scene.lock().map_stats.modify(|stats| {
                            stats.aspect = aspect.into();
                        })
                    }

                    let _ = self.render_tx.send(RenderMessage::Resize {
                        width: size.width,
                        height: size.height,
                    });

                    self.rendering_active = true;
                }
            }
            // _ => info!("new window event: {:?}", evt),
            _ => {}
        }

        Ok(())
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        info!("AppState dropping, sending exit message to render thread");
        let _ = self.render_tx.send(RenderMessage::Exit);

        if let Some(thread) = self.render_thread.take() {
            info!("Waiting for render thread to finish...");
            if let Err(e) = thread.join() {
                error!("Error joining render thread: {:?}", e);
            } else {
                info!("Render thread finished successfully");
            }
        }
    }
}
