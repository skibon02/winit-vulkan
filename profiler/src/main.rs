pub mod gui_app;

use std::{env, mem, thread};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use drop_guard::guard;
use log::{info, warn, LevelFilter};
use sparkles_parser::packet_decoder::PacketDecoder;
use sparkles_parser::parsed::ParsedEvent;
use simple_logger::SimpleLogger;
use crate::gui_app::{BufferedHistogram, FrameTimeApp, FrameTimeSample};
pub fn run_discovery_task() {
    let mut running_clients = BTreeMap::new();
    loop {
        thread::sleep(Duration::from_secs(3));

        match sparkles_parser::discover_local_udp_clients() {
            Ok(r) => {
                for (session_id, addrs) in r {
                    if !running_clients.contains_key(&session_id) {
                        // try to connect from a different process (because of GUI limitation)
                        let path = env::args().nth(0).unwrap();
                        let proc = std::process::Command::new(path).arg(addrs[0].to_string()).spawn().unwrap();
                        running_clients.insert(session_id, proc);
                    }
                }
            }
            Err(e) => {
                warn!("Error discovering clients: {:?}", e);
            }
        }

        // check how our childs are doing
        let sessions: Vec<_> = running_clients.keys().cloned().collect();
        for session in sessions {
            if let Ok(Some(status)) = running_clients.get_mut(&session).unwrap().try_wait() {
                info!("Child finished with status {:?}", status);
                running_clients.remove(&session);
            }
        }
    }
}
fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).with_module_level("sparkles_parser", LevelFilter::Warn).init().unwrap();
    
    match env::args().nth(1) {
        Some(addr) => {
            let addr = SocketAddr::from_str(&addr).unwrap();
            
            let histogram = Arc::new(Mutex::new(BufferedHistogram::new())); // Start with empty histogram
            let hist_clone = Arc::clone(&histogram);

            let disconnected = Arc::new(AtomicBool::new(false));
            let disconnected_c = disconnected.clone();

            thread::spawn(move || {
                let decoder = PacketDecoder::from_socket(addr);
                let mut sparkles_parser = sparkles_parser::SparklesParser::new();

                let g = guard((), |()| {
                    disconnected.store(true, Ordering::Relaxed);
                });

                let mut stored_samples = Vec::new();
                sparkles_parser.parse_to_end(decoder, |event, thread_info| {
                    match event {
                        ParsedEvent::Range {
                            start,
                            end,
                            name
                        } => {
                            if name.contains("Vulkan") && !name.contains("render") {
                                let dur = *end - *start;
                                let cur_sample = FrameTimeSample {
                                    inner: Vec::new(),
                                    start: *start,
                                    dur,
                                    name: Arc::from(name.clone().deref())
                                };
                                stored_samples.push(cur_sample);
                            } else if name.contains("render") {
                                let dur = *end - *start;
                                let cur_sample = FrameTimeSample {
                                    inner: mem::take(&mut stored_samples),
                                    start: *start,
                                    dur,
                                    name: Arc::from(name.clone().deref())
                                };
                                let mut hist = hist_clone.lock().unwrap();
                                hist.push(cur_sample);
                            } else if name.deref() == "[sparkles] Flushing local storage" {
                                let mut hist = hist_clone.lock().unwrap();
                                hist.add_overhead(*start as f64, (*end - *start) as f64);
                            }
                        }
                        ParsedEvent::Instant {
                            tm,
                            name,
                        } if name.deref() == "dense_event" => {
                            let mut hist = hist_clone.lock().unwrap();
                            hist.dense_event(*tm);
                        }
                        _ => {}
                    }
                }).unwrap();
            });

            let mut options = eframe::NativeOptions::default();
            options.centered = false;
            options.multisampling = 8;
            eframe::run_native(
                &addr.to_string(),
                options,
                Box::new(|_cc| Ok(Box::new(FrameTimeApp {
                    histogram,
                    disconnected: disconnected_c
                }))),
            ).unwrap();
            
        }
        None => {
            run_discovery_task();
        }
    }
}
