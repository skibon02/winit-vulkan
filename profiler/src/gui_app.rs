use std::cmp;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use colors_transform::{Color, Hsl};
use egui::{Color32, Context, ViewportCommand};
use egui_plot::{log_grid_spacer, Bar, BarChart, Legend, Plot};
use log::info;
use ringbuf::LocalRb;
use ringbuf::storage::Heap;
use ringbuf::traits::{Consumer, Observer, RingBuffer};
use sha2::Digest;

#[derive(Default, Clone)]
pub struct FrameTimeSample {
    pub name: Arc<str>,
    pub start: u64,
    pub dur: u64,
    pub inner: Vec<FrameTimeSample>
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
pub struct BufferedHistogram {
    data: LocalRb<Heap<FrameTimeSample>>,
    cur_frame: u64,
    max_start: u64,
    ovh_samples: LocalRb<Heap<(f64, f64)>>,

    prev_dense_event: Option<u64>,
    min_event_overhead: Option<u64>,

    ctx: Option<Context>
}

const TIME_BUFFER_S: f64 = 2.5;
impl BufferedHistogram {
    pub fn new() -> Self {
        Self {
            data: LocalRb::new(10_000),
            cur_frame: 0,
            max_start: 0,
            ovh_samples: LocalRb::new(100),

            prev_dense_event: None,
            min_event_overhead: None,
            ctx: None
        }
    }
    pub fn push(&mut self, sample: FrameTimeSample) {
        self.max_start = sample.start;
        self.cur_frame += 1;
        self.data.push_overwrite(sample);

        self.cleanup_if_needed();
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    pub fn set_context(&mut self, ctx: &Context) {
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone())
        }
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
            self.data.try_pop();
        }
    }

    fn get_unsorted(&self) -> Vec<FrameTimeSample> {
        let data = self.data.as_slices();
        let mut res = Vec::with_capacity(data.0.len() + data.1.len());
        res.extend_from_slice(data.0);
        res.extend_from_slice(data.1);
        res
    }

    pub fn add_overhead(&mut self, start: f64, dur: f64) {
        self.ovh_samples.push_overwrite((start, dur));
    }

    // FPS, 1% low, sparkles flushing overhead, sparkles event overhead
    fn cur_stats(&self) -> (f64, f64, f64, Option<f64>) {
        if self.data.is_empty() {
            return (0., 0., 0., None);
        }
        let mut frame_times = self.data.iter().map(|s| s.dur as f64 / 1000.0).collect::<Vec<_>>();
        frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let frame_time_sum: f64 = frame_times.iter().sum();
        let low_1_frame_time = frame_times[((frame_times.len() as f64 - 1.) * 0.99) as usize];
        let avg_frame_time = frame_time_sum / self.data.occupied_len() as f64;

        // calculate overhead for last 1 sec
        let cur_time = self.max_start;
        let mut total_dur = 0.0;
        for (start, dur) in self.ovh_samples.iter().rev() {
            if *start + 1_000_000_000.0 < cur_time as f64 {
                continue;
            }

            total_dur += dur;
        }

        let event_ovh = self.min_event_overhead.map(|ovh| {
            let mut event_cnt = 0;
            for sample in self.data.iter().rev() {
                if sample.start as f64 + 1_000_000_000.0 < cur_time as f64 {
                    continue;
                }

                event_cnt += (1 + sample.inner.len()) * 2 + 6;
            }
            ovh as f64 * event_cnt as f64 / 1_000_000_000.0
        });

        (avg_frame_time, low_1_frame_time, total_dur / 1_000_000_000.0, event_ovh)
    }

    pub fn dense_event(&mut self, tm: u64) {
        let Some(prev) = self.prev_dense_event else {
            self.prev_dense_event = Some(tm);
            return;
        };

        let dur = tm - prev;
        if self.min_event_overhead.is_some_and(|v| dur * 2 < v) {
            self.min_event_overhead = Some(dur * 2);
            info!("Got new event overhead: {}", dur * 2)
        }
        if self.min_event_overhead.is_none() {
            self.min_event_overhead = Some(dur);
        }
        self.prev_dense_event = Some(tm);
    }
}

pub struct FrameTimeApp {
    pub histogram: Arc<Mutex<BufferedHistogram>>,
    pub disconnected: Arc<AtomicBool>,
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
        if self.disconnected.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        let mut unsorted_data = self.histogram.lock().unwrap().get_unsorted();

        let unsorted_height_scale = 0.5;
        let max_i = unsorted_data.len();
        let max_start = unsorted_data.iter().map(|s| s.start).max().unwrap_or(0);
        let mut unsorted_bars = BTreeMap::new();
        unsorted_data.iter().enumerate().for_each(|(i, sample)| {
            let cur_pos = i as f64 / max_i as f64 * 100.0;
            let freshness = (max_start - sample.start) as f64 / 1_000_000_000.0;
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
            let (r,g,b) = color_from_name(&name);
            BarChart::new(bars)
                .name(&name)
                .color(Color32::from_rgb(r,g,b))
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name, b.value / unsorted_height_scale)
                }))
        }).collect::<Vec<_>>();

        let sorted_charts = sorted_bars.into_iter().map(|(name, bars)| {
            let (r,g,b) = color_from_name(&name);
            BarChart::new(bars)
                .name(&name)
                .color(Color32::from_rgb(r,g,b))
                .element_formatter(Box::new(move |b, _| {
                    format!("{} - {}us", name, b.value)
                }))
        }).collect::<Vec<_>>();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.label("Frame Time Analysis");
        });

        let (fps, low_1_percent, flush_overhead, event_overhead) = self.histogram.lock().unwrap().cur_stats();
        let fps = 1_000_000.0 / fps;
        let low_1_percent = 1_000_000.0 / low_1_percent;
        let flush_overhead = 1000.0 * flush_overhead;
        let event_overhead = event_overhead.map(|v| format!("{:.3}", v * 1000.0)).unwrap_or("-".to_string());


        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.heading("Stats");
            ui.label(format!("FPS: {:.2}", fps));
            ui.label(format!("1% Low: {:.2}", low_1_percent));
            ui.label(format!("Sparkles flush overhead: {:.3} ms/s", flush_overhead));
            ui.label(format!("Sparkles event overhead: {} ms/s", event_overhead));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            Plot::new("Frame Time Histogram")
                .legend(Legend::default())
                .y_axis_label("t (us)")
                // .grid_spacing(Rangef::new(0.01, 0.01))
                .x_grid_spacer(log_grid_spacer(20))
                .y_grid_spacer(log_grid_spacer(20))
                .cursor_color(Color32::PURPLE)
                .show(ui, |plot_ui| {
                    for chart in unsorted_charts {
                        plot_ui.bar_chart(chart);
                    }
                    for chart in sorted_charts {
                        plot_ui.bar_chart(chart);
                    }
                })
        });

        self.histogram.lock().unwrap().set_context(&ctx);
    }
}

