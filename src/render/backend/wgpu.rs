use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ============================================================================
// Pipeline: Universal Instanced SDF Scatter Rendering Structures
// ============================================================================

/// Represents instance data for unified geometric elements mapped through pure SDF shaders.
/// This layout maintains a strict 1:1 binary alignment with `PointData` in the WGSL shader.
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
    pub shape_type: f32, 
    // Mapped via PointShape enum integer IDs:
    // 0.0 = Circle, 1.0 = Square, 2.0 = Triangle, 3.0 = Star,
    // 4.0 = Diamond, 5.0 = Pentagon, 6.0 = Hexagon, 7.0 = Octagon
}

impl From<CircleConfig> for GpuPoint {
    fn from(config: CircleConfig) -> Self {
        let rgba = config.fill.rgba();
        Self {
            x: config.x as f32,
            y: config.y as f32,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            radius: config.radius as f32,
            shape_type: 0.0, // Circle shape variant identifier
        }
    }
}

impl From<RectConfig> for GpuPoint {
    fn from(config: RectConfig) -> Self {
        let rgba = config.fill.rgba();
        Self {
            // Calculate center position coordinates for accurate SDF radial mapping
            x: (config.x + config.width / 2.0) as f32,
            y: (config.y + config.height / 2.0) as f32,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            radius: (config.width / 2.0) as f32,
            shape_type: 1.0, // Square shape variant identifier
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
    _padding: f32,
}

pub struct WgpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    uniform_buffer: wgpu::Buffer,
    
    // The single high-performance pipeline driving all visual marks via GPU instancing
    sdf_pipeline: wgpu::RenderPipeline,
    sdf_bind_group: wgpu::BindGroup,
    pending_points: Vec<GpuPoint>,
    uploaded_point_count: u32,
}

impl RenderBackend for WgpuBackend {
    fn draw_circle(&mut self, config: CircleConfig) {
        self.pending_points.push(GpuPoint::from(config));
    }

    fn draw_rect(&mut self, config: RectConfig) {
        self.pending_points.push(GpuPoint::from(config));
    }

    fn draw_polygon(&mut self, config: PolygonConfig) {
        let vertex_count = config.points.len();
        if vertex_count < 3 {
            return;
        }

        let fill_color = config.fill.rgba();
        let alpha = config.fill_opacity as f32;

        // 1. Compute the structural centroid of the arbitrary incoming polygon
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        for &(x, y) in &config.points {
            cx += x as f32;
            cy += y as f32;
        }
        cx /= vertex_count as f32;
        cy /= vertex_count as f32;

        // 2. Estimate bounding radius via mathematical distance to the first vertex
        let r_x = config.points[0].0 as f32 - cx;
        let r_y = config.points[0].1 as f32 - cy;
        let estimated_radius = (r_x * r_x + r_y * r_y).sqrt();

        // 3. Match shape identity flags dynamically based on structural vertex layout counts
        let shape_type = match vertex_count {
            3 => 2.0,  // Equilateral Triangle
            10 => 3.0, // Star shape (consists of 5 outer and 5 inner vertices = 10)
            4 => 4.0,  // Diamond / Rhombus orientation
            5 => 5.0,  // Regular Pentagon
            6 => 6.0,  // Regular Hexagon
            8 => 7.0,  // Regular Octagon
            _ => 0.0,  // Fallback graceful degradation to Circle if unrecognized
        };

        // Stream straight to the uniform GPU instance caching collection array safely
        self.pending_points.push(GpuPoint {
            x: cx,
            y: cy,
            r: fill_color[0],
            g: fill_color[1],
            b: fill_color[2],
            a: fill_color[3] * alpha,
            radius: estimated_radius,
            shape_type,
        });
    }

    fn draw_path(&mut self, _config: PathConfig) {}
    fn draw_text(&mut self, _config: TextConfig) {}
    fn draw_line(&mut self, _config: LineConfig) {}
    fn draw_gradient_rect(&mut self, _config: GradientRectConfig) {}
}

impl WgpuBackend {
    pub fn flush_and_render(&mut self, view: &wgpu::TextureView) {
        if !self.pending_points.is_empty() {
            let points_to_upload = std::mem::take(&mut self.pending_points);
            self.update_points_buffer(&points_to_upload);
            self.pending_points = points_to_upload;
            self.pending_points.clear();
        }
        self.render(view);
    }

    pub async fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pure Instanced Master SDF Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("chart.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Global Projection Uniform Metrics Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                screen_width: width as f32,
                screen_height: height as f32,
                scale_factor,
                _padding: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sdf_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Layout Description Binding Interface"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
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
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let sdf_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Unified SDF Pipeline Hierarchy Layout Configuration"),
            bind_group_layouts: &[Some(&sdf_bind_group_layout)],
            immediate_size: 0,
        });

        let sdf_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Universal Pure Geometry Instanced SDF Render Pipeline"),
            layout: Some(&sdf_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
            cache: None,
            multiview_mask: None,
        });

        let dummy_storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SDF Storage Dummy Matrix Initialization"),
            contents: bytemuck::cast_slice(&[GpuPoint {
                x: 0.0, y: 0.0, r: 0.0, g: 0.0, b: 0.0, a: 0.0, radius: 0.0, shape_type: 0.0,
            }]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let sdf_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &sdf_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(dummy_storage.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()) },
            ],
            label: None,
        });

        Self {
            device,
            queue,
            uniform_buffer,
            sdf_pipeline,
            sdf_bind_group,
            pending_points: Vec::with_capacity(30_000),
            uploaded_point_count: 0,
        }
    }

    pub fn update_points_buffer(&mut self, points: &[GpuPoint]) {
        if points.is_empty() {
            return;
        }

        let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SDF Instanced Storage GPU Data Buffer"),
            contents: bytemuck::cast_slice(points),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_group_layout = self.sdf_pipeline.get_bind_group_layout(0);
        
        // FIX: Removed the incorrect .create_device() typo method call chain link context completely.
        self.sdf_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(storage_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(self.uniform_buffer.as_entire_buffer_binding()) },
            ],
            label: None,
        });

        self.uploaded_point_count = points.len() as u32;
    }

    pub fn render(&mut self, view: &wgpu::TextureView) {
        if self.uploaded_point_count == 0 {
            return;
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("One-Shot Unified SDF Pass Encoder Pass Matrix"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SDF Main Execution Graph View Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Dispatches a single draw instruction passing all element geometry arrays concurrently
            rpass.set_pipeline(&self.sdf_pipeline);
            rpass.set_bind_group(0, &self.sdf_bind_group, &[]);
            rpass.draw(0..4, 0..self.uploaded_point_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}