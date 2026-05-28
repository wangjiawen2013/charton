use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ============================================================================
// Pipeline 1: Instanced SDF Scatter Rendering Structures
// ============================================================================

/// Represents instance data for raw geometric elements mapped through SDF shaders.
/// This layout must maintain a strict 1:1 binary alignment with `PointData` in WGSL.
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
            shape_type: 0.0, // Circle shape variant identifier
        }
    }
}

impl From<RectConfig> for GpuPoint {
    fn from(config: RectConfig) -> Self {
        let rgba = config.fill.rgba();
        Self {
            x: config.x + config.width / 2.0,  // Displace local origin to center for proper SDF calculation
            y: config.y + config.height / 2.0, // Displace local origin to center for proper SDF calculation
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            radius: config.width / 2.0,        // Bounding metric for the box-extent sizing
            shape_type: 1.0,                   // Rounded Rectangle shape variant identifier
        }
    }
}

// ============================================================================
// Pipeline 2: Arbitrary Polygon Rendering Structures (Triangle Fans)
// ============================================================================

/// Represents a standalone vertex packed explicitly for custom polygon drawing.
/// This matches the structure layout expected by `PolygonInput` inside WGSL.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

// ============================================================================
// Global Shared Pipeline Contexts
// ============================================================================

/// Shared viewport and environment uniforms.
/// Enforces a strict 16-byte alignment constraint matching WebGPU specifications.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32, // HiDPI scaling factor for runtime transformations
    _padding: f32,     // 4-byte structural padding to fulfill 16-byte alignment rules
}

/// A high-performance WebGPU-native implementation of the `RenderBackend` trait.
pub struct WgpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    
    // Core Shared uniform context
    uniform_buffer: wgpu::Buffer,
    
    // Pipeline 1 Context: High-performance Instanced SDF Rendering
    sdf_pipeline: wgpu::RenderPipeline,
    sdf_bind_group: wgpu::BindGroup,
    pending_points: Vec<GpuPoint>,
    uploaded_point_count: u32,

    // Pipeline 2 Context: Arbitrary Convex / Star Poly Tessellator
    poly_pipeline: wgpu::RenderPipeline,
    poly_bind_group: wgpu::BindGroup,
    pending_vertices: Vec<GpuVertex>,
}

impl RenderBackend for WgpuBackend {
    fn draw_circle(&mut self, config: CircleConfig) {
        // Enqueue logical coordinate context; target scaling handles resolution inside the pipeline
        self.pending_points.push(GpuPoint::from(config));
    }

    fn draw_rect(&mut self, config: RectConfig) {
        // Natively routed down the instance-driven SDF pathing layer
        self.pending_points.push(GpuPoint::from(config));
    }

    fn draw_polygon(&mut self, config: PolygonConfig) {
        let vertex_count = config.points.len();
        if vertex_count < 3 {
            return; // Invalid spatial footprint topology
        }

        let fill_color = config.fill.rgba();
        let alpha = config.fill_opacity as f32;
        let packed_color = [
            fill_color[0],
            fill_color[1],
            fill_color[2],
            fill_color[3] * alpha,
        ];

        // 1. Extract and compute the centroid (geometric center) of the poly-shape
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        for &(x, y) in &config.points {
            cx += x;
            cy += y;
        }
        cx /= vertex_count as f32;
        cy /= vertex_count as f32;

        let center_vertex = GpuVertex {
            pos: [cx, cy],
            color: packed_color,
        };

        // 2. Map coordinates through a center-point triangle fan layout translated into individual triangle slices
        for i in 0..vertex_count {
            let current_pt = config.points[i];
            let next_pt = config.points[(i + 1) % vertex_count]; // Circular wrap-around layout

            // Anchor point triangle slice component 1: Shape Centroid
            self.pending_vertices.push(center_vertex);

            // Anchor point triangle slice component 2: Current Node Position
            self.pending_vertices.push(GpuVertex {
                pos: [current_pt.0, current_pt.1],
                color: packed_color,
            });

            // Anchor point triangle slice component 3: Next Adjacent Node Position
            self.pending_vertices.push(GpuVertex {
                pos: [next_pt.0, next_pt.1],
                color: packed_color,
            });
        }
    }

    // Unused backend specifications reserved for future platform expansion modules
    fn draw_path(&mut self, _config: PathConfig) {}
    fn draw_text(&mut self, _config: TextConfig) {}
    fn draw_line(&mut self, _config: LineConfig) {}
    fn draw_gradient_rect(&mut self, _config: GradientRectConfig) {}
}

impl WgpuBackend {
    /// Flushes all staging queues and schedules a complete layout frame presentation pass.
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
        // Compile unified shader module source context
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Unified Chart Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("chart.wgsl").into()),
        });

        // Initialize shared uniform context allocation matrix mapping
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Global Viewport Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                screen_width: width as f32,
                screen_height: height as f32,
                scale_factor,
                _padding: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // --------------------------------------------------------------------
        // Setup Pipeline 1: Instanced SDF Scatter Rendering
        // --------------------------------------------------------------------
        let sdf_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // Storage array containing points data metrics context
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, // Uniform canvas dimension layout settings block
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
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[Some(&sdf_bind_group_layout)],
            immediate_size: 0,
        });

        let sdf_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Render Pipeline"),
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
                topology: wgpu::PrimitiveTopology::TriangleStrip, // Generates a quad procedural frame context natively
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

        // Initialize empty dummy space to bind resource storage contexts safely at frame boot up
        let dummy_storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dummy SDF Storage Buffer Initializer"),
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
            label: Some("Initial SDF Bind Group Instance"),
        });

        // --------------------------------------------------------------------
        // Setup Pipeline 2: Arbitrary Convex / Star Polygon Tessellator
        // --------------------------------------------------------------------
        let poly_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Polygon Pipeline Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // Uniform metrics mapping binding context slot
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

        let poly_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Polygon Pipeline Layout Matrix"),
            bind_group_layouts: &[Some(&poly_bind_group_layout)],
            immediate_size: 0,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Location 0: 2D Vertex Position
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                // Location 1: RGBA Vertex Color Component mapping values
                wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
            ],
        };

        let poly_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Polygon Render Pipeline Instance"),
            layout: Some(&poly_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_polygon"),
                buffers: &[vertex_buffer_layout], // Standard explicit vertex sequential data indexing pathway setup
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_polygon"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Standard triangle stream grouping configuration layout
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

        let poly_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &poly_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()) },
            ],
            label: Some("Initial Polygon Pipeline Uniform Binding Context Lookup"),
        });

        Self {
            device,
            queue,
            uniform_buffer,
            sdf_pipeline,
            sdf_bind_group,
            pending_points: Vec::with_capacity(10_000),
            uploaded_point_count: 0,
            poly_pipeline,
            poly_bind_group,
            pending_vertices: Vec::with_capacity(30_000),
        }
    }

    /// Reallocates and streams instance properties over down to the target GPU Storage block context framework safely.
    pub fn update_points_buffer(&mut self, points: &[GpuPoint]) {
        if points.is_empty() {
            return;
        }

        let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SDF Instanced Storage Buffer Data Upload Stream"),
            contents: bytemuck::cast_slice(points),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_group_layout = self.sdf_pipeline.get_bind_group_layout(0);
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

    /// Orchestrates and encodes structural presentation pipeline bindings to draw pending entities.
    pub fn render(&self, view: &wgpu::TextureView) {
        // Halt if zero operational instructions are enqueued across pipelines
        if self.uploaded_point_count == 0 && self.pending_vertices.is_empty() {
            return;
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Chart Main Render Command Encoder Framework"),
        });

        // Compile and write dynamic vertex sequences bound targeting primitive lists execution paths
        let polygon_vertex_buffer = if !self.pending_vertices.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Dynamic Polygon Fan Vertex Buffer Segment"),
                contents: bytemuck::cast_slice(&self.pending_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        } else {
            None
        };

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Chart Visualization Rendering Pass Pipeline Execution Context"),
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

            // ----------------------------------------------------------------
            // Pass Execution Phase 1: Draw Instanced SDF Primitive Quads
            // ----------------------------------------------------------------
            if self.uploaded_point_count > 0 {
                rpass.set_pipeline(&self.sdf_pipeline);
                rpass.set_bind_group(0, self.sdf_group_lookup(), &[]);
                rpass.draw(0..4, 0..self.uploaded_point_count);
            }

            // ----------------------------------------------------------------
            // Pass Execution Phase 2: Draw Tessellated Poly Shapes (Triangle Fans)
            // ----------------------------------------------------------------
            if let Some(ref v_buf) = polygon_vertex_buffer {
                rpass.set_pipeline(&self.poly_pipeline);
                rpass.set_bind_group(0, &self.poly_bind_group, &[]);
                rpass.set_vertex_buffer(0, v_buf.slice(..));
                rpass.draw(0..self.pending_vertices.len() as u32, 0..1);
            }
        }

        // Finalize execution sequences and dispatch contexts down to the hardware core queue
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Mutability safe-clearing operation tracking configurations natively handled externally
        // via standard implementation context state frameworks if necessary.
    }

    /// Internal helper method to acquire safe bind group handles.
    fn sdf_group_lookup(&self) -> &wgpu::BindGroup {
        &self.sdf_bind_group
    }
}