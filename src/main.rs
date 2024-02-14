use winit::event_loop::EventLoop;
use winit_vulkan::run;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    console_subscriber::init();

    let event_loop = EventLoop::new().unwrap();
    run(event_loop);
}