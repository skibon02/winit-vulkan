use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use anyhow::Context;
use log::{error, info};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop},
    window::WindowBuilder,
};
use winit::window::WindowId;


pub mod helpers;

pub mod vulkan_backend;
use vulkan_backend::VulkanBackend;

pub mod resource_manager;


fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    console_subscriber::init();


    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().with_title("Winit hello!").build(&event_loop).unwrap();
    let main_window_id = window.id();

    let mut app = App::new_winit(window, main_window_id);

    event_loop.run(move |event, elwt| {
        match &event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if *window_id == main_window_id => {
                elwt.exit();

                app.wait_exit().unwrap();
            }
            _ => (),
        }

        if let Err(e) = app.handle_event(event) {
            error!("Error handling event: {:?}", e);
            return
        }

    }).unwrap();


}
pub struct App<E>
where E: 'static + Clone + Send + Debug{
    jh: Option<thread::JoinHandle<()>>,
    is_exiting: Arc<AtomicBool>,
    event_sender: Sender<Event<E>>
}

impl<E> App<E>
where E: Clone + Send + 'static + Debug {
    pub fn new_winit(window: winit::window::Window, main_window_id: WindowId) -> App<E> {

        let is_exiting = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();

        let is_exiting_clone = is_exiting.clone();

        let jh = thread::spawn(move || {
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
                            break;
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
                    info!("[app] exit requested");
                    break;
                }
            }
        });

        Self {
            jh: Some(jh),
            is_exiting: is_exiting_clone,
            event_sender: tx
        }
    }

    pub fn run(&self) {

    }

    pub fn wait_exit(&mut self) -> anyhow::Result<()> {
        info!("Waiting for exit");
        self.is_exiting.store(true, Ordering::Relaxed);

        self.jh.take().unwrap().join().unwrap();

        Ok(())
    }

    // should not be called
    pub fn handle_event(&mut self, evt: Event<E>) -> anyhow::Result<()> {
        info!("handling event...");
        self.event_sender.send(evt).unwrap();

        Ok(())
    }
}