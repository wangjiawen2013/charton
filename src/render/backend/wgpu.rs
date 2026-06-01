//! WGPU rendering backend implementation for 2D primitive rendering (circles, lines, rects, polygons, gradients, text)
//! Provides GPU-optimized data structures and render pipelines aligned with WGSL shaders

use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use crate::visual::color::SingleColor;
use bytemuck::{Pod, Zeroable};
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillVertex, FillVertexConstructor, StrokeOptions, StrokeTessellator,
    StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};
use wgpu::util::DeviceExt;

// ============================================================================
// GPU Data Structures (Strict 1:1 WGSL Alignment - std140 layout)
// ============================================================================

/// Base data for instanced SDF (Signed Distance Field) primitives (matches PointData in WGSL)
/// All fields use f32 for consistent GPU memory alignment
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuPoint {
    /// X coordinate of the primitive center (screen space)
    pub x: f32,
    /// Y coordinate of the primitive center (screen space)
    pub y: f32,
    /// Red color channel (0.0 - 1.0)
    pub r: f32,
    /// Green color channel (0.0 - 1.0)
    pub g: f32,
    /// Blue color channel (0.0 - 1.0)
    pub b: f32,
    /// Alpha transparency channel (0.0 - 1.0)
    pub a: f32,
    /// Radius of the SDF primitive (pixels)
    pub radius: f32,
}

/// GPU data structure for line primitives (matches LineData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuLine {
    /// Start X coordinate (screen space)
    pub x1: f32,
    /// Start Y coordinate (screen space)
    pub y1: f32,
    /// End X coordinate (screen space)
    pub x2: f32,
    /// End Y coordinate (screen space)
    pub y2: f32,
    /// Red color channel (0.0 - 1.0)
    pub r: f32,
    /// Green color channel (0.0 - 1.0)
    pub g: f32,
    /// Blue color channel (0.0 - 1.0)
    pub b: f32,
    /// Alpha transparency channel (0.0 - 1.0)
    pub a: f32,
    /// Line width (pixels)
    pub width: f32,
    /// Manual padding to ensure memory alignment
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

/// GPU data structure for rectangle primitives (matches RectData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuRect {
    /// Top-left X coordinate (screen space)
    pub x: f32,
    /// Top-left Y coordinate (screen space)
    pub y: f32,
    /// Rectangle width (pixels)
    pub width: f32,
    /// Rectangle height (pixels)
    pub height: f32,
    /// Red color channel (0.0 - 1.0)
    pub r: f32,
    /// Green color channel (0.0 - 1.0)
    pub g: f32,
    /// Blue color channel (0.0 - 1.0)
    pub b: f32,
    /// Alpha transparency channel (0.0 - 1.0)
    pub a: f32,
    /// Corner radius for rounded rectangles (pixels)
    pub corner_radius: f32,
}

/// GPU data structure for gradient-filled rectangles (matches GradientRectData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuGradientRect {
    /// Top-left X coordinate (screen space)
    pub x: f32,
    /// Top-left Y coordinate (screen space)
    pub y: f32,
    /// Rectangle width (pixels)
    pub width: f32,
    /// Rectangle height (pixels)
    pub height: f32,
    /// Start gradient red channel (0.0 - 1.0)
    start_r: f32,
    /// Start gradient green channel (0.0 - 1.0)
    start_g: f32,
    /// Start gradient blue channel (0.0 - 1.0)
    start_b: f32,
    /// Start gradient alpha channel (0.0 - 1.0)
    start_a: f32,
    /// End gradient red channel (0.0 - 1.0)
    end_r: f32,
    /// End gradient green channel (0.0 - 1.0)
    end_g: f32,
    /// End gradient blue channel (0.0 - 1.0)
    end_b: f32,
    /// End gradient alpha channel (0.0 - 1.0)
    end_a: f32,
    /// Gradient angle (radians) - 0 = horizontal, π/2 = vertical
    pub angle: f32,
    /// Overall opacity multiplier (0.0 - 1.0)
    pub opacity: f32,
}

/// Vertex data for text rendering (used with glyph atlas)
/// Contains position, texture coordinates, and color for each text vertex
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextVertex {
    /// Screen position (x, y) in pixels
    pub position: [f32; 2],
    /// Texture coordinates (u, v) for glyph atlas sampling
    pub tex_coords: [f32; 2],
    /// Text color (rgba, 0.0 - 1.0)
    pub color: [f32; 4],
}

impl TextVertex {
    /// Vertex buffer layout descriptor for text rendering pipelines
    /// Matches shader input locations (0 = position, 1 = tex_coords, 2 = color)
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
        ],
    };
}

/// Vertex data for path primitives (custom vector paths)
/// Contains position, color, and fill state for each path vertex
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PathVertex {
    /// Path vertex position (x, y) in screen space
    pub position: [f32; 2],
    /// Path color (rgba, 0.0 - 1.0)
    pub color: [f32; 4],
    /// Fill state flag (1.0 = fill path, 0.0 = stroke only)
    pub is_fill: f32,
}

impl PathVertex {
    /// Vertex buffer layout descriptor for path rendering pipelines
    /// Matches shader input locations (0 = position, 1 = color, 2 = is_fill)
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<[f32; 4]>())
                    as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32,
            },
        ],
    };
}

#[derive(Copy, Clone)]
struct PathVertexCtor {
    color: [f32; 4],
    is_fill: f32,
}

impl FillVertexConstructor<PathVertex> for PathVertexCtor {
    fn new_vertex(&mut self, vertex: FillVertex) -> PathVertex {
        PathVertex {
            position: vertex.position().to_array(),
            color: self.color,
            is_fill: self.is_fill,
        }
    }
}

impl StrokeVertexConstructor<PathVertex> for PathVertexCtor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> PathVertex {
        PathVertex {
            position: vertex.position().to_array(),
            color: self.color,
            is_fill: self.is_fill,
        }
    }
}

/// Global uniform data for all shaders (strict std140 alignment)
/// Contains screen dimensions and scaling factors for coordinate normalization
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    /// Current screen width (pixels)
    pub screen_width: f32,
    /// Current screen height (pixels)
    pub screen_height: f32,
    /// UI scale factor (for high-DPI displays)
    pub scale_factor: f32,
    /// Padding to maintain 16-byte alignment (std140 requirement)
    pub _padding: f32,
}

#[derive(Debug, Clone, Copy)]
/// Represents a single rendering batch command in the interleaved queue.
/// This enum acts as the "Instruction Manual" for the renderer, defining
/// which pipeline to use and which data range to fetch from GPU buffers.
pub enum DrawBatch {
    /// Batch of circles to be rendered via instancing.
    /// 'start' is the offset in the circle instance buffer.
    /// 'count' is the number of circle instances to draw in this call.
    Circle {
        start: u32,
        count: u32,
    },
    Rect {
        start: u32,
        count: u32,
    },
    Line {
        start: u32,
        count: u32,
    },
    Polygon {
        index_start: u32,
        index_count: u32,
    },
    GradientRect {
        start: u32,
        count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Helper enum to categorize batch types for matching.
pub enum BatchType {
    Circle,
    Rect,
    Line,
    Polygon,
    GradientRect,
}

// ============================================================================
// WGPU Backend Core Implementation
// ============================================================================

/// WGPU-based rendering backend for 2D primitive rendering
/// Manages GPU resources (pipelines, buffers, bind groups) and handles rendering commands
pub struct WgpuBackend {
    // Core WGPU device and queue
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Global uniform buffer (screen dimensions, scale factor)
    uniform_buffer: wgpu::Buffer,

    // Main bind group (matches @group(0) in chart.wgsl)
    main_bind_group: wgpu::BindGroup,
    main_bind_group_layout: wgpu::BindGroupLayout,

    // Circle primitive resources
    circle_pipeline: wgpu::RenderPipeline,
    circle_buffer: wgpu::Buffer,
    pending_circles: Vec<GpuPoint>,
    uploaded_circle_count: u32,

    // Rectangle primitive resources
    rect_pipeline: wgpu::RenderPipeline,
    rect_buffer: wgpu::Buffer,
    pending_rects: Vec<GpuRect>,
    uploaded_rect_count: u32,

    // Polygon primitive resources
    polygon_pipeline: wgpu::RenderPipeline,
    polygon_vertex_buffer: wgpu::Buffer,
    polygon_index_buffer: wgpu::Buffer,
    pending_polygon_vertices: Vec<PathVertex>,
    pending_polygon_indices: Vec<u16>,
    uploaded_polygon_index_count: u32,

    // Line primitive resources
    line_pipeline: wgpu::RenderPipeline,
    line_buffer: wgpu::Buffer,
    pending_lines: Vec<GpuLine>,
    uploaded_line_count: u32,

    // Path primitive resources
    path_pipeline: wgpu::RenderPipeline,
    path_vertex_buffer: wgpu::Buffer,
    path_index_buffer: wgpu::Buffer,
    pending_path_vertices: Vec<PathVertex>,
    pending_path_indices: Vec<u16>,
    uploaded_path_index_count: u32,

    // Gradient rectangle resources
    gradient_rect_pipeline: wgpu::RenderPipeline,
    gradient_rect_buffer: wgpu::Buffer,
    pending_gradient_rects: Vec<GpuGradientRect>,
    uploaded_gradient_rect_count: u32,

    // Text rendering resources (placeholder implementation)
    text_pipeline: wgpu::RenderPipeline,
    text_vertex_buffer: wgpu::Buffer,
    _text_atlas_texture: wgpu::Texture,
    _text_atlas_view: wgpu::TextureView,
    _text_atlas_sampler: wgpu::Sampler,
    uploaded_text_vertex_count: u32,

    /// Interleaved batch queue to preserve rendering order
    batches: Vec<DrawBatch>,

    /// Running instance count for rendering primitives (used as buffer offset)
    current_circle_count: u32,
    current_rect_count: u32,
    current_line_count: u32,
    current_polygon_index_count: u32,
    current_grad_rect_count: u32,
}

impl WgpuBackend {
    /// Creates a new WGPU rendering backend
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `queue` - WGPU command queue
    /// * `screen_width` - Current screen width in pixels
    /// * `screen_height` - Current screen height in pixels
    /// * `scale_factor` - UI scale factor for high-DPI displays
    ///
    /// # Returns
    /// Initialized WgpuBackend instance
    pub async fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        screen_width: u32,
        screen_height: u32,
        scale_factor: f32,
    ) -> Self {
        // Load WGSL shader module (chart.wgsl contains all primitive shaders)
        let shader = device.create_shader_module(wgpu::include_wgsl!("chart.wgsl"));

        // Initialize global uniform buffer with screen dimensions
        let uniforms = Uniforms {
            screen_width: screen_width as f32,
            screen_height: screen_height as f32,
            scale_factor,
            _padding: 0.0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create main bind group layout (matches @group(0) bindings in chart.wgsl)
        // Bindings:
        // 0: SDF (circle) storage buffer
        // 1: Line storage buffer
        // 2: Rectangle storage buffer
        // 4: Gradient rectangle storage buffer
        // 5: Global uniform buffer
        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Main Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
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

        // Create initial bind group with dummy buffers (replaced in flush)
        let dummy_circles = Self::create_dummy_buffer::<GpuPoint>(&device);
        let dummy_rects = Self::create_dummy_buffer::<GpuRect>(&device);
        let dummy_lines = Self::create_dummy_buffer::<GpuLine>(&device);
        let dummy_grad_rects = Self::create_dummy_buffer::<GpuGradientRect>(&device);
        let dummy_polygon_vertices = Self::create_dummy_buffer::<PathVertex>(&device);
        let dummy_polygon_indices = Self::create_dummy_buffer::<u16>(&device);

        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group"),
            layout: &main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        dummy_circles.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(dummy_lines.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(dummy_rects.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(
                        dummy_grad_rects.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(
                        uniform_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        });

        // Create all render pipelines
        let circle_pipeline =
            Self::create_circle_pipeline(&device, &shader, &main_bind_group_layout);
        let (_line_bg_layout, line_pipeline) =
            Self::create_line_pipeline(&device, &shader, &main_bind_group_layout);
        let rect_pipeline = Self::create_rect_pipeline(&device, &shader, &main_bind_group_layout);
        let polygon_pipeline =
            Self::create_polygon_pipeline(&device, &shader, &main_bind_group_layout);
        let path_pipeline = Self::create_path_pipeline(&device, &shader, &main_bind_group_layout);
        let (_grad_bg_layout, gradient_rect_pipeline) =
            Self::create_gradient_rect_pipeline(&device, &shader, &main_bind_group_layout);
        let (
            _text_bg_layout,
            text_pipeline,
            text_atlas_texture,
            text_atlas_view,
            text_atlas_sampler,
        ) = Self::create_text_pipeline(&device, &shader, &main_bind_group_layout).await;

        // Create placeholder buffers (replaced with actual data in flush)
        let circle_buffer = Self::create_dummy_buffer::<GpuPoint>(&device);
        let rect_buffer = Self::create_dummy_buffer::<GpuRect>(&device);
        let line_buffer = Self::create_dummy_buffer::<GpuLine>(&device);
        let path_vertex_buffer = Self::create_dummy_buffer::<PathVertex>(&device);
        let path_index_buffer = Self::create_dummy_buffer::<u16>(&device);
        let gradient_rect_buffer = Self::create_dummy_buffer::<GpuGradientRect>(&device);
        let text_vertex_buffer = Self::create_dummy_buffer::<TextVertex>(&device);

        Self {
            device,
            queue,
            uniform_buffer,
            main_bind_group,
            main_bind_group_layout,

            circle_pipeline,
            circle_buffer,
            pending_circles: Vec::with_capacity(30_000),
            uploaded_circle_count: 0,

            rect_pipeline,
            rect_buffer,
            pending_rects: Vec::with_capacity(10_000),
            uploaded_rect_count: 0,

            polygon_pipeline,
            polygon_vertex_buffer: dummy_polygon_vertices,
            polygon_index_buffer: dummy_polygon_indices,
            pending_polygon_vertices: Vec::with_capacity(50_000),
            pending_polygon_indices: Vec::with_capacity(100_000),
            uploaded_polygon_index_count: 0,

            line_pipeline,
            line_buffer,
            pending_lines: Vec::with_capacity(10_000),
            uploaded_line_count: 0,

            path_pipeline,
            path_vertex_buffer,
            path_index_buffer,
            pending_path_vertices: Vec::with_capacity(50_000),
            pending_path_indices: Vec::with_capacity(100_000),
            uploaded_path_index_count: 0,

            gradient_rect_pipeline,
            gradient_rect_buffer,
            pending_gradient_rects: Vec::with_capacity(10_000),
            uploaded_gradient_rect_count: 0,

            text_pipeline,
            text_vertex_buffer,
            _text_atlas_texture: text_atlas_texture,
            _text_atlas_view: text_atlas_view,
            _text_atlas_sampler: text_atlas_sampler,
            uploaded_text_vertex_count: 0,

            batches: Vec::with_capacity(1024), // Pre-allocate space for batch queue
            current_circle_count: 0,
            current_rect_count: 0,
            current_line_count: 0,
            current_polygon_index_count: 0,
            current_grad_rect_count: 0,
        }
    }

    fn push_batch(&mut self, batch_type: BatchType, count: u32) {
        // Attempt to merge with the last batch if the type is the same
        match (self.batches.last_mut(), batch_type) {
            (Some(DrawBatch::Circle { count: c, .. }), BatchType::Circle) => *c += count,
            (Some(DrawBatch::Rect { count: c, .. }), BatchType::Rect) => *c += count,
            (Some(DrawBatch::Line { count: c, .. }), BatchType::Line) => *c += count,
            (Some(DrawBatch::Polygon { index_count: c, .. }), BatchType::Polygon) => *c += count,
            (Some(DrawBatch::GradientRect { count: c, .. }), BatchType::GradientRect) => {
                *c += count
            }

            // If types don't match or the queue is empty, create a new batch
            _ => {
                // Here, since BatchType implements Copy, we can safely use it again
                self.batches.push(match batch_type {
                    BatchType::Circle => DrawBatch::Circle {
                        start: self.current_circle_count.saturating_sub(count),
                        count,
                    },
                    BatchType::Rect => DrawBatch::Rect {
                        start: self.current_rect_count.saturating_sub(count),
                        count,
                    },
                    BatchType::Line => DrawBatch::Line {
                        start: self.current_line_count.saturating_sub(count),
                        count,
                    },
                    BatchType::Polygon => DrawBatch::Polygon {
                        index_start: self.current_polygon_index_count.saturating_sub(count),
                        index_count: count,
                    },
                    BatchType::GradientRect => DrawBatch::GradientRect {
                        start: self.current_grad_rect_count.saturating_sub(count),
                        count,
                    },
                });
            }
        }
    }

    /// Clears all state, batches, and buffers to prepare for the next frame.
    /// This should be called at the end of each frame's rendering cycle.
    pub fn reset(&mut self) {
        // 1. Reset the batch queue for the new frame
        self.batches.clear();

        // 2. Reset counters used for generating batch start offsets
        self.current_circle_count = 0;
        self.current_rect_count = 0;
        self.current_line_count = 0;
        self.current_polygon_index_count = 0;
        self.current_grad_rect_count = 0;

        // 3. Clear all pending data buffers (CPU side)
        // These are the buffers that accumulate data via draw_* calls
        self.pending_circles.clear();
        self.pending_rects.clear();
        self.pending_lines.clear();
        self.pending_polygon_vertices.clear();
        self.pending_polygon_indices.clear();
        self.pending_path_vertices.clear();
        self.pending_path_indices.clear();
        self.pending_gradient_rects.clear();

        // Assuming you have a corresponding pending buffer for text
        // self.pending_text_vertices.clear();

        // 4. Reset uploaded counters
        // This is crucial! It tells the system that no data has been uploaded to GPU
        // for the new frame yet.
        self.uploaded_circle_count = 0;
        self.uploaded_rect_count = 0;
        self.uploaded_line_count = 0;
        self.uploaded_polygon_index_count = 0;
        self.uploaded_path_index_count = 0;
        self.uploaded_gradient_rect_count = 0;
        self.uploaded_text_vertex_count = 0;
    }

    // ============================================================================
    // Pipeline Creation Helpers
    // ============================================================================

    /// Creates the circle render pipeline (uses SDF shader for perfect anti-aliasing)
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Circle render pipeline
    fn create_circle_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Circle Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Circle Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("circle_vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("circle_fs"),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Creates the line render pipeline and bind group layout
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Tuple of (bind group layout, render pipeline)
    fn create_line_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        let line_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Line Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("line_vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("line_fs"),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        (line_bind_group_layout, pipeline)
    }

    /// Creates the rectangle render pipeline
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Rectangle render pipeline
    fn create_rect_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("rect_vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("rect_fs"),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Creates the polygon render pipeline (receives CPU-precomputed vertices)
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Polygon render pipeline
    fn create_polygon_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Polygon Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Polygon Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("polygon_vs"),
                buffers: &[PathVertex::DESC],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("polygon_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Creates the path render pipeline
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Path render pipeline
    fn create_path_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Path Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Path Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("path_vs"),
                buffers: &[PathVertex::DESC],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("path_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Creates the gradient rectangle render pipeline and bind group layout
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Tuple of (bind group layout, render pipeline)
    fn create_gradient_rect_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        let gradient_rect_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Gradient Rect Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gradient Rect Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gradient Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("grad_rect_vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("grad_rect_fs"),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        (gradient_rect_bind_group_layout, pipeline)
    }

    /// Creates the text render pipeline and associated resources (placeholder)
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    /// * `shader` - Compiled WGSL shader module
    /// * `_main_layout` - Main bind group layout (@group(0))
    ///
    /// # Returns
    /// Tuple of (bind group layout, pipeline, atlas texture, atlas view, sampler)
    async fn create_text_pipeline(
        device: &wgpu::Device,
        _shader: &wgpu::ShaderModule,
        _main_layout: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
        wgpu::Texture,
        wgpu::TextureView,
        wgpu::Sampler,
    ) {
        // Create 2048x2048 glyph atlas texture (RGBA8 format)
        let atlas_size = (2048u32, 2048u32);
        let text_atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas Texture"),
            size: wgpu::Extent3d {
                width: atlas_size.0,
                height: atlas_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let text_atlas_view =
            text_atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Default sampler for glyph atlas sampling
        let text_atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        // Minimal WGSL shader for text rendering (placeholder implementation)
        let text_wgsl = r#"
struct Uniforms { screen_width: f32, screen_height: f32, scale_factor: f32, _padding: f32 };
@group(0) @binding(5) var<uniform> uniforms: Uniforms;
struct In { 
    @location(0) position: vec2<f32>, 
    @location(1) tex_coords: vec2<f32>, 
    @location(2) color: vec4<f32>, 
};
struct Out { 
    @builtin(position) clip_pos: vec4<f32>, 
    @location(0) color: vec4<f32>, 
};

@vertex fn text_vs(in: In) -> Out {
    // Convert screen space to NDC (Normalized Device Coordinates)
    let sw = uniforms.screen_width * uniforms.scale_factor;
    let sh = uniforms.screen_height * uniforms.scale_factor;
    let ndc = vec4((in.position.x/sw)*2.0-1.0, 1.0-(in.position.y/sh)*2.0, 0.0, 1.0);
    
    var o: Out;
    o.clip_pos = ndc;
    o.color = in.color;
    return o;
}

@fragment fn text_fs(in: Out) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

        // Compile text shader module
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(text_wgsl)),
        });

        // Create text bind group layout (only uniform buffer binding)
        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create text pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(&text_bind_group_layout)],
            immediate_size: 0,
        });

        // Create text render pipeline
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("text_vs"),
                buffers: &[TextVertex::DESC],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("text_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        (
            text_bind_group_layout,
            text_pipeline,
            text_atlas_texture,
            text_atlas_view,
            text_atlas_sampler,
        )
    }

    // ============================================================================
    // Buffer Creation Helpers
    // ============================================================================

    /// Creates a dummy buffer with zero-initialized data (for initial bind group setup)
    ///
    /// # Arguments
    /// * `device` - WGPU device handle
    ///
    /// # Returns
    /// Zero-initialized buffer of type T
    fn create_dummy_buffer<T: Pod>(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("Dummy {} Buffer", std::any::type_name::<T>()).as_str()),
            contents: bytemuck::cast_slice(&[T::zeroed()]),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX,
        })
    }

    /// Creates a GPU buffer from a slice of POD (Plain Old Data) values
    ///
    /// # Arguments
    /// * `data` - Slice of POD values to copy to GPU
    ///
    /// # Returns
    /// WGPU buffer containing the copied data
    fn create_buffer<T: Pod>(&self, data: &[T]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("Updated {} Buffer", std::any::type_name::<T>()).as_str()),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::INDEX,
            })
    }

    fn append_path_vertices(&mut self, buffers: VertexBuffers<PathVertex, u16>) {
        let base_index = self.pending_path_vertices.len() as u16;
        self.pending_path_vertices.extend(buffers.vertices);
        self.pending_path_indices
            .extend(buffers.indices.into_iter().map(|index| index + base_index));
    }

    fn tessellate_path(&mut self, config: PathConfig) {
        if config.points.len() < 2 {
            return;
        }

        let mut path_builder = Path::builder();
        let mut tokens = config.points.into_iter();
        if let Some((x0, y0)) = tokens.next() {
            path_builder.begin(point(x0, y0));
            for (x, y) in tokens {
                path_builder.line_to(point(x, y));
            }
            path_builder.end(false);
        }

        let path = path_builder.build();
        let stroke_color = config.stroke.rgba();
        let color = [
            stroke_color[0],
            stroke_color[1],
            stroke_color[2],
            stroke_color[3] * config.opacity,
        ];

        let mut buffers = VertexBuffers::<PathVertex, u16>::new();
        let mut tessellator = StrokeTessellator::new();
        let stroke_options = StrokeOptions::default().with_line_width(config.stroke_width);
        let _ = tessellator.tessellate_path(
            &path,
            &stroke_options,
            &mut BuffersBuilder::new(
                &mut buffers,
                PathVertexCtor {
                    color,
                    is_fill: 0.0,
                },
            ),
        );

        self.append_path_vertices(buffers);
    }

    // ============================================================================
    // Path & Text Helpers
    // ============================================================================

    /// Processes text config into vertex data (placeholder implementation)
    fn process_text(&mut self) {
        // Empty implementation (to be filled with text layout/rasterization logic)
    }

    // ============================================================================
    // Render & Flush
    // ============================================================================
    /// Flushes pending render data to GPU buffers and renders to the target texture view
    ///
    /// # Arguments
    /// * `view` - Target texture view to render to
    pub fn flush_and_render(&mut self, view: &wgpu::TextureView) {
        // --------------------------------------------------------------------
        // PHASE 1: DATA UPLOAD
        // Transfer all accumulated data from CPU vectors to GPU buffers.
        // This part remains the same because the GPU needs all data in memory
        // before we can execute our batch drawing commands.
        // --------------------------------------------------------------------
        if !self.pending_circles.is_empty() {
            let circles = std::mem::take(&mut self.pending_circles);
            self.circle_buffer = self.create_buffer(&circles);
            self.uploaded_circle_count = circles.len() as u32;
        }

        if !self.pending_rects.is_empty() {
            let rects = std::mem::take(&mut self.pending_rects);
            self.rect_buffer = self.create_buffer(&rects);
            self.uploaded_rect_count = rects.len() as u32;
        }

        if !self.pending_polygon_vertices.is_empty() || !self.pending_polygon_indices.is_empty() {
            let vertices = std::mem::take(&mut self.pending_polygon_vertices);
            let indices = std::mem::take(&mut self.pending_polygon_indices);
            self.polygon_vertex_buffer = self.create_buffer(&vertices);
            self.polygon_index_buffer = self.create_buffer(&indices);
            self.uploaded_polygon_index_count = indices.len() as u32;
        }

        if !self.pending_lines.is_empty() {
            let lines = std::mem::take(&mut self.pending_lines);
            self.line_buffer = self.create_buffer(&lines);
            self.uploaded_line_count = lines.len() as u32;
        }

        if !self.pending_gradient_rects.is_empty() {
            let grad_rects = std::mem::take(&mut self.pending_gradient_rects);
            self.gradient_rect_buffer = self.create_buffer(&grad_rects);
            self.uploaded_gradient_rect_count = grad_rects.len() as u32;
        }

        if !self.pending_path_vertices.is_empty() || !self.pending_path_indices.is_empty() {
            let vertices = std::mem::take(&mut self.pending_path_vertices);
            let indices = std::mem::take(&mut self.pending_path_indices);

            self.path_vertex_buffer = self.create_buffer(&vertices);
            self.path_index_buffer = self.create_buffer(&indices);
            self.uploaded_path_index_count = indices.len() as u32;
        }

        if self.uploaded_text_vertex_count > 0 {
            self.process_text();
        }

        // --------------------------------------------------------------------
        // PHASE 2: BIND GROUP SETUP
        // --------------------------------------------------------------------
        self.main_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group (Updated)"),
            layout: &self.main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        self.circle_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        self.line_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        self.rect_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(
                        self.gradient_rect_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(
                        self.uniform_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        });

        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // --------------------------------------------------------------------
        // PHASE 3: ORCHESTRATED DRAWING (The Magic Happens Here)
        // We now iterate through the `batches` queue. This guarantees that
        // the GPU draws elements in the EXACT order they were submitted by
        // the frontend (Painter's Algorithm), fixing all Z-Index/overlap issues.
        // --------------------------------------------------------------------
        {
            let mut pass = encoder.begin_render_pass(&render_pass_desc);
            pass.set_bind_group(0, &self.main_bind_group, &[]);

            for batch in &self.batches {
                match batch {
                    DrawBatch::Circle { start, count } => {
                        pass.set_pipeline(&self.circle_pipeline);
                        // Draw exactly this batch's slice (instances)
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Line { start, count } => {
                        pass.set_pipeline(&self.line_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Rect { start, count } => {
                        pass.set_pipeline(&self.rect_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::GradientRect { start, count } => {
                        pass.set_pipeline(&self.gradient_rect_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Polygon {
                        index_start,
                        index_count,
                    } => {
                        pass.set_pipeline(&self.polygon_pipeline);
                        pass.set_vertex_buffer(0, self.polygon_vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            self.polygon_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        // Draw exactly this batch's index range
                        pass.draw_indexed(*index_start..(*index_start + *index_count), 0, 0..1);
                    }
                }
            }

            // ----------------------------------------------------------------
            // FALLBACK FOR UN-BATCHED ELEMENTS
            // Since Path and Text might not be integrated into DrawBatch yet,
            // we render them at the very end to ensure they stay visible.
            // (Consider adding them to DrawBatch in the future for full ordering).
            // ----------------------------------------------------------------
            if self.uploaded_path_index_count > 0 {
                pass.set_pipeline(&self.path_pipeline);
                pass.set_vertex_buffer(0, self.path_vertex_buffer.slice(..));
                pass.set_index_buffer(self.path_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..self.uploaded_path_index_count, 0, 0..1);
            }

            if self.uploaded_text_vertex_count > 0 {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                pass.draw(0..self.uploaded_text_vertex_count, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));

        // Clear state for the next frame to prevent data accumulation or rendering artifacts
        self.reset();
    }
}

// ============================================================================
// RenderBackend Trait Implementation
// ============================================================================

impl RenderBackend for WgpuBackend {
    fn draw_circle(&mut self, config: CircleConfig) {
        let fill = config.fill.rgba();
        let point = GpuPoint {
            x: config.x,
            y: config.y,
            r: fill[0],
            g: fill[1],
            b: fill[2],
            a: fill[3] * config.opacity,
            radius: config.radius,
        };

        // 1. Store the circle data into the CPU-side pending buffer
        self.pending_circles.push(point);

        // 2. Increment the counter (used as a reference for calculating the start offset in batching)
        self.current_circle_count += 1;

        // 3. Register the draw command in the batch queue (entry point for batching)
        self.push_batch(BatchType::Circle, 1);
    }

    fn draw_line(&mut self, config: LineConfig) {
        let color = config.color.rgba();
        let line = GpuLine {
            x1: config.x1,
            y1: config.y1,
            x2: config.x2,
            y2: config.y2,
            r: color[0],
            g: color[1],
            b: color[2],
            a: color[3] * config.opacity,
            width: config.width,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        // 1. Store the line data into the CPU-side pending buffer
        self.pending_lines.push(line);

        // 2. Increment the line counter (used as a reference for calculating the start offset)
        self.current_line_count += 1;

        // 3. Register the draw command in the batch queue
        self.push_batch(BatchType::Line, 1);
    }

    fn draw_rect(&mut self, config: RectConfig) {
        let fill = config.fill.rgba();
        let rect = GpuRect {
            x: config.x,
            y: config.y,
            width: config.width,
            height: config.height,
            r: fill[0],
            g: fill[1],
            b: fill[2],
            a: fill[3] * config.opacity,
            corner_radius: 0.0,
        };

        // 1. Store the rect data into the CPU-side pending buffer
        self.pending_rects.push(rect);

        // 2. Increment the rect counter (used as a reference for calculating the start offset)
        self.current_rect_count += 1;

        // 3. Register the draw command in the batch queue
        self.push_batch(BatchType::Rect, 1);
    }

    // ------------------------------
    // Direct vertex shapes (NO TESSELLATE)
    // ------------------------------
    /// Renders a polygon using PRE-COMPUTED vertices directly (no tessellation needed).
    /// This is the most efficient path for regular shapes (triangle, diamond, star, hexagon, etc.)
    /// that are already generated upstream in the PointRenderer.
    ///
    /// - Skips expensive path tessellation/geometry subdivision
    /// - Directly uploads vertices to GPU for maximum performance
    /// - Matches SVG/PNG backend behavior 1:1
    /// - Uses simple triangle fan for convex polygons & stars
    fn draw_polygon(&mut self, config: PolygonConfig) {
        // A valid polygon requires at least 3 vertices. Early exit if invalid.
        if config.points.len() < 3 {
            return;
        }

        // Resolve fill color and apply opacity modulation
        let fill = config.fill.rgba();
        let color = [fill[0], fill[1], fill[2], fill[3] * config.fill_opacity];

        // Triangle fan rendering: use the FIRST vertex as the common origin/fan center
        let base_vertex = self.pending_polygon_vertices.len() as u16;
        let point_count = config.points.len();

        for &(x, y) in &config.points {
            self.pending_polygon_vertices.push(PathVertex {
                position: [x as f32, y as f32],
                color,
                is_fill: 1.0,
            });
        }

        // Generate triangle fan indices
        let mut indices = Vec::new();
        for i in 1..point_count - 1 {
            indices.extend([
                base_vertex,
                base_vertex + i as u16,
                base_vertex + (i + 1) as u16,
            ]);
        }

        // 1. Append finalized indices to pending render buffers
        let index_count = indices.len() as u32;
        self.pending_polygon_indices.extend(indices);

        // 2. Update the counter (track total indices uploaded)
        self.current_polygon_index_count += index_count;

        // 3. Register the polygon batch
        self.push_batch(BatchType::Polygon, index_count);
    }

    fn draw_gradient_rect(&mut self, config: GradientRectConfig) {
        let (start_color, end_color) = match config.stops.as_slice() {
            [] => (SingleColor::none(), SingleColor::none()),
            [(_, color)] => (color.clone(), color.clone()),
            _ => (
                config.stops.first().unwrap().1.clone(),
                config.stops.last().unwrap().1.clone(),
            ),
        };

        let start_rgba = start_color.rgba();
        let end_rgba = end_color.rgba();
        let grad_rect = GpuGradientRect {
            x: config.x,
            y: config.y,
            width: config.width,
            height: config.height,
            start_r: start_rgba[0],
            start_g: start_rgba[1],
            start_b: start_rgba[2],
            start_a: start_rgba[3],
            end_r: end_rgba[0],
            end_g: end_rgba[1],
            end_b: end_rgba[2],
            end_a: end_rgba[3],
            angle: if config.is_vertical {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            },
            opacity: 1.0,
        };

        // 1. Store the gradient rect data into the CPU-side pending buffer
        self.pending_gradient_rects.push(grad_rect);

        // 2. Increment the gradient rect counter
        self.current_grad_rect_count += 1;

        // 3. Register the draw command in the batch queue
        self.push_batch(BatchType::GradientRect, 1);
    }

    fn draw_path(&mut self, config: PathConfig) {
        self.tessellate_path(config);
    }

    fn draw_text(&mut self, _config: TextConfig) {
        // Text rendering is not fully implemented in WGPU yet.
    }
}
