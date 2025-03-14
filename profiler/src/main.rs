use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use colors_transform::{Color, Hsl};
use drop_guard::guard;
use egui::Color32;
use egui_plot::{Plot, BarChart, Bar, PlotItem};
use log::{info, warn, LevelFilter};
use ringbuf::consumer::Consumer;
use ringbuf::LocalRb;
use ringbuf::producer::Producer;
use ringbuf::storage::Heap;
use ringbuf::traits::Observer;
use sha2::Digest;
use sparkles_parser::packet_decoder::PacketDecoder;
use sparkles_parser::parsed::ParsedEvent;
use simple_logger::SimpleLogger;

struct BufferedHistogram {
    data: BTreeMap<Arc<str>, LocalRb<Heap<(u64, u64)>>>,
    capacity: usize,
    cur_frame: u64,
}

impl BufferedHistogram {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: BTreeMap::new(),
            cur_frame: 0,
        }
    }
    fn push(&mut self, name: Arc<str>, value: u64, cur_frame: u64) {
        let entry = self.data.entry(name).or_insert_with(|| LocalRb::new(self.capacity));
        if entry.occupied_len() == self.capacity {
            entry.try_pop();
        }
        entry.try_push((value, cur_frame)).unwrap();

        self.cur_frame = cur_frame;
    }
    
    fn get_sorted(&self) -> Vec<(Arc<str>, Vec<(u64, u64)>)> {
        self.data.iter().map(|v| {
            let data = v.1.as_slices();
            let mut res = Vec::with_capacity(data.0.len() + data.1.len());
            res.extend_from_slice(data.0);
            res.extend_from_slice(data.1);
            res.sort();
            (v.0.clone(), res)
        }).collect()
    }

    fn get_unsorted(&self) -> Vec<(Arc<str>, Vec<(u64, u64)>)> {
        self.data.iter().map(|v| {
            let data = v.1.as_slices();
            let mut res = Vec::with_capacity(data.0.len() + data.1.len());
            res.extend_from_slice(data.0);
            res.extend_from_slice(data.1);
            (v.0.clone(), res)
        }).collect()
    }

    fn cur_frame(&self) -> usize {
        self.cur_frame as usize
    }
}

struct FrameTimeApp {
    histogram: Arc<Mutex<BufferedHistogram>>,
    // disconnected: Arc<AtomicBool>,
}

fn color_from_name(name: &str) -> (u8, u8, u8) {
    let sha = sha2::Sha256::digest(name.as_bytes());
    let h = sha[0] as f32 / 255.0 * 360.0;

    let s = 80.0;
    let l = 70.0;

    let hsl = Hsl::from(h,s,l);
    let rgb = hsl.to_rgb();
    let r = rgb.get_red();
    let g = rgb.get_green();
    let b = rgb.get_blue();

    (r as u8, g as u8, b as u8)
}

impl eframe::App for FrameTimeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // if self.disconnected.load(Ordering::Relaxed) {
        //     
        // }
        
        let sorted_data = self.histogram.lock().unwrap().get_sorted();
        let unsorted_data = self.histogram.lock().unwrap().get_unsorted();
        let cur_frame = self.histogram.lock().unwrap().cur_frame();

        let max_len = sorted_data.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
        let mut offsets = vec![0.0; max_len];

        let sorted_charts: Vec<_> = sorted_data.iter().filter_map(|(name, samples)| {
            let i_offset = max_len - samples.len();
            if i_offset > 0 {
                return None;
            }

            // generate color based on name
            let (r, g, b) = color_from_name(name);

            let max_start = samples.iter().map(|(_, start)| *start).max().unwrap();
            let min_start = samples.iter().map(|(_, start)| *start).min().unwrap();
            let bars = samples.iter().enumerate()
                .map(|(i, (dur, start))| {
                    let alpha = ((start - min_start) as f64 / (max_start - min_start) as f64 * 250.0) as u8;
                    let i = i + i_offset;
                    let res = Bar::new(i as f64, *dur as f64 / 1000.0)
                        .base_offset(offsets[i])
                        .fill(Color32::from_rgba_unmultiplied(r, g, b, alpha));
                    offsets[i] += *dur as f64 / 1000.0;

                    res
                })
                .take(max_len / 100 * 99)
                .collect();

            let name_clone = name.clone();
            Some(BarChart::new(bars)
                // show name
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name_clone, b.value)
                }))
                .name(name))
        }).collect();

        let mut offsets = vec![0.0; max_len];
        let unsorted_charts: Vec<_> = unsorted_data.iter().filter_map(|(name, samples)| {
            let (r, g, b) = color_from_name(name);

            let min_i = cur_frame.saturating_sub(max_len.saturating_sub(1)); // current buffer is from min_i to cur_frame
            let bars = samples.iter()
                .filter(|(_, i)| *i as usize >= min_i)
                .map(|(dur, i)| {
                    let offsets_i = *i as usize - min_i;

                    let i = *i as usize;
                    let alpha = 55 + (offsets_i as f64 / max_len as f64 * 200.0) as u8;
                    let res = Bar::new((i % (max_len * 2)) as f64, *dur as f64 / -1000.0 * 0.5)
                        .base_offset(offsets[offsets_i])
                        .fill(Color32::from_rgba_unmultiplied(r, g, b, alpha));
                    offsets[offsets_i] -= *dur as f64 / 1000.0 * 0.5;
                    res
                }).collect();

            let name_clone = name.clone();
            Some(BarChart::new(bars)
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name_clone, b.value * 2.0)
                }))
                .name(name))
        }).collect();

        egui::CentralPanel::default().show(ctx, |ui| {
            Plot::new("Frame Time Histogram")
                .show(ui, |plot_ui| {
                    for chart in sorted_charts {
                        plot_ui.bar_chart(chart);
                    }
                    for chart in unsorted_charts {
                        plot_ui.bar_chart(chart);
                    }
                });
        });

        ctx.request_repaint(); // Keep updating
    }
}

fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).with_module_level("sparkles_parser", LevelFilter::Warn).init().unwrap();

    // Client discovery channel
    let (new_client_tx, new_client_rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(3));
            match sparkles_parser::discover_local_udp_clients() {
                Ok(r) => {
                    for addr in r {
                        new_client_tx.send(addr).unwrap();
                    }
                }
                Err(e) => {
                    warn!("Error discovering clients: {:?}", e);
                }
            }
        }
    });

    let capacity = std::env::args().nth(2).unwrap_or("10000".to_string());
    let capacity = capacity.parse().unwrap();

    // static CONNECTED_CLIENTS: Mutex<Vec<SocketAddr>> = Mutex::new(Vec::new());
    while let Ok(addr) =  new_client_rx.recv() {
        // if CONNECTED_CLIENTS.lock().unwrap().contains(&addr) {
        //     continue;
        // }

        // CONNECTED_CLIENTS.lock().unwrap().push(addr);
        let histogram = Arc::new(Mutex::new(BufferedHistogram::new(capacity))); // Start with empty histogram
        let hist_clone = Arc::clone(&histogram);

        // let disconnected = Arc::new(AtomicBool::new(false));
        // let disconnected_c = disconnected.clone();
        thread::spawn(move || {
            let decoder = PacketDecoder::from_socket(addr.clone());
            let mut sparkles_parser = sparkles_parser::SparklesParser::new();

            // let g = guard((), |()| {
            //     let mut clients = CONNECTED_CLIENTS.lock().unwrap();
            //     let i = clients.iter().position(|a| a == &addr);
            //     if let Some(i) = i {
            //         clients.swap_remove(i);
            //     }
            //     disconnected.store(true, Ordering::Relaxed);
            // });

            let mut frame = 0;
            sparkles_parser.parse_to_end(decoder, |event, thread_info| {
                match event {
                    ParsedEvent::Range {
                        start,
                        end,
                        name
                    } => {
                        if name.contains("Vulkan") && !name.contains("render") {
                            let dur = *end - *start;
                            let mut hist = hist_clone.lock().unwrap();
                            hist.push(Arc::from(name.deref()), dur, frame);
                        }
                        else if name.contains("render") {
                            frame += 1;
                        }
                    }
                    _ => {}
                }
            }).unwrap();
        });

        let mut options = eframe::NativeOptions::default();
        eframe::run_native(
            &addr.to_string(),
            options,
            Box::new(|_cc| Ok(Box::new(FrameTimeApp { 
                histogram, 
                // disconnected: disconnected_c
            }))),
        ).unwrap();
    }
}
