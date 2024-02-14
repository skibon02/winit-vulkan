use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use anyhow::Context;
use log::{error, info, warn};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop},
    window::WindowBuilder,
};
use winit::event_loop::EventLoopBuilder;
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::window::WindowId;


pub mod helpers;

pub mod vulkan_backend;
use vulkan_backend::VulkanBackend;

pub mod resource_manager;



#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let event_loop = EventLoopBuilder::default().with_android_app(app).build().unwrap();
    run(event_loop);
}

pub fn run(event_loop: EventLoop<()>) {
    let window = WindowBuilder::new().with_title("Winit hello!").build(&event_loop).unwrap();
    let main_window_id = window.id();

    let mut app = App::new_winit(window, main_window_id);

    event_loop.run(move |event, elwt| {
        if app.is_finished() {
            info!("Exit requested!");
            elwt.exit();
        }
        match app.handle_event(event) {
            Ok(_) => {
            },
            Err(e) => {
                error!("Error handling event: {:?}", e);
            }
        }

    }).unwrap();
}


pub struct App<E>
    where E: 'static + Clone + Send + Debug{
    jh: Option<thread::JoinHandle<()>>,
    is_exiting: Arc<AtomicBool>,
    event_sender: Sender<Event<E>>,
    main_window_id: WindowId,
    app_finished: bool
}

pub enum AppResult {
    Idle,
    Exit
}

impl<E> App<E>
    where E: Clone + Send + 'static + Debug {
    pub fn new_winit(window: winit::window::Window, main_window_id: WindowId) -> App<E> {

        let is_exiting = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();

        let is_exiting_clone = is_exiting.clone();

        let jh = thread::Builder::new().name("vulkan_thread".to_string()).spawn(move || {
            info!("Thread started!");
            #[cfg(target_os = "android")]
            {
                info!("Waiting for RESUMED event...");
                loop {
                    let event = rx.recv().unwrap();
                    info!("Received event: {:?}", event);
                    match event {
                        Event::Resumed => {

                            info!("Resumed event received!");
                            break;
                        }
                        _ => (),
                    }
                }
            }
            //set thread name
            let mut app = VulkanBackend::new(&window).unwrap();
            app.init_swapchain().context("Swapchain initialization").unwrap();

            loop {
                let event = rx.recv().unwrap();
                info!("Received event: {:?}", event);
                // println!("On thread {:?}", std::thread::current().id());

                match event {
                    Event::WindowEvent {
                        event,
                        window_id,
                    } if window_id == main_window_id => match event {
                        WindowEvent::RedrawRequested => {
                            info!("Redraw requested");
                            app.render().unwrap();
                        }
                        _ => (),
                    }
                    Event::AboutToWait => {
                        info!("About to wait");
                        app.render().unwrap();
                    }
                    _ => (),
                }

                if is_exiting.load(Ordering::Relaxed) {
                    info!("[app] exit requested...");
                    thread::sleep(Duration::from_secs(1));
                    break;
                }
            }
        }).unwrap();

        Self {
            jh: Some(jh),
            is_exiting: is_exiting_clone,
            event_sender: tx,
            main_window_id,
            app_finished: false
        }
    }

    pub fn is_finished(&self) -> bool {
        self.app_finished
    }

    // should not be called
    pub fn handle_event(&mut self, evt: Event<E>) -> anyhow::Result<()> {
        match &evt {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if *window_id == self.main_window_id => {
                info!("Close requested...");
                self.is_exiting.store(true, Ordering::Relaxed);
                self.jh.take().unwrap().join().unwrap();
                info!("Main thread joined!");
                self.app_finished = true;
            }
            _ => (),
        }

        if self.event_sender.send(evt.clone()).is_err() {
            warn!("Event sender is closed! event {:?} was not delivered!", evt);
        }

        Ok(())
    }
}