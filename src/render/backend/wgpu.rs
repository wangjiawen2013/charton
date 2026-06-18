//! WGPU rendering backend implementation for 2D primitive rendering
//! Provides GPU-optimized data structures and render pipelines aligned with WGSL shaders
//!
//! Architecture: Two-Tier Bind Group Design
//! - @group(0): Global Environment (Uniforms shared across all pipelines)
//! - @group(1): Isolated Instance Data (Exclusive storage buffers per pipeline)

use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ============================================================================
// GPU Data Structures (Strict 1:1 WGSL Alignment - std140 layout)
// ============================================================================

/// Base data for instanced SDF primitives (matches PointData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuPoint {
    pub x: f32,
    pub y: f32,
    pub fill_r: f32,
    pub fill_g: f32,
    pub fill_b: f32,
    pub fill_a: f32,
    pub stroke_r: f32,
    pub stroke_g: f32,
    pub stroke_b: f32,
    pub stroke_a: f32,
    pub radius: f32,
    pub stroke_width: f32,
}

/// GPU data structure for rectangle primitives (matches RectData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill_r: f32,
    pub fill_g: f32,
    pub fill_b: f32,
    pub fill_a: f32,
    pub stroke_r: f32,
    pub stroke_g: f32,
    pub stroke_b: f32,
    pub stroke_a: f32,
    pub stroke_width: f32,
    pub corner_radius: f32,
}

/// GPU data structure for line primitives (matches LineData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuLine {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub width: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

/// GPU data structure for gradient-filled rectangles (matches GradientRectData in WGSL)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuGradientRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    start_r: f32,
    start_g: f32,
    start_b: f32,
    start_a: f32,
    end_r: f32,
    end_g: f32,
    end_b: f32,
    end_a: f32,
    pub angle: f32,
    pub opacity: f32,
}

// ----------------------------------------------------------------------------
// Pure GPU Polyline Extrusion Layouts (Path Stream)
// ----------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPathPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPathStyle {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub thickness: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPathArgs {
    pub start_point_idx: u32,
    pub style_idx: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

/// Vertex data for polygon primitives (matches CPU-generated stream)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PathVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub is_fill: f32,
}

impl PathVertex {
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

/// Global uniform data for all shaders (strict std140 alignment)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub screen_width: f32,
    pub screen_height: f32,
    pub scale_factor: f32,
    pub _padding: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum DrawBatch {
    Circle { start: u32, count: u32 },
    Rect { start: u32, count: u32 },
    Line { start: u32, count: u32 },
    Polygon { index_start: u32, index_count: u32 },
    GradientRect { start: u32, count: u32 },
    PathSimple { path_idx: u32, point_count: u32 },
    PathComplexStencil { index_start: u32, index_count: u32 },
    PathComplexCover { index_start: u32, index_count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchType {
    Circle,
    Rect,
    Line,
    Polygon,
    GradientRect,
    PathComplexStencil,
    PathComplexCover,
}

// ============================================================================
// WGPU Backend Core Implementation
// ============================================================================

pub struct WgpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Global uniform buffer
    uniform_buffer: wgpu::Buffer,

    // Two-Tier Architecture Bind Group Definitions
    global_bind_group: wgpu::BindGroup,
    global_bind_group_layout: wgpu::BindGroupLayout,
    instance_bind_group_layout: wgpu::BindGroupLayout,

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

    // Line primitive resources
    line_pipeline: wgpu::RenderPipeline,
    line_buffer: wgpu::Buffer,
    pending_lines: Vec<GpuLine>,
    uploaded_line_count: u32,

    // Polygon primitive resources
    polygon_pipeline: wgpu::RenderPipeline,
    complex_stencil_pipeline: wgpu::RenderPipeline, // For irregular polygons using stencil buffering
    complex_cover_pipeline: wgpu::RenderPipeline, // For irregular polygons using cover pass after stencil masking
    polygon_vertex_buffer: wgpu::Buffer,
    polygon_index_buffer: wgpu::Buffer,
    pending_polygon_vertices: Vec<PathVertex>,
    pending_polygon_indices: Vec<u16>,
    uploaded_polygon_index_count: u32,

    // Gradient rectangle resources
    gradient_rect_pipeline: wgpu::RenderPipeline,
    gradient_rect_buffer: wgpu::Buffer,
    pending_gradient_rects: Vec<GpuGradientRect>,
    uploaded_gradient_rect_count: u32,

    // Path primitive resources
    path_simple_pipeline: wgpu::RenderPipeline,
    path_bind_group_layout: wgpu::BindGroupLayout,
    pending_path_points: Vec<GpuPathPoint>,
    pending_path_styles: Vec<GpuPathStyle>,
    pending_path_args: Vec<GpuPathArgs>,

    pub collected_texts: Vec<TextConfig>,
    // A tuple containing the batch and its associated scissor state.
    #[allow(clippy::type_complexity)]
    batches: Vec<(DrawBatch, Option<(u32, u32, u32, u32)>)>,
    // Add state tracking for physical dimensions and current clipping rect.
    current_scissor: Option<(u32, u32, u32, u32)>,
    // Physical width of the rendering surface in device pixels (logical_width * scale_factor).
    // Used for scissor rect clamping and full-screen restore operations.
    physical_width: u32,
    // Physical height of the rendering surface in device pixels (logical_height * scale_factor).
    // Used for scissor rect clamping and full-screen restore operations.
    physical_height: u32,
    scale_factor: f32,

    current_circle_count: u32,
    current_rect_count: u32,
    current_line_count: u32,
    current_polygon_index_count: u32,
    current_grad_rect_count: u32,

    // Invisible mask canvas used by the GPU to calculate complex polygon fills.
    depth_stencil_view: wgpu::TextureView,
}

impl WgpuBackend {
    pub async fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        screen_width: u32,
        screen_height: u32,
        scale_factor: f32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("chart.wgsl"));

        let uniforms = Uniforms {
            screen_width: screen_width as f32,
            screen_height: screen_height as f32,
            scale_factor,
            _padding: 0.0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer - Group 0"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ====================================================================
        // RESOURCE BIND GROUP LAYOUT DECLARATIONS (Scientific Architecture)
        // ====================================================================

        // Group 0: Global Environment (Uniforms)
        let global_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global Environment Bind Group Layout 0"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Group 1: Universal Instance Data (Circles, Rects, Lines, Gradients)
        let instance_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Universal Instance Bind Group Layout 1"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Path stream layout (Group 1 with 3 decoupled slots)
        let path_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Path Storage Bind Group Layout 1"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create the initial global bind group state
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global Environment Bind Group 0"),
            layout: &global_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
            }],
        });

        // ====================================================================
        // RENDER PIPELINES COMPILATION
        // ====================================================================
        let circle_pipeline = Self::create_circle_pipeline(
            &device,
            &shader,
            &global_bind_group_layout,
            &instance_bind_group_layout,
        );
        let line_pipeline = Self::create_line_pipeline(
            &device,
            &shader,
            &global_bind_group_layout,
            &instance_bind_group_layout,
        );
        let rect_pipeline = Self::create_rect_pipeline(
            &device,
            &shader,
            &global_bind_group_layout,
            &instance_bind_group_layout,
        );
        let polygon_pipeline = Self::create_polygon_pipeline(
            &device,
            &shader,
            &global_bind_group_layout, // Polygon only needs Group 0
        );
        let gradient_rect_pipeline = Self::create_gradient_rect_pipeline(
            &device,
            &shader,
            &global_bind_group_layout,
            &instance_bind_group_layout,
        );

        // Path Simple Pipeline
        let path_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Path Simple Pipeline Layout"),
            bind_group_layouts: &[
                Some(&global_bind_group_layout),
                Some(&path_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let path_simple_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Path Hardware Extrusion Pipeline"),
            layout: Some(&path_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("path_simple_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("path_simple_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            // Match pass format for compatibility, but bypass actual testing for non-polygon pipelines
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Complex Path Pipeline
        // Shared pipeline layout utilizing only global environment uniforms
        let complex_path_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Complex Path Pipeline Layout"),
                bind_group_layouts: &[Some(&global_bind_group_layout)],
                immediate_size: 0,
            });

        // PASS 1: STENCIL PIPELINE - Generates the polygon mask via odd-even inversion (Color write disabled)
        let complex_stencil_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Complex Path - Stencil Pass Pipeline"),
                layout: Some(&complex_path_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("polygon_vs"),
                    buffers: &[PathVertex::DESC],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("polygon_fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::empty(), // No color writes completely for the stencil pass
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState {
                        // Invert bits on pass to natively resolve self-intersections and concavity
                        front: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Invert,
                        },
                        back: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Invert,
                        },
                        read_mask: 0xFF,
                        write_mask: 0xFF,
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        // PASS 2: COVER PIPELINE - Rasterizes the bounding color quad, filling and auto-clearing the masked region
        let complex_cover_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Complex Path - Cover Pass Pipeline"),
                layout: Some(&complex_path_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("polygon_vs"),
                    buffers: &[PathVertex::DESC],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("polygon_fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL, // Restore color channel writing
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState {
                        // Render color where stencil is non-zero, resetting it to zero simultaneously
                        front: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::NotEqual,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Zero,
                        },
                        back: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::NotEqual,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Zero,
                        },
                        read_mask: 0xFF,
                        write_mask: 0xFF,
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        // Dummy buffers for initialization
        let circle_buffer = Self::create_dummy_buffer::<GpuPoint>(&device);
        let rect_buffer = Self::create_dummy_buffer::<GpuRect>(&device);
        let line_buffer = Self::create_dummy_buffer::<GpuLine>(&device);
        let gradient_rect_buffer = Self::create_dummy_buffer::<GpuGradientRect>(&device);
        let dummy_polygon_vertices = Self::create_dummy_buffer::<PathVertex>(&device);
        let dummy_polygon_indices = Self::create_dummy_buffer::<u16>(&device);

        // STENCIL MASK BUFFER FOR HETEROGENEOUS VECTOR FILLS
        let physical_width = (screen_width as f32 * scale_factor).round() as u32;
        let physical_height = (screen_height as f32 * scale_factor).round() as u32;
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("2D Vector Stencil Texture"),
            size: wgpu::Extent3d {
                width: physical_width,
                height: physical_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Invisible mask canvas used by the GPU to calculate complex polygon fills.
        let depth_stencil_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            device,
            queue,
            uniform_buffer,
            global_bind_group,
            global_bind_group_layout,
            instance_bind_group_layout,

            circle_pipeline,
            circle_buffer,
            pending_circles: Vec::with_capacity(30_000),
            uploaded_circle_count: 0,

            rect_pipeline,
            rect_buffer,
            pending_rects: Vec::with_capacity(10_000),
            uploaded_rect_count: 0,

            line_pipeline,
            line_buffer,
            pending_lines: Vec::with_capacity(10_000),
            uploaded_line_count: 0,

            polygon_pipeline,
            complex_stencil_pipeline,
            complex_cover_pipeline,
            polygon_vertex_buffer: dummy_polygon_vertices,
            polygon_index_buffer: dummy_polygon_indices,
            pending_polygon_vertices: Vec::with_capacity(50_000),
            pending_polygon_indices: Vec::with_capacity(100_000),
            uploaded_polygon_index_count: 0,

            gradient_rect_pipeline,
            gradient_rect_buffer,
            pending_gradient_rects: Vec::with_capacity(10_000),
            uploaded_gradient_rect_count: 0,

            path_simple_pipeline,
            path_bind_group_layout,
            pending_path_points: Vec::with_capacity(100_000),
            pending_path_styles: Vec::with_capacity(1024),
            pending_path_args: Vec::with_capacity(1024),

            collected_texts: Vec::new(),
            batches: Vec::with_capacity(1024),
            current_scissor: None,
            physical_width,
            physical_height,
            scale_factor,

            current_circle_count: 0,
            current_rect_count: 0,
            current_line_count: 0,
            current_polygon_index_count: 0,
            current_grad_rect_count: 0,

            depth_stencil_view,
        }
    }

    /// Pushes a new draw command or merges it into the active batch to optimize GPU draw calls.
    fn push_batch(&mut self, batch_type: BatchType, count: u32) {
        // BATCH MERGING: Only merge if the primitive type AND the scissor clipping state are identical.
        let can_merge = self
            .batches
            .last()
            .is_some_and(|(_, scissor)| *scissor == self.current_scissor);

        if can_merge
            && let Some((last_batch, _)) = self.batches.last_mut() {
                match (last_batch, batch_type) {
                    (DrawBatch::Circle { count: c, .. }, BatchType::Circle) => {
                        *c += count;
                        return;
                    }
                    (DrawBatch::Rect { count: c, .. }, BatchType::Rect) => {
                        *c += count;
                        return;
                    }
                    (DrawBatch::Line { count: c, .. }, BatchType::Line) => {
                        *c += count;
                        return;
                    }
                    (DrawBatch::Polygon { index_count: c, .. }, BatchType::Polygon) => {
                        *c += count;
                        return;
                    }
                    (DrawBatch::GradientRect { count: c, .. }, BatchType::GradientRect) => {
                        *c += count;
                        return;
                    }
                    (
                        DrawBatch::PathComplexStencil { index_count: c, .. },
                        BatchType::PathComplexStencil,
                    ) => {
                        *c += count;
                        return;
                    }
                    (
                        DrawBatch::PathComplexCover { index_count: c, .. },
                        BatchType::PathComplexCover,
                    ) => {
                        *c += count;
                        return;
                    }
                    _ => {} // PathSimple and other types fall through to isolation
                }
        }

        // FALLBACK / ISOLATION: Create a new draw batch paired with the current active scissor state.
        let new_batch = match batch_type {
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
            BatchType::PathComplexStencil => DrawBatch::PathComplexStencil {
                index_start: self.current_polygon_index_count.saturating_sub(count),
                index_count: count,
            },
            BatchType::PathComplexCover => DrawBatch::PathComplexCover {
                index_start: self.current_polygon_index_count.saturating_sub(count),
                index_count: count,
            },
        };

        self.batches.push((new_batch, self.current_scissor));
    }

    pub fn reset(&mut self) {
        self.batches.clear();
        self.current_circle_count = 0;
        self.current_rect_count = 0;
        self.current_line_count = 0;
        self.current_polygon_index_count = 0;
        self.current_grad_rect_count = 0;

        self.pending_circles.clear();
        self.pending_rects.clear();
        self.pending_lines.clear();
        self.pending_polygon_vertices.clear();
        self.pending_polygon_indices.clear();
        self.pending_gradient_rects.clear();
        self.pending_path_points.clear();
        self.pending_path_styles.clear();
        self.pending_path_args.clear();

        self.uploaded_circle_count = 0;
        self.uploaded_rect_count = 0;
        self.uploaded_line_count = 0;
        self.uploaded_polygon_index_count = 0;
        self.uploaded_gradient_rect_count = 0;

        self.collected_texts.clear();
    }

    // ============================================================================
    // Pipeline Creation Helpers
    // ============================================================================

    fn create_circle_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        global_layout: &wgpu::BindGroupLayout,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Circle Pipeline Layout"),
            bind_group_layouts: &[Some(global_layout), Some(instance_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Circle Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("circle_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("circle_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_line_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        global_layout: &wgpu::BindGroupLayout,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[Some(global_layout), Some(instance_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("line_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("line_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_rect_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        global_layout: &wgpu::BindGroupLayout,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[Some(global_layout), Some(instance_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("rect_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("rect_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_polygon_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        global_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Polygon Pipeline Layout"),
            bind_group_layouts: &[Some(global_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Polygon Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("polygon_vs"),
                buffers: &[PathVertex::DESC],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("polygon_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_gradient_rect_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        global_layout: &wgpu::BindGroupLayout,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gradient Rect Pipeline Layout"),
            bind_group_layouts: &[Some(global_layout), Some(instance_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gradient Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("grad_rect_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("grad_rect_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

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

    /// Renders a stroked path using GPU extrusion (simple open/closed polylines).
    fn stroke_path(&mut self, config: PathConfig) {
        if config.points.len() < 2 {
            return;
        }

        let start_point_idx = self.pending_path_points.len() as u32;
        let point_count = config.points.len() as u32;
        let style_idx = self.pending_path_styles.len() as u32;
        let path_idx = self.pending_path_args.len() as u32;

        for &(x, y) in &config.points {
            self.pending_path_points.push(GpuPathPoint { x, y });
        }

        let stroke_color = config.stroke.rgba();
        self.pending_path_styles.push(GpuPathStyle {
            r: stroke_color[0],
            g: stroke_color[1],
            b: stroke_color[2],
            a: stroke_color[3] * config.opacity,
            thickness: config.stroke_width,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        });

        self.pending_path_args.push(GpuPathArgs {
            start_point_idx,
            style_idx,
            _pad0: 0,
            _pad1: 0,
        });

        self.batches.push((
            DrawBatch::PathSimple {
                path_idx,
                point_count,
            },
            self.current_scissor,
        ));
    }

    /// Complex Path (Stencil-then-Cover universal concave polygon filler)
    fn fill_path_complex(&mut self, config: PathConfig) {
        if config.points.len() < 3 {
            return;
        }

        let point_count = config.points.len();
        let base_vertex = self.pending_polygon_vertices.len() as u16;

        // ====================================================================
        // PASS 1: STENCIL (Triangle Fan with Odd-Even Winding)
        // ====================================================================
        // Push all vertices of the complex polygon. Color is irrelevant as
        // this pass disables color writes.
        for &(x, y) in &config.points {
            self.pending_polygon_vertices.push(PathVertex {
                position: [x, y],
                color: [0.0, 0.0, 0.0, 0.0],
                is_fill: 1.0,
            });
        }

        // Generate indices using a Triangle Fan topology (anchor at the first vertex)
        let mut stencil_indices = Vec::new();
        for i in 1..point_count - 1 {
            stencil_indices.extend([
                base_vertex, // The first vertex, index 0
                base_vertex + i as u16,
                base_vertex + (i + 1) as u16,
            ]);
        }

        let stencil_index_count = stencil_indices.len() as u32;
        self.pending_polygon_indices.extend(stencil_indices);
        self.current_polygon_index_count += stencil_index_count;

        // Dispatch to the dedicated Stencil pipeline batch
        self.push_batch(BatchType::PathComplexStencil, stencil_index_count);

        // ====================================================================
        // PASS 2: COVER (Bounding Box Fill)
        // ====================================================================
        // Calculate the bounding box to minimize the GPU pixel fill area
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(x, y) in &config.points {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        let fill = config.fill.rgba();
        let fill_color = [fill[0], fill[1], fill[2], fill[3] * config.opacity];
        let cover_base = self.pending_polygon_vertices.len() as u16;

        // Generate a quad covering the bounding box
        let bb_points = [
            (min_x, min_y),
            (max_x, min_y),
            (max_x, max_y),
            (min_x, max_y),
        ];

        for &(x, y) in &bb_points {
            self.pending_polygon_vertices.push(PathVertex {
                position: [x, y],
                color: fill_color,
                is_fill: 1.0,
            });
        }

        let cover_indices = vec![
            cover_base,
            cover_base + 1,
            cover_base + 2,
            cover_base,
            cover_base + 2,
            cover_base + 3,
        ];

        let cover_index_count = cover_indices.len() as u32;
        self.pending_polygon_indices.extend(cover_indices);
        self.current_polygon_index_count += cover_index_count;

        // Dispatch to the dedicated Cover pipeline batch (renders where stencil != 0)
        self.push_batch(BatchType::PathComplexCover, cover_index_count);
    }

    // ============================================================================
    // Render & Flush
    // ============================================================================
    pub fn flush_and_render(
        &mut self,
        view: &wgpu::TextureView, // Required to bind the hardware stencil attachment
        output_ledger: &mut Vec<TextConfig>,
    ) {
        // --------------------------------------------------------------------
        // PHASE 1: GPU DATA UPLOAD (Host-to-Device Memory Transfer)
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

        if !self.pending_lines.is_empty() {
            let lines = std::mem::take(&mut self.pending_lines);
            self.line_buffer = self.create_buffer(&lines);
            self.uploaded_line_count = lines.len() as u32;
        }

        // Shared buffer allocation for both standard polygons and complex path geometry
        if !self.pending_polygon_vertices.is_empty() || !self.pending_polygon_indices.is_empty() {
            let vertices = std::mem::take(&mut self.pending_polygon_vertices);
            let indices = std::mem::take(&mut self.pending_polygon_indices);
            self.polygon_vertex_buffer = self.create_buffer(&vertices);
            self.polygon_index_buffer = self.create_buffer(&indices);
            self.uploaded_polygon_index_count = indices.len() as u32;
        }

        if !self.pending_gradient_rects.is_empty() {
            let grad_rects = std::mem::take(&mut self.pending_gradient_rects);
            self.gradient_rect_buffer = self.create_buffer(&grad_rects);
            self.uploaded_gradient_rect_count = grad_rects.len() as u32;
        }

        // Generate the structural bind group for uniform-based vector paths if data exists
        let path_bind_group = if !self.pending_path_points.is_empty() {
            let points_buf = self.create_buffer(&self.pending_path_points);
            let styles_buf = self.create_buffer(&self.pending_path_styles);
            let args_buf = self.create_buffer(&self.pending_path_args);

            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Path Bind Group 1"),
                layout: &self.path_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            points_buf.as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(
                            styles_buf.as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(
                            args_buf.as_entire_buffer_binding(),
                        ),
                    },
                ],
            }))
        } else {
            None
        };

        // --------------------------------------------------------------------
        // PHASE 2: BIND GROUP SETUP (Global and Instanced Storage)
        // --------------------------------------------------------------------

        self.global_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global Bind Group 0"),
            layout: &self.global_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    self.uniform_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let circle_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Circle Instance Group 1"),
            layout: &self.instance_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    self.circle_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let rect_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rect Instance Group 1"),
            layout: &self.instance_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    self.rect_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let line_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Line Instance Group 1"),
            layout: &self.instance_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    self.line_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let grad_rect_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gradient Rect Instance Group 1"),
            layout: &self.instance_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    self.gradient_rect_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        // Main descriptor incorporating the mandatory stencil target attachment
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
            // Binds depth_stencil and clears the stencil buffer to 0 at the start of the frame
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
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
        // PHASE 3: ORCHESTRATED DRAWING (Multi-Group Context Switching)
        // --------------------------------------------------------------------
        {
            let mut pass = encoder.begin_render_pass(&render_pass_desc);

            // Bind global environment once for the entire pass
            pass.set_bind_group(0, &self.global_bind_group, &[]);

            // Tracks the current scissor state on the GPU to minimize redundant commands
            let mut active_scissor: Option<(u32, u32, u32, u32)> = None;

            for (batch, scissor) in &self.batches {
                // Dynamically update GPU hardware scissor rect if the domain changed
                if *scissor != active_scissor {
                    if let Some((x, y, w, h)) = scissor {
                        pass.set_scissor_rect(*x, *y, *w, *h);
                    } else {
                        // Restore to full-screen viewport when no clipping is active
                        pass.set_scissor_rect(0, 0, self.physical_width, self.physical_height);
                    }
                    active_scissor = *scissor;
                }

                match batch {
                    DrawBatch::Circle { start, count } => {
                        pass.set_pipeline(&self.circle_pipeline);
                        pass.set_bind_group(1, &circle_bind_group, &[]);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Rect { start, count } => {
                        pass.set_pipeline(&self.rect_pipeline);
                        pass.set_bind_group(1, &rect_bind_group, &[]);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Line { start, count } => {
                        pass.set_pipeline(&self.line_pipeline);
                        pass.set_bind_group(1, &line_bind_group, &[]);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::GradientRect { start, count } => {
                        pass.set_pipeline(&self.gradient_rect_pipeline);
                        pass.set_bind_group(1, &grad_rect_bind_group, &[]);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Polygon {
                        index_start,
                        index_count,
                    } => {
                        pass.set_pipeline(&self.polygon_pipeline);
                        // Polygon does not rely on Group 1 instance storage
                        pass.set_vertex_buffer(0, self.polygon_vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            self.polygon_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        pass.draw_indexed(*index_start..(*index_start + *index_count), 0, 0..1);
                    }
                    DrawBatch::PathSimple {
                        path_idx,
                        point_count,
                    } => {
                        if let Some(global_path_bg) = &path_bind_group {
                            pass.set_pipeline(&self.path_simple_pipeline);
                            pass.set_bind_group(1, global_path_bg, &[]);

                            let virtual_vertex_count = (*point_count - 1) * 6;
                            pass.draw(0..virtual_vertex_count, *path_idx..(*path_idx + 1));
                        }
                    }
                    // COMPONENT 1: Stencil Pass - Carves the winding topology into the stencil target
                    DrawBatch::PathComplexStencil {
                        index_start,
                        index_count,
                    } => {
                        pass.set_pipeline(&self.complex_stencil_pipeline);
                        pass.set_vertex_buffer(0, self.polygon_vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            self.polygon_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        pass.set_stencil_reference(0); // Sets baseline comparison reference
                        pass.draw_indexed(*index_start..(*index_start + *index_count), 0, 0..1);
                    }
                    // COMPONENT 2: Cover Pass - Fills pixels matching the mask, resetting the stencil to 0 inline
                    DrawBatch::PathComplexCover {
                        index_start,
                        index_count,
                    } => {
                        pass.set_pipeline(&self.complex_cover_pipeline);
                        pass.set_vertex_buffer(0, self.polygon_vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            self.polygon_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        pass.set_stencil_reference(0);
                        pass.draw_indexed(*index_start..(*index_start + *index_count), 0, 0..1);
                    }
                }
            }
        }

        // Submit operational command buffer stream to the GPU graphics queue
        self.queue.submit(Some(encoder.finish()));

        // Offload UI text configurations for external composition passes
        output_ledger.clear();
        output_ledger.append(&mut self.collected_texts);

        // Reset temporary buffers and batch lists for the next rendering cycle
        self.reset();
    }
}

// ============================================================================
// RenderBackend Trait Implementation
// ============================================================================

impl RenderBackend for WgpuBackend {
    fn begin_clip_scope(&mut self, rect: &crate::coordinate::Rect) {
        // Convert virtual logical coordinates into physical pixels
        let physical_x = ((rect.x as f32) * self.scale_factor).max(0.0) as u32;
        let physical_y = ((rect.y as f32) * self.scale_factor).max(0.0) as u32;
        let physical_w = ((rect.width as f32) * self.scale_factor).max(1.0) as u32;
        let physical_h = ((rect.height as f32) * self.scale_factor).max(1.0) as u32;

        // Clamp tightly to the safe physical boundaries to prevent WGPU panic/crashes
        let x = physical_x.min(self.physical_width);
        let y = physical_y.min(self.physical_height);
        let w = physical_w.min(self.physical_width.saturating_sub(x)).max(1);
        let h = physical_h
            .min(self.physical_height.saturating_sub(y))
            .max(1);

        self.current_scissor = Some((x, y, w, h));
    }

    fn end_clip_scope(&mut self) {
        // Reset the state to full viewport rendering
        self.current_scissor = None;
    }

    fn draw_circle(&mut self, config: CircleConfig) {
        let fill = config.fill.rgba();
        let stroke = config.stroke.rgba();

        let point = GpuPoint {
            x: config.x,
            y: config.y,
            fill_r: fill[0],
            fill_g: fill[1],
            fill_b: fill[2],
            fill_a: fill[3] * config.opacity,
            stroke_r: stroke[0],
            stroke_g: stroke[1],
            stroke_b: stroke[2],
            stroke_a: stroke[3],
            radius: config.radius,
            stroke_width: config.stroke_width,
        };

        self.pending_circles.push(point);
        self.current_circle_count += 1;
        self.push_batch(BatchType::Circle, 1);
    }

    fn draw_rect(&mut self, config: RectConfig) {
        let fill = config.fill.rgba();
        let stroke = config.stroke.rgba();

        let rect = GpuRect {
            x: config.x,
            y: config.y,
            width: config.width,
            height: config.height,
            fill_r: fill[0],
            fill_g: fill[1],
            fill_b: fill[2],
            fill_a: fill[3] * config.opacity,
            stroke_r: stroke[0],
            stroke_g: stroke[1],
            stroke_b: stroke[2],
            stroke_a: stroke[3],
            stroke_width: config.stroke_width,
            corner_radius: 0.0,
        };

        self.pending_rects.push(rect);
        self.current_rect_count += 1;
        self.push_batch(BatchType::Rect, 1);
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

        self.pending_lines.push(line);
        self.current_line_count += 1;
        self.push_batch(BatchType::Line, 1);
    }

    /// Draws a fully enclosed polygon primitive by sequentially generating its interior fill mesh and boundary stroke.
    fn draw_polygon(&mut self, config: PolygonConfig) {
        // A valid polygon requires at least 3 vertices to form an enclosed area
        if config.points.len() < 3 {
            return;
        }

        // ====================================================================
        // LAYER 1: Fill Phase - Tessellation via Triangle Fan
        // ====================================================================
        // Only process fill geometry if the alpha channel is greater than zero
        if config.fill.rgba()[3] > 0.0 {
            let fill = config.fill.rgba();
            // Premultiply the layer's global opacity with the fill color's alpha channel
            let color = [fill[0], fill[1], fill[2], fill[3] * config.opacity];

            // Record the starting index in the vertex buffer to calculate local index offsets
            let base_vertex = self.pending_polygon_vertices.len() as u16;
            let point_count = config.points.len();

            // Push polygon coordinates into the vertex buffer
            for &(x, y) in &config.points {
                self.pending_polygon_vertices.push(PathVertex {
                    position: [x, y],
                    color,
                    is_fill: 1.0, // Flag indicating this vertex belongs to a fill mesh
                });
            }

            // Generate Index Buffer using a Triangle Fan topology (suitable for convex polygons)
            let mut indices = Vec::new();
            for i in 1..point_count - 1 {
                indices.extend([
                    base_vertex,
                    base_vertex + i as u16,
                    base_vertex + (i + 1) as u16,
                ]);
            }

            let index_count = indices.len() as u32;
            self.pending_polygon_indices.extend(indices);
            self.current_polygon_index_count += index_count;

            // Dispatch a draw batch for the fill geometry
            self.push_batch(BatchType::Polygon, index_count);
        }

        // ====================================================================
        // LAYER 2: Stroke Phase & Auto-AA Fringe Generation
        // ====================================================================
        // Evaluate geometric rendering requirements based on style attributes
        let has_stroke = config.stroke_width > 0.0 && config.stroke.rgba()[3] > 0.0;
        let has_fill = config.fill.rgba()[3] > 0.0;

        // An edge pass is required if a custom stroke is defined, or if a fill
        // exists and needs an anti-aliasing perimeter to prevent stencil aliasing (jagged edges).
        if has_stroke || has_fill {
            let mut closed_points = config.points.clone();

            // The underlying path extrusion pipeline processes inputs as open
            // polylines. To render a proper closed polygon loop, we duplicate and append the
            // first vertex to the end of the collection, forcing the final segment to tie back to the start.
            if let Some(&first_pt) = config.points.first() {
                closed_points.push(first_pt);
            }

            if has_stroke {
                // Scenario A: Standard User-Defined Stroke
                // Dispatches a dedicated wireframe mesh generation pass using the assigned stroke configuration.
                self.stroke_path(PathConfig {
                    points: closed_points,
                    fill: crate::visual::color::SingleColor::none(), // Prevent redundant stencil operations
                    stroke: config.stroke,
                    stroke_width: config.stroke_width,
                    opacity: config.opacity,
                    dash: vec![], // Polygons implicitly utilize solid stroke patterns
                    topology: PathTopology::Simple,
                });
            } else {
                // Scenario B: Auto-AA Fringe (Automatic Edge Feathering)
                // When a polygon has a solid fill but no outline, the standard binary stencil-then-cover
                // rasterization leaves hard, jagged pixels.
                //
                // THE TRICK: We injected a ghost stroke with a physical width of 0.0, matching the fill color.
                // Guided by the shader's internal 1.5px screen-space padding and SDF-driven alpha-stepping,
                // this generates a sub-pixel anti-aliasing gradient that perfectly smooths out the raw mesh boundaries.
                self.stroke_path(PathConfig {
                    points: closed_points,
                    fill: crate::visual::color::SingleColor::none(),
                    stroke: config.fill, // Fallback to fill color for seamless edge integration
                    stroke_width: 0.0, // Zero physical width; relies entirely on the shader's AA padding
                    opacity: config.opacity,
                    dash: vec![],
                    topology: PathTopology::Simple,
                });
            }
        }
    }

    fn draw_gradient_rect(&mut self, config: GradientRectConfig) {
        if config.stops.is_empty() {
            return;
        }

        if config.stops.len() == 1 {
            let start_rgba = config.stops[0].1.rgba();
            let grad_rect = GpuGradientRect {
                x: config.x,
                y: config.y,
                width: config.width,
                height: config.height,
                start_r: start_rgba[0],
                start_g: start_rgba[1],
                start_b: start_rgba[2],
                start_a: start_rgba[3],
                end_r: start_rgba[0],
                end_g: start_rgba[1],
                end_b: start_rgba[2],
                end_a: start_rgba[3],
                angle: 0.0,
                opacity: 1.0,
            };
            self.pending_gradient_rects.push(grad_rect);
            self.current_grad_rect_count += 1;
            self.push_batch(BatchType::GradientRect, 1);
            return;
        }

        let mut count = 0;
        for window in config.stops.windows(2) {
            let (offset1, color1) = &window[0];
            let (offset2, color2) = &window[1];

            let (sub_x, sub_y, sub_width, sub_height) = if config.is_vertical {
                (
                    config.x,
                    config.y + offset1 * config.height,
                    config.width,
                    (offset2 - offset1) * config.height,
                )
            } else {
                (
                    config.x + offset1 * config.width,
                    config.y,
                    (offset2 - offset1) * config.width,
                    config.height,
                )
            };

            let start_rgba = color1.rgba();
            let end_rgba = color2.rgba();

            let grad_rect = GpuGradientRect {
                x: sub_x,
                y: sub_y,
                width: sub_width,
                height: sub_height,
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

            self.pending_gradient_rects.push(grad_rect);
            count += 1;
        }

        self.current_grad_rect_count += count;
        self.push_batch(BatchType::GradientRect, count);
    }

    /// Routes the incoming vector path configuration by decomposing it into
    /// two highly optimized, complementary GPU execution phases.
    fn draw_path(&mut self, config: PathConfig) {
        // PHASE 1: Interior Geometry Rasterization (No Edges)
        let has_fill = config.fill.rgba()[3] > 0.0 && config.opacity > 0.0;
        if has_fill {
            self.fill_path_complex(config.clone());
        }

        // PHASE 2: Boundary Wireframe Mesh Generation (No Fills)
        let has_stroke = config.stroke_width > 0.0 && config.stroke.rgba()[3] > 0.0;
        if has_stroke {
            self.stroke_path(config);
        } else if has_fill {
            // Auto anti-aliasing fringe
            let mut aa_config = config.clone();
            aa_config.stroke = config.fill;
            aa_config.stroke_width = 0.0; // Make the edge smooth while keeping the fill color
            self.stroke_path(aa_config);
        }
    }

    fn draw_text(&mut self, config: TextConfig) {
        self.collected_texts.push(config);
    }
}
