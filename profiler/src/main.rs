use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread;
use egui_plot::{Plot, BarChart, Bar, PlotItem};
use log::{info, LevelFilter};
use ringbuf::consumer::Consumer;
use ringbuf::LocalRb;
use ringbuf::producer::Producer;
use ringbuf::storage::Heap;
use ringbuf::traits::Observer;
use sparkles_parser::packet_decoder::PacketDecoder;
use sparkles_parser::parsed::ParsedEvent;
use simple_logger::SimpleLogger;

struct BufferedHistogram {
    data: BTreeMap<Arc<str>, LocalRb<Heap<u64>>>,
    capacity: usize
}

impl BufferedHistogram {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: BTreeMap::new(),
        }
    }
    fn push(&mut self, name: Arc<str>, value: u64) {
        let entry = self.data.entry(name).or_insert_with(|| LocalRb::new(self.capacity));
        if entry.occupied_len() == self.capacity {
            entry.try_pop();
        }
        entry.try_push(value).unwrap();
    }
    
    fn get_sorted(&self) -> Vec<(Arc<str>, Vec<u64>)> {
        self.data.iter().map(|v| {
            let data = v.1.as_slices();
            let mut res = Vec::with_capacity(data.0.len() + data.1.len());
            res.extend_from_slice(data.0);
            res.extend_from_slice(data.1);
            res.sort();
            (v.0.clone(), res)
        }).collect()
    }
}

struct FrameTimeApp {
    histogram: Arc<Mutex<BufferedHistogram>>,
}

impl eframe::App for FrameTimeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let data = self.histogram.lock().unwrap().get_sorted();

        let max_len = data.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
        let mut offsets = vec![0.0; max_len];
        let charts: Vec<_> = data.iter().filter_map(|(name, samples)| {
            let i_offset = max_len - samples.len();
            if i_offset > 0 {
                return None;
            }
            let bars = samples.iter().enumerate()
                .map(|(i, &v)| {
                    let i = i + i_offset;
                    let res = Bar::new(i as f64, v as f64 / 1000.0)
                        .base_offset(offsets[i]);
                    offsets[i] += v as f64 / 1000.0;

                    res
                })
                .take(max_len / 100 * 95)
                .collect();

            let name_clone = name.clone();
            Some(BarChart::new(bars)
                // show name
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name_clone, b.value)
                }))
                .name(name))
        }).collect();

        egui::CentralPanel::default().show(ctx, |ui| {
            Plot::new("Frame Time Histogram")
                .show(ui, |plot_ui| {
                    if charts.len() > 1 {
                        for chart in charts {
                            plot_ui.bar_chart(chart);
                        }
                    }
                });
        });

        ctx.request_repaint(); // Keep updating
    }
}

fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).with_module_level("sparkles_parser", LevelFilter::Warn).init().unwrap();
        
    let mut addr = std::env::args().nth(1).unwrap_or("127.0.0.1".to_string());
    if !addr.contains(":") {
        addr.push_str(":38338");
    }

    let capacity = std::env::args().nth(2).unwrap_or("10000".to_string());
    let capacity = capacity.parse().unwrap();

    let histogram = Arc::new(Mutex::new(BufferedHistogram::new(capacity))); // Start with empty histogram
    let hist_clone = Arc::clone(&histogram);


    thread::spawn(move || {
        loop {
            let decoder = PacketDecoder::from_socket(addr.clone());
            let mut sparkles_parser = sparkles_parser::SparklesParser::new();

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
                            hist.push(Arc::from(name.deref()), dur);
                        }
                    }
                    _ => {}
                }
            }).unwrap();
        }
    });

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Frame Time Graph",
        options,
        Box::new(|_cc| Ok(Box::new(FrameTimeApp { histogram }))),
    ).unwrap()
}
