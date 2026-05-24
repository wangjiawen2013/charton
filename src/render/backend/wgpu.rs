use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuPoint {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub radius: f32,
    pub shape_type: f32, // 0.0 = Circle, 1.0 = Rounded Rect (Perfect alignment: 8 * 4 = 32 bytes)
}

impl From<CircleConfig> for GpuPoint {
    fn from(config: CircleConfig) -> Self {
        let rgba = config.fill.rgba();
        Self {
            x: config.x,
            y: config.y,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            radius: config.radius,
            shape_type: 0.0, // Circle flag
        }
    }
}

// Enable support for RectConfig mapping to the same shader pipeline!
impl From<RectConfig> for GpuPoint {
    fn from(config: RectConfig) -> Self {
        let rgba = config.fill.rgba();
        Self {
            x: config.x + config.width / 2.0, // Move origin to center for SDF calculation
            y: config.y + config.height / 2.0, // Move origin to center for SDF calculation
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            // Use half of width as the bounding radius metric for the SDF size box
            radius: config.width / 2.0,
            shape_type: 1.0, // Rounded Rectangle flag
        }
    }
}

// Included to keep compiler happy based on your implementation structure
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    screen_width: f32,
    screen_height: f32,
}

pub struct WgpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    pending_points: Vec<GpuPoint>,
    point_count: u32,
}

impl RenderBackend for WgpuBackend {
    fn draw_circle(&mut self, config: CircleConfig) {
        self.pending_points.push(GpuPoint::from(config));
    }

    fn draw_rect(&mut self, config: RectConfig) {
        // Now natively supported by Layer 2 SDF pipeline!
        self.pending_points.push(GpuPoint::from(config));
    }

    fn draw_path(&mut self, _config: PathConfig) {}
    fn draw_polygon(&mut self, _config: PolygonConfig) {}
    fn draw_text(&mut self, _config: TextConfig) {}
    fn draw_line(&mut self, _config: LineConfig) {}
    fn draw_gradient_rect(&mut self, _config: GradientRectConfig) {}
}

impl WgpuBackend {
    pub fn flush_and_render(&mut self, view: &wgpu::TextureView) {
        if !self.pending_points.is_empty() {
            // High-speed buffer swap and sync
            let points_to_upload = std::mem::take(&mut self.pending_points);
            self.update_points(&points_to_upload);
            self.pending_points = points_to_upload;
            self.pending_points.clear();
        }
        self.render(view);
    }

    pub async fn new(device: wgpu::Device, queue: wgpu::Queue, width: u32, height: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chart Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("chart.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                screen_width: width as f32,
                screen_height: height as f32,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Chart Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // Visibility updated: Fragment shader now needs access to read point data parameters via instance index
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Chart Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Safe Fix: Allocated 32 bytes of zeroed data to provide a valid non-empty initial storage link
        let dummy_storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dummy Storage"),
            contents: bytemuck::cast_slice(&[GpuPoint {
                x: 0.0,
                y: 0.0,
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
                radius: 0.0,
                shape_type: 0.0,
            }]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        dummy_storage.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        uniform_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
            label: Some("Initial Chart Bind Group"),
        });

        Self {
            device,
            queue,
            pipeline,
            bind_group,
            uniform_buffer,
            pending_points: Vec::with_capacity(10_000), // Pre-allocated vector buffer to prevent realloc overheads
            point_count: 0,
        }
    }

    pub fn update_points(&mut self, points: &[GpuPoint]) {
        if points.is_empty() {
            return;
        }

        // Reallocates buffer inside GPU global storage context matching point sizes safely
        let storage_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Points Storage Buffer"),
                contents: bytemuck::cast_slice(points),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        storage_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        self.uniform_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
            label: None,
        });

        self.point_count = points.len() as u32;
    }

    pub fn render(&self, view: &wgpu::TextureView) {
        if self.point_count == 0 {
            return;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Chart Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.draw(0..4, 0..self.point_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
