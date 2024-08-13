use log::LevelFilter;
use simple_logger::SimpleLogger;
use winit::event_loop::EventLoop;
use winit_vulkan::run;

fn main() {
    SimpleLogger::new().with_utc_timestamps().with_colors(true).with_level(LevelFilter::Info).init().unwrap();
    // console_subscriber::init();
    let event_loop = EventLoop::new().unwrap();
    run(event_loop);
}