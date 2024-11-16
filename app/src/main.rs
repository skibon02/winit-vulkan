use log::LevelFilter;
use simple_logger::SimpleLogger;
use framework::run;

fn main() {
    SimpleLogger::new().with_utc_timestamps().with_colors(true).with_level(LevelFilter::Info).init().unwrap();
    run();
}