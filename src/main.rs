use anyhow::Context;
use log::{info};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};


pub mod helpers;

pub mod vulkan_backend;
use vulkan_backend::VulkanBackend;

pub mod resource_manager;

enum MsgToHandler {
    Event(Event<'static, ()>),
    Exit,
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    console_subscriber::init();
    info!("Hello, world!");


    //init tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("Winit hello!").build(&event_loop).unwrap();
    let main_window_id = window.id();

    let event_handler = 
    rt.block_on(async {
        let mut app = VulkanBackend::new(&window).unwrap();
        app.init_swapchain().context("Swapchain initialization").unwrap();

        rt.spawn(async move {
            loop {
                let event = event_rx.recv().await.unwrap();
                // println!("Received event: {:?}", event);
                // println!("On thread {:?}", std::thread::current().id());

                if let MsgToHandler::Event(event) = event {
                    match event {
                        Event::WindowEvent {
                            event: WindowEvent::Resized(size),
                            window_id,
                        } if window_id == main_window_id => {
                            println!("Resized to {:?}", size);
                        }
                        Event::MainEventsCleared => {
                            println!("Main events cleared");
                            app.render().unwrap();
                        }
                        Event::RedrawRequested(window_id) if window_id == main_window_id => {
                            println!("Redraw requested");
                            // app.render();
                        }
                        _ => (),
                    }
                }
                else {
                    break;
                }

            }
            info!("Exiting event loop...");
        })
    });


    let mut event_handler = Some(event_handler);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match &event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if *window_id == main_window_id => {

                event_tx.send(MsgToHandler::Exit).unwrap();
                if let Some(event_handler) = event_handler.take() {
                    rt.block_on(async move {
                        event_handler.await.unwrap();
                    });
                }
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }

        if let Some(evt) = event.to_static() {
            if event_tx.send(MsgToHandler::Event(evt)).is_err() {
                return
            }
        }
    });


}
