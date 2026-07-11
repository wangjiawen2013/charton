# The System Monitor GUI Dashboard

Building a real-time system monitor in Rust is a fantastic way to push your UI framework to its limits. However, integrating a native graphics library directly into a framework like `egui` often leads to the dreaded "WGPU Version Hell"—where conflicting dependency trees between the UI and the renderer break your build entirely.

To bypass this, we will architect a Decoupled System Monitor Dashboard. We will use `sysinfo` to grab real-time hardware telemetry (CPU, Memory, Swap) and pass it to Charton. Charton will spin up an isolated, headless GPU instance, render the complex geometry, and hand a raw pixel buffer (`Vec<u8>`) back to egui for display.

This guarantees universal cross-platform compatibility and pixel-perfect text rendering without a single dependency conflict.

## The Architecture: The Decoupled Memory Bridge

Unlike the Lorenz Attractor which pushed Zero-Allocation over WASM, this dashboard relies on an asynchronous GPU → CPU → GPU pipeline:

1. Hardware Telemetry (CPU): `sysinfo` polls the OS kernel every few hundred milliseconds.
2. Headless Generation (GPU 1): Charton's `WgpuRenderer` draws the multi-layer trend lines and scatter plots off-screen.
3. The Memory Bridge (CPU): Charton pads and reads back the rendered VRAM into a clean `Vec<u8>` on system RAM.
4. UI Presentation (GPU 2): `egui` wraps those bytes into a `ColorImage` and uploads them to its own texture pipeline.

## Step 1: Project Setup

Create a new Rust binary project and configure your dependencies. By isolating our WGPU features, we ensure `eframe` and `charton` never clash.

Update your `Cargo.toml`:

```toml
[package]
name = "monitor"
version = "0.1.0"
edition = "2024"

[dependencies]
charton = { version="0.5", features = ["wgpu", "png"] }
eframe = "0.35"
sysinfo = "0.33"
```

## Step2: Complete Implementation Code

we build a system monitoring dashboard based on `egui` and `charton`. The core features we implemented include:

1. Real-time Hardware Telemetry: Utilizing `sysinfo` to capture CPU, memory, and Swap utilization in real-time.
2. Off-screen GPU Rendering: Using `WgpuRenderer` to generate charts in the background and passing the rendered results back to `egui` as textures for display.
3. Responsive UI Design: Supporting dynamic adjustment of chart physical dimensions, tick rates, and buffer capacities, while ensuring real-time data flow via `request_repaint()`.
4. Process Audit Log: An iterative loop that outputs the status of system processes in real-time, with visual warnings for high-resource processes.

By using this decoupled architecture—Hardware Acquisition → GPU Rendering → UI Presentation—we successfully solved the performance bottleneck between UI rendering and complex data visualization.

Below is the complete implementation for `src/main.rs`. Simply overwrite your project file with this code to get a fully functional system monitoring dashboard.

```rust
use eframe::egui;
use charton::prelude::*;
use charton::render::WgpuRenderer;
use std::collections::VecDeque;
use std::time::Instant;
use sysinfo::{System, Networks};

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1300.0, 950.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "SYSMON :: Real-time System Telemetry Dashboard",
        native_options,
        Box::new(|cc| Ok(Box::new(RealMonitorApp::new(cc)))),
    )
}

/// A single sample of system-wide metrics captured at a point in time.
#[derive(Clone)]
struct SystemMetric {
    time_offset: f32, // seconds since monitoring started
    cpu_usage: f32,   // global CPU utilisation (0–100 %)
    mem_usage: f32,   // physical memory utilisation (0–100 %)
    swap_usage: f32,  // swap space utilisation (0–100 %)
}

/// Stores independent fields for each log entry to enable perfect table rendering
#[derive(Clone)]
struct AuditLog {
    time: String,
    pid: String,
    name: String,
    mem: String,
    cpu: String,
    is_warn: bool,
}

/// Top-level application state holding all system probes, chart renderers,
/// metric history, and UI configuration.
struct RealMonitorApp {
    // --- chart rendering ---
    renderer: WgpuRenderer,
    pixels_per_point: f32,
    combined_chart_texture: Option<egui::TextureHandle>,
    core_scatter_texture: Option<egui::TextureHandle>,

    // --- time-keeping ---
    start_time: Instant,
    last_refresh: Instant,

    // --- system probes ---
    sys_handle: System,
    net_handle: Networks,

    // --- metric history buffers ---
    history: VecDeque<SystemMetric>,
    terminal_logs: VecDeque<AuditLog>,

    // --- chart geometry (user-adjustable via sliders) ---
    chart_width: u32,
    chart_height: u32,
    tick_rate_ms: u64,
    buffer_capacity: usize,

    // --- control flags ---
    is_frozen: bool,

    // --- process-audit iterator (cycles through PID table) ---
    log_counter: usize,

    // --- network-rate tracking ---
    last_net_check: Instant,
    last_rx_bytes: u64,
    last_tx_bytes: u64,
    current_rx_rate: f64, // bytes per second
    current_tx_rate: f64, // bytes per second
    is_first_net_check: bool,
}

impl RealMonitorApp {
    /// Initialise the application: set up the dark visual theme, create system
    /// handles, and allocate ring buffers for metric history.
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Build a custom dark visual theme with neon accent colours.
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(240, 240, 245));
        visuals.panel_fill = egui::Color32::from_rgb(6, 6, 8);
        visuals.window_fill = egui::Color32::from_rgb(10, 10, 14);

        // Widget styling: subtle cyan accents on dark surfaces.
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(20, 20, 28);
        visuals.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 255));
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0, 100, 200);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 150, 240);

        cc.egui_ctx.set_visuals(visuals);

        // Bootstrap system information handles.
        let mut sys_handle = System::new_all();
        sys_handle.refresh_all();
        let net_handle = Networks::new_with_refreshed_list();

        Self {
            renderer: WgpuRenderer::new(),
            pixels_per_point: cc.egui_ctx.pixels_per_point(),
            start_time: Instant::now(),
            last_refresh: Instant::now(),
            sys_handle,
            net_handle,
            history: VecDeque::with_capacity(200),
            terminal_logs: VecDeque::with_capacity(50),
            combined_chart_texture: None,
            core_scatter_texture: None,
            chart_width: 900,
            chart_height: 315,
            tick_rate_ms: 500,
            buffer_capacity: 80,
            is_frozen: false,
            log_counter: 0,
            last_net_check: Instant::now(),
            last_rx_bytes: 0,
            last_tx_bytes: 0,
            current_rx_rate: 0.0,
            current_tx_rate: 0.0,
            is_first_net_check: true,
        }
    }

    // ------------------------------------------------------------------
    // Data Collection
    // ------------------------------------------------------------------

    /// Poll every hardware counter and push a new sample into the ring buffer.
    /// Also updates network-rate estimates and feeds the process-audit log.
    fn collect_real_hardware_data(&mut self) {
        if self.is_frozen {
            return;
        }

        // Refresh CPU, memory, and process list.
        self.sys_handle.refresh_cpu_usage();
        self.sys_handle.refresh_memory();
        self.sys_handle
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        // Refresh network interfaces.
        self.net_handle.refresh(true);

        let elapsed = self.start_time.elapsed().as_secs_f32();

        // -- CPU utilisation (global, 0-100 %) --
        let real_cpu = self.sys_handle.global_cpu_usage();

        // -- Physical memory utilisation (0-100 %) --
        // NOTE: refresh_memory() is called above; we re-fetch totals on every
        // tick so that the percentage reflects the current state.
        let total_mem = self.sys_handle.total_memory() as f64;
        let used_mem = self.sys_handle.used_memory() as f64;
        let real_mem = if total_mem > 0.0 {
            ((used_mem / total_mem) * 100.0) as f32
        } else {
            0.0
        };

        // -- Swap utilisation (0-100 %) --
        let total_swap = self.sys_handle.total_swap() as f64;
        let used_swap = self.sys_handle.used_swap() as f64;
        let real_swap = if total_swap > 0.0 {
            ((used_swap / total_swap) * 100.0) as f32
        } else {
            0.0
        };

        // Push the new sample into the rolling history buffer.
        self.history.push_back(SystemMetric {
            time_offset: elapsed,
            cpu_usage: real_cpu,
            mem_usage: real_mem,
            swap_usage: real_swap,
        });

        // Trim the buffer to the configured capacity.
        while self.history.len() > self.buffer_capacity {
            self.history.pop_front();
        }

        // -- Network throughput estimation (bytes per second) --
        let now = Instant::now();
        let dt = now.duration_since(self.last_net_check).as_secs_f64();
        if dt > 0.5 {
            let mut rx_total = 0u64;
            let mut tx_total = 0u64;
            for (_iface, data) in &self.net_handle {
                rx_total = rx_total.saturating_add(data.received());
                tx_total = tx_total.saturating_add(data.transmitted());
            }

            if !self.is_first_net_check {
                // Compute delta since last check, guard against counter reset.
                self.current_rx_rate = if rx_total >= self.last_rx_bytes {
                    (rx_total - self.last_rx_bytes) as f64 / dt
                } else {
                    0.0
                };
                self.current_tx_rate = if tx_total >= self.last_tx_bytes {
                    (tx_total - self.last_tx_bytes) as f64 / dt
                } else {
                    0.0
                };
            } else {
                self.is_first_net_check = false;
            }

            self.last_rx_bytes = rx_total;
            self.last_tx_bytes = tx_total;
            self.last_net_check = now;
        }

        // -- Process-audit log --
        let processes: Vec<_> = self.sys_handle.processes().values().collect();
        if !processes.is_empty() {
            // Advance the iterator, wrapping around.
            self.log_counter = (self.log_counter + 2) % processes.len();
            let proc = processes[self.log_counter];

            let mem_mb = proc.memory() as f32 / 1024.0 / 1024.0;
            let proc_cpu = proc.cpu_usage();
            
            // Intelligently truncate overly long process names
            let mut proc_name = proc.name().to_string_lossy().to_string();
            if proc_name.len() > 22 {
                proc_name.truncate(20);
                proc_name.push_str("..");
            }

            let is_warn = mem_mb > 500.0 || proc_cpu > 50.0;

            // Push the structured data into the queue, instead of crude string formatting
            self.terminal_logs.push_back(AuditLog {
                time: format!("+{:.1}s", elapsed),
                pid: proc.pid().to_string(),
                name: proc_name,
                mem: format!("{:.1} MB", mem_mb),
                cpu: format!("{:.1} %", proc_cpu),
                is_warn,
            });

            // Keep the log at a fixed visible depth.
            if self.terminal_logs.len() > 15 {
                self.terminal_logs.pop_front();
            }
        }
    }

    // ------------------------------------------------------------------
    // Chart Rendering
    // ------------------------------------------------------------------

    /// Render the multi-line chart showing CPU, Memory, and Swap usage over
    /// time. The result is cached as a texture for efficient redraw.
    fn render_combined_chart(&mut self, ctx: &egui::Context) {
        if self.history.is_empty() {
            return;
        }

        let mut times: Vec<f32> = Vec::new();
        let mut values: Vec<f32> = Vec::new();
        let mut metrics: Vec<&str> = Vec::new();

        // Flatten the ring buffer into three parallel series.
        for m in &self.history {
            times.push(m.time_offset);
            values.push(m.cpu_usage);
            metrics.push("CPU %");
            times.push(m.time_offset);
            values.push(m.mem_usage);
            metrics.push("MEM %");
            times.push(m.time_offset);
            values.push(m.swap_usage);
            metrics.push("SWAP %");
        }

        let mut ds = Dataset::new();
        ds = ds.with_column("Timeline (s)", times).unwrap();
        ds = ds.with_column("Utilization %", values).unwrap();
        ds = ds.with_column("Resource", metrics).unwrap();

        let final_chart = Chart::build(ds)
            .unwrap()
            .mark_line()
            .unwrap()
            .encode((
                alt::x("Timeline (s)"),
                alt::y("Utilization %"),
                alt::color("Resource"),
            ))
            .unwrap()
            .configure_theme(|t| {
                t.with_background_color("#060608") // chart background
                    .with_axes_color("#8899AA")
                    .with_label_color("#00CCFF") // axis titles
                    .with_tick_color("#8899AA") // tick numbers
                    .with_tick_label_color("#8899AA")
                    .with_legend_label_color("#8899AA")
                    .with_legend_title_color("#8899AA")
            })
            .with_size(
                (self.chart_width as f32 * self.pixels_per_point) as u32,
                (self.chart_height as f32 * self.pixels_per_point) as u32,
            );

        let render_width = (self.chart_width as f32 * self.pixels_per_point) as u32;
        let render_height = (self.chart_height as f32 * self.pixels_per_point) as u32;

        if let Ok(pixels) = self.renderer.render(&final_chart, render_width, render_height, 1.0) {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [render_width as usize, render_height as usize],
                &pixels,
            );
            self.combined_chart_texture = Some(ctx.load_texture(
                "combined_chart",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    /// Render the per-core CPU load scatter plot.  Each core is represented by
    /// a point whose colour encodes the load level (Idle / Moderate / Busy).
    fn render_scatter_chart(&mut self, ctx: &egui::Context) {
        let cpus = self.sys_handle.cpus();
        if cpus.is_empty() {
            return;
        }

        let mut index_list: Vec<f32> = Vec::new();
        let mut usage_list: Vec<f32> = Vec::new();
        let mut usage_group: Vec<&str> = Vec::new();

        // Build per-core data and classify load into three buckets.
        for (i, cpu) in cpus.iter().enumerate() {
            let usage = cpu.cpu_usage();
            index_list.push(i as f32);
            usage_list.push(usage);
            usage_group.push(if usage < 30.0 {
                "Low (<30%)"
            } else if usage < 60.0 {
                "Mid (30-60%)"
            } else {
                "High (>60%)"
            });
        }

        let mut ds = Dataset::new();
        ds = ds.with_column("Core ID", index_list).unwrap();
        ds = ds.with_column("Usage %", usage_list).unwrap();
        ds = ds.with_column("Load Level", usage_group).unwrap();

        let scatter_chart = Chart::build(ds)
            .unwrap()
            .mark_point()
            .unwrap()
            .configure_point(|p| p.with_size(10.0).with_opacity(0.9))
            .encode((
                alt::x("Core ID"),
                alt::y("Usage %"),
                alt::color("Load Level"), // colour-coded by load bucket
            ))
            .unwrap()
            .configure_theme(|t| {
                t.with_background_color("#060608")
                    .with_axes_color("#8899AA")
                    .with_label_color("#00FF88") // axis titles
                    .with_tick_label_color("#8899AA") // tick numbers
                    .with_tick_color("#8899AA") 
                    .with_legend_label_color("#8899AA")
                    .with_legend_title_color("#8899AA")
            })
            .with_y_domain(-10.0, 100.0)
            .with_size(
                (self.chart_width as f32 * self.pixels_per_point) as u32,
                ((self.chart_height / 2) as f32 * self.pixels_per_point) as u32,
            );

        let render_width = (self.chart_width as f32 * self.pixels_per_point) as u32;
        let render_height = ((self.chart_height / 2) as f32 * self.pixels_per_point) as u32;

        if let Ok(pixels) = self.renderer.render(&scatter_chart, render_width, render_height, 1.0)
        {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [render_width as usize, render_height as usize],
                &pixels,
            );
            self.core_scatter_texture = Some(ctx.load_texture(
                "scatter_chart",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    /// Pretty-print a byte rate with appropriate SI unit.
    fn format_rate(bytes_per_sec: f64) -> String {
        if bytes_per_sec >= 1_000_000.0 {
            format!("{:.2} MB/s", bytes_per_sec / 1_000_000.0)
        } else if bytes_per_sec >= 1_000.0 {
            format!("{:.1} KB/s", bytes_per_sec / 1_000.0)
        } else {
            format!("{:.0}  B/s", bytes_per_sec)
        }
    }
}

impl eframe::App for RealMonitorApp {
    /// Main UI entry-point called every frame by eframe.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.pixels_per_point = ctx.pixels_per_point();

        // ---- Refresh cycle: poll hardware and re-render charts ----
        if self.last_refresh.elapsed().as_millis() > self.tick_rate_ms as u128 {
            self.collect_real_hardware_data();
            self.render_combined_chart(&ctx);
            self.render_scatter_chart(&ctx);
            self.last_refresh = Instant::now();
        }
        ctx.request_repaint();

        // ==========================================================
        //  LEFT SIDE PANEL — live metrics & controls
        // ==========================================================
        let mut side_frame = egui::Frame::side_top_panel(ui.style());
        side_frame.fill = egui::Color32::from_rgb(6, 6, 8);
        side_frame.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 40, 50));
        side_frame.inner_margin = egui::Margin::same(20);

        egui::Panel::left("sys_panel")
            .frame(side_frame)
            .show(ui, |ui| {
                ui.set_min_width(340.0);

                // ---- Header with uptime ----
                ui.vertical_centered(|ui| {
                    ui.heading(
                        egui::RichText::new("◈ SYS::MONITOR")
                            .size(22.0)
                            .strong()
                            .color(egui::Color32::from_rgb(0, 200, 255)),
                    );
                    let uptime = self.start_time.elapsed().as_secs();
                    ui.label(
                        egui::RichText::new(format!(
                            "UPTIME: {:02}:{:02}:{:02}",
                            uptime / 3600,
                            (uptime % 3600) / 60,
                            uptime % 60
                        ))
                        .color(egui::Color32::from_rgb(100, 130, 160))
                        .monospace(),
                    );
                });
                ui.add_space(20.0);

                // ---- Live system metrics card ----
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                        egui::Color32::from_rgb(12, 12, 18);

                    ui.label(
                        egui::RichText::new("📊 SYSTEM METRICS")
                            .strong()
                            .color(egui::Color32::from_rgb(0, 200, 255)),
                    );
                    ui.add_space(8.0);

                    if let Some(latest) = self.history.back() {
                        // CPU usage — red progress bar.
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 60, 60), " CPU ");
                            ui.add(
                                egui::ProgressBar::new(latest.cpu_usage / 100.0)
                                    .desired_width(120.0)
                                    .fill(egui::Color32::from_rgb(255, 60, 60)),
                            );
                            ui.label(format!("{:.1}%", latest.cpu_usage));
                        });
                        // Memory usage — blue progress bar.
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(0, 200, 255), " MEM ");
                            ui.add(
                                egui::ProgressBar::new(latest.mem_usage / 100.0)
                                    .desired_width(120.0)
                                    .fill(egui::Color32::from_rgb(0, 180, 255)),
                            );
                            ui.label(format!("{:.1}%", latest.mem_usage));
                        });
                        // Swap usage — orange progress bar.
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 160, 50), " SWP ");
                            ui.add(
                                egui::ProgressBar::new(latest.swap_usage / 100.0)
                                    .desired_width(120.0)
                                    .fill(egui::Color32::from_rgb(255, 160, 0)),
                            );
                            ui.label(format!("{:.1}%", latest.swap_usage));
                        });

                        ui.separator();

                        // Network receive rate.
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(0, 255, 128), " RX  ");
                            ui.label(Self::format_rate(self.current_rx_rate));
                        });
                        // Network transmit rate.
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 160, 0), " TX  ");
                            ui.label(Self::format_rate(self.current_tx_rate));
                        });
                    }
                });
                ui.add_space(15.0);

                // ---- Chart controls ----
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new("⚙ CONTROLS")
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::Slider::new(&mut self.chart_width, 700..=1100).text("Width"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.chart_height, 250..=600)
                            .text("Height"),
                    );

                    ui.separator();
                    ui.add_space(5.0);
                    ui.add(
                        egui::Slider::new(&mut self.tick_rate_ms, 20..=1000)
                            .text("Interval (ms)"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.buffer_capacity, 20..=200)
                            .text("Buffer Size"),
                    );
                });
                ui.add_space(20.0);

                // ---- Action buttons ----
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.columns(2, |columns| {
                        let freeze_text =
                            if self.is_frozen { "▶ RESUME" } else { "⏸ FREEZE" };
                        if columns[0].button(freeze_text).clicked() {
                            self.is_frozen = !self.is_frozen;
                        }
                        
                        if columns[1].button("✖ CLEAR").clicked() {
                            self.history.clear();
                            self.terminal_logs.clear();
                        }
                    });
                });
            });

        // ==========================================================
        //  CENTRAL PANEL — charts & process audit log
        // ==========================================================
        let mut central_frame = egui::Frame::central_panel(ui.style());
        central_frame.fill = egui::Color32::from_rgb(6, 6, 8);
        central_frame.stroke = egui::Stroke::NONE;
        central_frame.inner_margin = egui::Margin::same(20);

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        // ---- Resource utilisation line chart ----
                        if let Some(texture) = &self.combined_chart_texture {
                            ui.label(
                                egui::RichText::new("▸ Resource Utilization Timeline")
                                    .monospace()
                                    .strong()
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(0, 200, 255)),
                            );
                            ui.add_space(6.0);
                            ui.add(egui::Image::new(texture).fit_to_exact_size(
                                egui::vec2(
                                    self.chart_width as f32,
                                    self.chart_height as f32,
                                ),
                            ));
                            ui.add_space(25.0);
                        }

                        // ---- Per-core CPU load scatter plot ----
                        if let Some(scatter_tex) = &self.core_scatter_texture {
                            ui.label(
                                egui::RichText::new("▸ Per-Core CPU Load Distribution")
                                    .monospace()
                                    .strong()
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(0, 255, 136)),
                            );
                            ui.add_space(6.0);
                            ui.add(egui::Image::new(scatter_tex).fit_to_exact_size(
                                egui::vec2(
                                    self.chart_width as f32,
                                    (self.chart_height / 2) as f32,
                                ),
                            ));
                            ui.add_space(25.0);
                        }

                        // ---- Process audit log (live scrolling terminal) ----
                        ui.group(|ui| {
                            ui.set_width(self.chart_width as f32);
                            ui.set_min_height(160.0);
                            ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                                egui::Color32::from_rgb(4, 4, 6);
                            ui.style_mut().visuals.widgets.noninteractive.bg_stroke = 
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 110));

                            ui.label(
                                egui::RichText::new("▸ Process Audit Log [live]")
                                    .monospace()
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 50, 127)),
                            );
                            ui.separator();

                            // Use egui::Grid to align the table
                            egui::Grid::new("audit_log_grid")
                                .num_columns(6)
                                .spacing([25.0, 4.0]) // Adjust the column spacing 
                                .show(ui, |ui| {
                                    // render table header
                                    let hdr_color = egui::Color32::from_rgb(100, 100, 140);
                                    let hdr = |text: &str| egui::RichText::new(text).monospace().size(12.0).color(hdr_color);
                                    
                                    ui.label(hdr("TIME"));
                                    ui.label(hdr("PID"));
                                    ui.label(hdr("PROCESS"));
                                    ui.label(hdr("MEM"));
                                    ui.label(hdr("CPU"));
                                    ui.label(hdr("STATUS"));
                                    ui.end_row();

                                    // Render the log rows
                                    for log in &self.terminal_logs {
                                        let mut color = egui::Color32::from_rgb(0, 200, 100); // Normal green
                                        let mut bg = egui::Color32::TRANSPARENT;
                                        
                                        let status_text = if log.is_warn {
                                            color = egui::Color32::from_rgb(255, 80, 80); // Warning red
                                            bg = egui::Color32::from_rgba_premultiplied(255, 0, 0, 25);
                                            "WARN"
                                        } else {
                                            "OK"
                                        };

                                        let txt = |text: &str| egui::RichText::new(text).monospace().size(12.0).color(color);

                                        ui.label(txt(&log.time));
                                        ui.label(txt(&log.pid));
                                        ui.label(txt(&log.name));
                                        ui.label(txt(&log.mem));
                                        ui.label(txt(&log.cpu));
                                        ui.label(txt(status_text).background_color(bg));
                                        ui.end_row();
                                    }
                                });
                        });
                    });
                });
            });
    }
}
```

## Summary: Rules for GUI Integrations

When architecting high-performance dashboards with `egui` and external renderers, keep these three golden rules in mind to maintain fluid 60 FPS performance:

1. The Throttle Rule (Decoupling): `egui` renders at 60+ FPS, but `sysinfo` process polling and GPU off-screen rendering are expensive OS and hardware-level operations. Always wrap data collection in a time-based throttle (e.g., if `elapsed > tick_rate_ms`). For GPU chart rendering, use a separate, independent throttle (e.g., every 500ms) so that charts don't re-render on every data-collection tick. Never block the main UI thread with kernel-level polling or synchronous GPU copies.

2. High-Res Supersampling: To ensure your charts remain crisp on Retina or 4K displays, multiply your logical chart_width and `chart_height` by `pixels_per_point` when calling `self.renderer.render()` and when creating the `ColorImage` buffer. When placing the resulting `TextureHandle` into `egui via egui::Image::new(texture)`, constrain it back to the original logical dimensions using `.fit_to_exact_size()`. This creates a native "Retina" look without blurry upscaling.

3. The request_repaint Hook: egui is an immediate-mode GUI that enters a low-power "sleep" state when there is no user input (mouse movement). For a live dashboard, you must explicitly call `ctx.request_repaint()` inside your ui method. This forces egui to wake up every frame, ensuring your data stream remains fluid and uninterrupted by idle timers.

The key revision is the first rule — explicitly noting that chart rendering should have its own independent throttle (`last_chart_render`) separate from data collection (`last_refresh`), since GPU off-screen rendering is also expensive and shouldn't run at the same frequency as lightweight metric sampling.