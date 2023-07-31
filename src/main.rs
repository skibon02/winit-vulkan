use std::time::Duration;

use log::{info, error};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod helpers;

pub mod app;
use app::App;

pub mod resource_manager;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    info!("Hello, world!");
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("Winit hello!").build(&event_loop).unwrap();

    let mut app = App::new(&window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                //draw here
                app.render().unwrap();
            }
            _ => (),
        }
    });
}
