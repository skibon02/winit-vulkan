use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{cmp, mem, thread};
use std::time::Duration;
use colors_transform::{Color, Hsl};
use drop_guard::guard;
use egui::{Color32, Frame};
use egui_plot::{Plot, BarChart, Bar, PlotItem};
use log::{info, warn, LevelFilter};
use ringbuf::consumer::Consumer;
use ringbuf::LocalRb;
use ringbuf::producer::Producer;
use ringbuf::storage::Heap;
use ringbuf::traits::{Observer, RingBuffer};
use sha2::Digest;
use sparkles_parser::packet_decoder::PacketDecoder;
use sparkles_parser::parsed::ParsedEvent;
use simple_logger::SimpleLogger;


#[derive(Default, Clone)]
struct FrameTimeSample {
    name: Arc<str>,
    start: u64,
    dur: u64,
    inner: Vec<FrameTimeSample>
}

impl PartialEq for FrameTimeSample {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl PartialOrd for FrameTimeSample {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.dur.partial_cmp(&other.dur)
    }
}

impl Eq for FrameTimeSample {}
impl Ord for FrameTimeSample {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.dur.cmp(&other.dur)
    }
}
struct BufferedHistogram {
    data: LocalRb<Heap<FrameTimeSample>>,
    cur_frame: u64,
    frame_time_sum: u64,
    max_start: u64,
}

const TIME_BUFFER_S: f64 = 2.5;
impl BufferedHistogram {
    pub fn new() -> Self {
        Self {
            data: LocalRb::new(10_000),
            cur_frame: 0,
            frame_time_sum: 0,
            max_start: 0,
        }
    }
    fn push(&mut self, sample: FrameTimeSample) {
        self.frame_time_sum += sample.dur;
        self.max_start = sample.start;
        self.cur_frame += 1;
        if let Some(removed) = self.data.push_overwrite(sample) {
            self.frame_time_sum -= removed.dur;
        }

        self.cleanup_if_needed();
    }

    fn cleanup_if_needed(&mut self) {
        let mut cnt = 0;
        
        for sample in self.data.iter_mut() {
            if sample.start as f64 + TIME_BUFFER_S * 1_000_000_000.0 < self.max_start as f64 {
                cnt += 1;
            }
            else {
                break;
            }
        }
        
        for _ in 0..cnt {
            if let Some(removed) = self.data.try_pop() {
                self.frame_time_sum -= removed.dur;
            }
        }
    }

    fn get_unsorted(&self) -> Vec<FrameTimeSample> {
        let data = self.data.as_slices();
        let mut res = Vec::with_capacity(data.0.len() + data.1.len());
        res.extend_from_slice(data.0);
        res.extend_from_slice(data.1);
        res
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
        let mut unsorted_data = self.histogram.lock().unwrap().get_unsorted();
        
        let unsorted_height_scale = 0.5;
        let max_i = unsorted_data.len();
        let max_start = unsorted_data.iter().map(|s| s.start).max().unwrap_or(0);
        let mut unsorted_bars = BTreeMap::new();
        unsorted_data.iter().enumerate().for_each(|(i, sample)| {
            let cur_pos = i as f64 / max_i as f64 * 100.0;
            let freshness = (max_start - sample.start) as f64 / 1000_000_000.0;
            // interpolate alpha 0.4 to 1.0 for freshness 1.0 to 0.0
            let alpha = 1.0 - (freshness % 1.0 * 0.8);

            for inner in &sample.inner {
                let name = inner.name.clone();
                let (r, g, b) = color_from_name(&name);
                let bars: &mut Vec<_> = unsorted_bars.entry(name).or_default();
                let bar_height = inner.dur as f64 / 1000.0 * unsorted_height_scale * -1.0;
                let bar_start = (inner.start - sample.start) as f64 / 1000.0 * unsorted_height_scale * -1.0;
                let bar = Bar::new(cur_pos, bar_height)
                    .base_offset(bar_start)
                    .width(1.0 / max_i as f64 * 100.0)
                    .fill(Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0) as u8));
                bars.push(bar);
            }
        });
        
        unsorted_data.sort();
        
        let mut sorted_bars = BTreeMap::new();
        // now sorted
        unsorted_data.iter().enumerate().for_each(|(i, sample)| {
            let cur_pos = i as f64 / max_i as f64 * 100.0;
            let freshness = (max_start - sample.start) as f64 / 1000_000_000.0;
            
            // interpolate alpha 0.4 to 1.0 for freshness 1.0 to 0.0
            let alpha = 1.0 - (freshness / TIME_BUFFER_S * 0.9);

            for inner in &sample.inner {
                let name = inner.name.clone();
                let (r, g, b) = color_from_name(&name);
                let bars: &mut Vec<_> = sorted_bars.entry(name).or_default();
                let bar_height = inner.dur as f64 / 1000.0;
                let bar_start = (inner.start - sample.start) as f64 / 1000.0;
                let bar = Bar::new(cur_pos, bar_height)
                    .base_offset(bar_start)
                    .width(1.0 / max_i as f64 * 100.0)
                    .fill(Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0) as u8));
                bars.push(bar);
            }
        });
        
        let unsorted_charts = unsorted_bars.into_iter().map(|(name, bars)| {
            BarChart::new(bars)
                .name(&name)
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name, b.value / unsorted_height_scale)
                }))
        }).collect::<Vec<_>>();
        
        let sorted_charts = sorted_bars.into_iter().map(|(name, bars)| {
            BarChart::new(bars)
                .name(&name)
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name, b.value)
                }))
        }).collect::<Vec<_>>();

        egui::CentralPanel::default().show(ctx, |ui| {
            Plot::new("Frame Time Histogram")
                .show(ui, |plot_ui| {
                    for chart in unsorted_charts {
                        plot_ui.bar_chart(chart);
                    }
                    for chart in sorted_charts {
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

    // static CONNECTED_CLIENTS: Mutex<Vec<SocketAddr>> = Mutex::new(Vec::new());
    while let Ok(addr) =  new_client_rx.recv() {
        // if CONNECTED_CLIENTS.lock().unwrap().contains(&addr) {
        //     continue;
        // }

        // CONNECTED_CLIENTS.lock().unwrap().push(addr);
        let histogram = Arc::new(Mutex::new(BufferedHistogram::new())); // Start with empty histogram
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
                        }
                        else if name.contains("render") {
                            let dur = *end - *start;
                            let cur_sample = FrameTimeSample {
                                inner: mem::take(&mut stored_samples),
                                start: *start,
                                dur,
                                name: Arc::from(name.clone().deref())
                            };
                            let mut hist = hist_clone.lock().unwrap();
                            hist.push(cur_sample);
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
