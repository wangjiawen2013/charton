# Integrating Charton with egui (Decoupled Approach)

In desktop data analytics software and quantitative trading dashboards, egui is the most popular GUI framework in the Rust community.

However, embedding an independent graphics library directly into egui's rendering loop often leads to "WGPU Version Hell"—if the two libraries rely on conflicting versions of `wgpu`, the project will fail to compile. To guarantee native cross-platform compatibility and pixel-perfect text rendering, Charton utilizes a Decoupled Memory Bridge architecture to interface with egui.

This tutorial guides you through creating an interactive dashboard featuring a sleek, borderless layout and a custom sidebar panel where chart dimensions can be adjusted smoothly in real time.

## Cargo Dependencies

Add `eframe` (the official framework wrapping egui) and `charton` to your project's `Cargo.toml`. By leveraging off-screen rendering via raw byte buffers, your application avoids explicit dependency conflicts.

```toml
[package]
name = "charton_egui_example"
version = "0.1.0"
edition = "2024"

[dependencies]
charton = { version="0.5", features = ["wgpu", "png"] }
eframe = "0.35"
```

## Step-by-Step Architecture Breakdown

### Step 1: Tracking the High-DPI Scale Factor

High-density screens (like Mac Retina or 4K monitors) require special scaling parameters. We store `pixels_per_point` directly in the application's state (`ChartApp`), initialized safely from `eframe::CreationContext` during startup.

### Step 2: The Borderless UI Control Panel

We strip away egui’s default gray separator lines by overriding the layout frames with `egui::Frame::stroke = egui::Stroke::NONE`. Sliders use immediate-mode checks (`.changed()`) to catch UI updates exactly when they occur.

### Step 3: High-Res Supersampling Logic

To completely eliminate blurry lines and text, your code implements supersampling. We multiply the logical canvas size by our monitor's scale factor, instructing Charton to render a massive, razor-sharp physical image buffer behind the scenes.

### Step 4: The Downscaled Memory Bridge

Once Charton passes back the raw high-resolution pixel vector (`Vec<u8>`), we load it into an `egui::ColorImage` and pass it to egui’s texture memory. Finally, we tell `egui::Image` to pack that dense, high-resolution texture back down into its exact logical dimensions (`fit_to_exact_size`), resulting in a pristine, crystal-clear visualization.

## The Complete Implementation

```rust
use eframe::egui;
use charton::prelude::*;
use charton::render::WgpuRenderer;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Charton + egui Dynamic Dashboard",
        native_options,
        Box::new(|cc| Ok(Box::new(ChartApp::new(cc)))),
    )
}

struct ChartApp {
    renderer: WgpuRenderer,
    chart_width: u32,
    chart_height: u32,
    chart_texture: Option<egui::TextureHandle>,
    pixels_per_point: f32,  // Stores the screen scale factor (DPI)
}

impl ChartApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Retrieve the monitor's DPI scale factor
        let pixels_per_point = cc.egui_ctx.pixels_per_point();
        
        Self {
            renderer: WgpuRenderer::new(),
            chart_width: 750,
            chart_height: 550,
            chart_texture: None,
            pixels_per_point,
        }
    }

    fn build_chart(&self) -> LayeredChart {
        let ds = load_dataset("penguins").unwrap(); 
        
        Chart::build(&ds)
            .unwrap()
            .mark_point()
            .unwrap()
            .configure_point(|p| p.with_size(5.0))
            .encode((
                alt::x("Flipper Length (mm)").with_zero(false),
                alt::y("Body Mass (g)").with_zero(false),
                alt::color("Species"),
            ))
            .unwrap()
            .configure_theme(|t| t.with_label_size(20.0).with_tick_label_size(18.0))
            // Render at physical pixels, then downscale for display
            .with_size(
                (self.chart_width as f32 * self.pixels_per_point) as u32,
                (self.chart_height as f32 * self.pixels_per_point) as u32,
            )
            .into()
    }
}

impl eframe::App for ChartApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut inputs_changed = false;
        let ctx = ui.ctx().clone();
        
        // Update scale factor (might change if window moves across monitors)
        self.pixels_per_point = ctx.pixels_per_point();

        // ==========================================
        // Visual Polish 1: Custom Frame for Left Control Panel
        // ==========================================
        let mut side_panel_frame = egui::Frame::side_top_panel(ui.style());
        side_panel_frame.stroke = egui::Stroke::NONE;
        side_panel_frame.inner_margin = egui::Margin::same(20);

        egui::Panel::left("controls")
            .frame(side_panel_frame)
            .show(ui, |ui| {
                ui.set_min_width(260.0);
                
                ui.vertical_centered(|ui| {
                    ui.heading(egui::RichText::new("📊 Dashboard").size(24.0).strong());
                });
                ui.add_space(30.0);
                
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new("Canvas Size").strong());
                    ui.add_space(10.0);
                    
                    if ui.add(egui::Slider::new(&mut self.chart_width, 400..=1000).text("Width")).changed() {
                        inputs_changed = true;
                    }
                    ui.add_space(5.0);
                    if ui.add(egui::Slider::new(&mut self.chart_height, 300..=800).text("Height")).changed() {
                        inputs_changed = true;
                    }
                });
                
                ui.add_space(20.0);
                ui.label(egui::RichText::new("Tip: Drag sliders to trigger fast WGPU off-screen rendering.").weak());
            });

        // ==========================================
        // Chart Rendering Logic
        // ==========================================
        if self.chart_texture.is_none() || inputs_changed {
            let chart = self.build_chart();
            
            // Render using high resolution (supersampling)
            let render_width = (self.chart_width as f32 * self.pixels_per_point) as u32;
            let render_height = (self.chart_height as f32 * self.pixels_per_point) as u32;
            
            if let Ok(pixels) = self.renderer.render(&chart, render_width, render_height, 1.0) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [render_width as usize, render_height as usize],
                    &pixels,
                );
                
                self.chart_texture = Some(ctx.load_texture(
                    "charton_texture",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // ==========================================
        // Visual Polish 3: Custom Frame for Central Panel
        // ==========================================
        let mut central_frame = egui::Frame::central_panel(ui.style());
        central_frame.stroke = egui::Stroke::NONE;
        central_frame.inner_margin = egui::Margin::same(20);

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ui, |ui| {
                if let Some(texture) = &self.chart_texture {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            // Display at logical size, allowing egui to scale automatically
                            ui.add(
                                egui::Image::new(texture)
                                    .fit_to_exact_size(egui::vec2(
                                        self.chart_width as f32,
                                        self.chart_height as f32,
                                    ))
                            );
                        });
                    });
                }
            });
    }
}
```
