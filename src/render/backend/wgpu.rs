use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use ab_glyph::{FontArc, PxScale};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ============================================================================
// GPU Data Structures (Strict 1:1 WGSL Alignment)
// ============================================================================

/// 实例化SDF图元的基础数据（对应WGSL的PointData）
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
}

/// 线图元的GPU数据（对应WGSL的LineData）
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
}

/// 矩形图元的GPU数据（对应WGSL的RectData）
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub corner_radius: f32,
}

/// 多边形图元的GPU数据（对应WGSL的PolygonData）
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuPolygon {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub radius: f32,
    pub sides: f32,
    pub shape_type: f32, // 形状类型（1=三角/2=菱形/3=五边形等）
}

/// 渐变矩形的GPU数据（对应WGSL的GradientRectData）
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
    pub angle: f32, // 渐变角度（弧度）
    pub opacity: f32,
}

/// 文本顶点数据（用于字形图集渲染）
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

impl TextVertex {
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

/// 路径顶点数据
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
                offset: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32,
            },
        ],
    };
}

/// 全局Uniforms（对应WGSL的Uniforms，严格std140对齐）
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub screen_width: f32,
    pub screen_height: f32,
    pub scale_factor: f32,
    pub _padding: f32,
}

// ============================================================================
// WGPU Backend Core Implementation
// ============================================================================

pub struct WgpuBackend {
    // 基础WGPU设备
    device: wgpu::Device,
    queue: wgpu::Queue,
    uniform_buffer: wgpu::Buffer,
    uniforms: Uniforms,

    // 主绑定组（与chart.wgsl中@group(0) bindings 对应）
    main_bind_group: wgpu::BindGroup,
    main_bind_group_layout: wgpu::BindGroupLayout,

    // SDF图元（圆）相关
    sdf_pipeline: wgpu::RenderPipeline,
    sdf_buffer: wgpu::Buffer,
    pending_sdf_points: Vec<GpuPoint>,
    uploaded_sdf_count: u32,

    // 矩形图元相关
    rect_pipeline: wgpu::RenderPipeline,
    rect_buffer: wgpu::Buffer,
    pending_rects: Vec<GpuRect>,
    uploaded_rect_count: u32,

    // 多边形图元相关
    polygon_pipeline: wgpu::RenderPipeline,
    polygon_buffer: wgpu::Buffer,
    pending_polygons: Vec<GpuPolygon>,
    uploaded_polygon_count: u32,

    // 线图元相关
    line_pipeline: wgpu::RenderPipeline,
    line_buffer: wgpu::Buffer,
    pending_lines: Vec<GpuLine>,
    uploaded_line_count: u32,

    // 路径图元相关
    path_pipeline: wgpu::RenderPipeline,
    path_vertex_buffer: wgpu::Buffer,
    path_index_buffer: wgpu::Buffer,
    pending_path_vertices: Vec<PathVertex>,
    pending_path_indices: Vec<u16>,
    uploaded_path_index_count: u32,

    // 渐变矩形相关
    gradient_rect_pipeline: wgpu::RenderPipeline,
    gradient_rect_buffer: wgpu::Buffer,
    pending_gradient_rects: Vec<GpuGradientRect>,
    uploaded_gradient_rect_count: u32,

    // 文本渲染相关（占位，后续实现）
    text_pipeline: wgpu::RenderPipeline,
    text_vertex_buffer: wgpu::Buffer,
    text_atlas_texture: wgpu::Texture,
    text_atlas_view: wgpu::TextureView,
    text_atlas_sampler: wgpu::Sampler,
    uploaded_text_vertex_count: u32,
}

impl WgpuBackend {
    // 修复：匹配你项目的调用参数 + 类型转换
    pub async fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        screen_width: u32,
        screen_height: u32,
        scale_factor: f32,
    ) -> Self {
        // 自动加载你的chart.wgsl着色器，无需外部传入
        let shader = device.create_shader_module(wgpu::include_wgsl!("chart.wgsl"));

        // 初始化Uniform缓冲区
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

        // 创建主绑定组布局，匹配 chart.wgsl 中的 @group(0) bindings:
        // 0: circles, 1: lines, 2: rects, 3: polygons, 4: gradient_rects, 5: uniforms
        let main_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Main Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });

        // 创建初始主绑定组（使用占位缓冲），随后在 flush 时会用真实缓冲替换
        let dummy_circles = Self::create_dummy_buffer::<GpuPoint>(&device);
        let dummy_rects = Self::create_dummy_buffer::<GpuRect>(&device);
        let dummy_polys = Self::create_dummy_buffer::<GpuPolygon>(&device);
        let dummy_lines = Self::create_dummy_buffer::<GpuLine>(&device);
        let dummy_grad_rects = Self::create_dummy_buffer::<GpuGradientRect>(&device);

        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group"),
            layout: &main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(dummy_circles.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(dummy_lines.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(dummy_rects.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Buffer(dummy_polys.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Buffer(dummy_grad_rects.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()) },
            ],
        });

        // 创建所有渲染管线（使用 main_bind_group_layout 作为 group0 layout）
        let (_sdf_bg_layout, sdf_pipeline) = Self::create_sdf_pipeline(&device, &shader, &main_bind_group_layout);
        let (_line_bg_layout, line_pipeline) = Self::create_line_pipeline(&device, &shader, &main_bind_group_layout);
        let rect_pipeline = Self::create_rect_pipeline(&device, &shader, &main_bind_group_layout);
        let polygon_pipeline = Self::create_polygon_pipeline(&device, &shader, &main_bind_group_layout);
        let path_pipeline = Self::create_path_pipeline(&device, &shader, &main_bind_group_layout);
        let (_grad_bg_layout, gradient_rect_pipeline) = Self::create_gradient_rect_pipeline(&device, &shader, &main_bind_group_layout);
        let (_text_bg_layout, text_pipeline, text_atlas_texture, text_atlas_view, text_atlas_sampler) = Self::create_text_pipeline(&device, &shader, &main_bind_group_layout).await;

        // 创建占位缓冲区
        let sdf_buffer = Self::create_dummy_buffer::<GpuPoint>(&device);
        let rect_buffer = Self::create_dummy_buffer::<GpuRect>(&device);
        let polygon_buffer = Self::create_dummy_buffer::<GpuPolygon>(&device);
        let line_buffer = Self::create_dummy_buffer::<GpuLine>(&device);
        let path_vertex_buffer = Self::create_dummy_buffer::<PathVertex>(&device);
        let path_index_buffer = Self::create_dummy_buffer::<u16>(&device);
        let gradient_rect_buffer = Self::create_dummy_buffer::<GpuGradientRect>(&device);
        let text_vertex_buffer = Self::create_dummy_buffer::<TextVertex>(&device);

        Self {
            device,
            queue,
            uniform_buffer,
            uniforms,
            main_bind_group,
            main_bind_group_layout,

            sdf_pipeline,
            sdf_buffer,
            pending_sdf_points: Vec::with_capacity(30_000),
            uploaded_sdf_count: 0,

            rect_pipeline,
            rect_buffer,
            pending_rects: Vec::with_capacity(10_000),
            uploaded_rect_count: 0,

            polygon_pipeline,
            polygon_buffer,
            pending_polygons: Vec::with_capacity(10_000),
            uploaded_polygon_count: 0,

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
            text_atlas_texture,
            text_atlas_view,
            text_atlas_sampler,
            uploaded_text_vertex_count: 0,
        }
    }

    // ============================================================================
    // Pipeline Creation Helpers
    // ============================================================================

    fn create_sdf_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        let sdf_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Bind Group Layout"),
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
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Render Pipeline"),
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
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        (sdf_bind_group_layout, pipeline)
    }

    fn create_line_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        let line_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                buffers: &[],
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

    fn create_gradient_rect_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        main_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        let gradient_rect_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gradient Rect Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
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
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, strip_index_format: None, front_face: wgpu::FrontFace::Ccw, cull_mode: None, unclipped_depth: false, polygon_mode: wgpu::PolygonMode::Fill, conservative: false },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        (gradient_rect_bind_group_layout, pipeline)
    }

    async fn create_text_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        _main_layout: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
        wgpu::Texture,
        wgpu::TextureView,
        wgpu::Sampler,
    ) {
        // 简单占位实现：创建一个轻量 WGSL shader，为 text pipeline 提供可用的 entry points
        let atlas_size = (2048u32, 2048u32);
        let text_atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas Texture"),
            size: wgpu::Extent3d { width: atlas_size.0, height: atlas_size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let text_atlas_view = text_atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let text_atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        // minimal WGSL for text pipeline
        let text_wgsl = r#"
struct Uniforms { screen_width: f32, screen_height: f32, scale_factor: f32, _padding: f32 };
@group(0) @binding(5) var<uniform> uniforms: Uniforms;
struct In { @location(0) position: vec2<f32>, @location(1) tex_coords: vec2<f32>, @location(2) color: vec4<f32>, };
struct Out { @builtin(position) clip_pos: vec4<f32>, @location(0) color: vec4<f32>, };
@vertex fn text_vs(in: In) -> Out {
    let sw = uniforms.screen_width * uniforms.scale_factor;
    let sh = uniforms.screen_height * uniforms.scale_factor;
    let ndc = vec4((in.position.x/sw)*2.0-1.0, 1.0-(in.position.y/sh)*2.0, 0.0, 1.0);
    var o: Out; o.clip_pos = ndc; o.color = in.color; return o;
}
@fragment fn text_fs(in: Out) -> @location(0) vec4<f32> { return in.color; }
"#;

        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("text_wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(text_wgsl)) });

        let text_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("Text Bind Group Layout"), entries: &[wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }] });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("Text Pipeline Layout"), bind_group_layouts: &[Some(&text_bind_group_layout)], immediate_size: 0 });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &text_shader, entry_point: Some("text_vs"), buffers: &[TextVertex::DESC], compilation_options: wgpu::PipelineCompilationOptions::default() },
            fragment: Some(wgpu::FragmentState { module: &text_shader, entry_point: Some("text_fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: wgpu::PipelineCompilationOptions::default() }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        (text_bind_group_layout, text_pipeline, text_atlas_texture, text_atlas_view, text_atlas_sampler)
    }

    // ============================================================================
    // Bind Group Creation Helpers
    // ============================================================================

    

    // ============================================================================
    // Buffer Creation Helpers
    // ============================================================================

    fn create_dummy_buffer<T: Pod>(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("Dummy {} Buffer", std::any::type_name::<T>()).as_str()),
            contents: bytemuck::cast_slice(&[T::zeroed()]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
        })
    }

    // 修复：重构方法，解决可变借用冲突
    fn create_buffer<T: Pod>(&self, data: &[T]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("Updated {} Buffer", std::any::type_name::<T>()).as_str()),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
        })
    }

    // ============================================================================
    // Path & Text Helpers
    // ============================================================================

    fn tessellate_path(&mut self, _config: &PathConfig) {
        self.pending_path_vertices.clear();
        self.pending_path_indices.clear();
    }

    fn process_text(&mut self) {
        // 空实现，避免未定义枚举错误
    }

    // ============================================================================
    // Render & Flush
    // ============================================================================

    pub fn flush_and_render(&mut self, view: &wgpu::TextureView) {

        // 更新SDF缓冲区（修复可变借用冲突）
        if !self.pending_sdf_points.is_empty() {
            let points = std::mem::take(&mut self.pending_sdf_points);
            self.sdf_buffer = self.create_buffer(&points);
            self.uploaded_sdf_count = points.len() as u32;
        }

        // 更新矩形缓冲区
        if !self.pending_rects.is_empty() {
            let rects = std::mem::take(&mut self.pending_rects);
            self.rect_buffer = self.create_buffer(&rects);
            self.uploaded_rect_count = rects.len() as u32;
        }

        // 更新多边形缓冲区
        if !self.pending_polygons.is_empty() {
            let polygons = std::mem::take(&mut self.pending_polygons);
            self.polygon_buffer = self.create_buffer(&polygons);
            self.uploaded_polygon_count = polygons.len() as u32;
        }

        // 更新线缓冲区
        if !self.pending_lines.is_empty() {
            let lines = std::mem::take(&mut self.pending_lines);
            self.line_buffer = self.create_buffer(&lines);
            self.uploaded_line_count = lines.len() as u32;
        }

        // 更新路径缓冲区
        if !self.pending_path_vertices.is_empty() {
            let vertices = std::mem::take(&mut self.pending_path_vertices);
            let indices = std::mem::take(&mut self.pending_path_indices);
            self.path_vertex_buffer = self.create_buffer(&vertices);
            self.path_index_buffer = self.create_buffer(&indices);
            self.uploaded_path_index_count = indices.len() as u32;
        }

        // 更新渐变矩形缓冲区
        if !self.pending_gradient_rects.is_empty() {
            let rects = std::mem::take(&mut self.pending_gradient_rects);
            self.gradient_rect_buffer = self.create_buffer(&rects);
            self.uploaded_gradient_rect_count = rects.len() as u32;
        }

        // 处理文本
        self.process_text();

        // 在所有缓冲区更新后，重建主绑定组以指向最新缓冲区
        self.main_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group (updated)"),
            layout: &self.main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(self.sdf_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(self.line_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(self.rect_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Buffer(self.polygon_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Buffer(self.gradient_rect_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Buffer(self.uniform_buffer.as_entire_buffer_binding()) },
            ],
        });

        // 执行渲染
        self.render(view);
    }

    fn render(&mut self, view: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Charton Render Encoder"),
        });

        {
            // 修复：补充缺失的 depth_slice + multiview_mask
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Charton Main Render Pass"),
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

            // 渲染SDF图元（圆）
            if self.uploaded_sdf_count > 0 {
                rpass.set_pipeline(&self.sdf_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_sdf_count);
            }

            // 渲染矩形图元
            if self.uploaded_rect_count > 0 {
                rpass.set_pipeline(&self.rect_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_rect_count);
            }

            // 渲染多边形图元
            if self.uploaded_polygon_count > 0 {
                rpass.set_pipeline(&self.polygon_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_polygon_count);
            }

            // 渲染线图元
            if self.uploaded_line_count > 0 {
                rpass.set_pipeline(&self.line_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_line_count);
            }

            // 渲染路径图元
            if self.uploaded_path_index_count > 0 {
                rpass.set_pipeline(&self.path_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.path_vertex_buffer.slice(..));
                rpass.set_index_buffer(self.path_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..self.uploaded_path_index_count, 0, 0..1);
            }

            // 渲染渐变矩形
            if self.uploaded_gradient_rect_count > 0 {
                rpass.set_pipeline(&self.gradient_rect_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_gradient_rect_count);
            }

            // 渲染文本
            if self.uploaded_text_vertex_count > 0 {
                rpass.set_pipeline(&self.text_pipeline);
                rpass.set_bind_group(0, &self.main_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                rpass.draw(0..self.uploaded_text_vertex_count * 3, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // 重置上传计数
        self.uploaded_sdf_count = 0;
        self.uploaded_line_count = 0;
        self.uploaded_path_index_count = 0;
        self.uploaded_gradient_rect_count = 0;
        self.uploaded_text_vertex_count = 0;
    }
}

// ============================================================================
// RenderBackend Implementation
// ============================================================================

impl RenderBackend for WgpuBackend {
        fn draw_circle(&mut self, config: CircleConfig) {
        let rgba = config.fill.rgba();
        self.pending_sdf_points.push(GpuPoint {
            x: config.x as f32,
            y: config.y as f32,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            radius: config.radius as f32,
            // 新增 shape_type 字段（需同步修改 GpuPoint 结构体）
            shape_type: 0.0, // 0 = 圆形
        });
    }

    fn draw_rect(&mut self, config: RectConfig) {
        let rgba = config.fill.rgba();
        self.pending_rects.push(GpuRect {
            x: config.x as f32,
            y: config.y as f32,
            width: config.width as f32,
            height: config.height as f32,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            corner_radius: 0.0,
        });
    }

        // 重构 draw_polygon：专用于规则凸多边形标记（三角形/菱形/五边形等）
    fn draw_polygon(&mut self, config: PolygonConfig) {
        let vertex_count = config.points.len();
        // 仅处理 3+ 顶点的规则凸多边形（符合设计契约）
        if vertex_count < 3 {
            return;
        }

        let fill_color = config.fill.rgba();
        let alpha = config.fill_opacity as f32;

        // 1. 计算多边形几何质心（更精准的中心计算）
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        for &(x, y) in &config.points {
            cx += x as f32;
            cy += y as f32;
        }
        cx /= vertex_count as f32;
        cy /= vertex_count as f32;

        // 2. 计算最大半径（覆盖所有顶点的外接圆半径，保证多边形完整渲染）
        let mut max_radius = 0.0f32;
        for &(x, y) in &config.points {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let radius = (dx * dx + dy * dy).sqrt();
            if radius > max_radius {
                max_radius = radius;
            }
        }

        // 3. 映射规则多边形类型（与 WGSL 中的 SDF 公式一一对应）
        // 严格遵循「仅处理规则凸多边形」的语义约束
        let shape_type = match vertex_count {
            3 => 1.0,  // 三角形（等边）
            4 => 2.0,  // 菱形
            5 => 3.0,  // 五边形
            6 => 4.0,  // 六边形
            8 => 5.0,  // 八边形
            10 => 6.0, // 五角星（10顶点）
            _ => 0.0,  // 降级为圆形（兜底）
        };

        // 4. 推入多边形专属缓冲区（而非复用圆形缓冲区，语义分离）
        self.pending_polygons.push(GpuPolygon {
            x: cx,          // 多边形中心X
            y: cy,          // 多边形中心Y
            r: fill_color[0],// 红通道
            g: fill_color[1],// 绿通道
            b: fill_color[2],// 蓝通道
            a: fill_color[3] * alpha, // 透明度
            radius: max_radius,       // 外接圆半径
            sides: vertex_count as f32, // 边数（供WGSL SDF计算）
            shape_type,                // 形状类型（供WGSL分支判断）
        });
    }

    fn draw_path(&mut self, config: PathConfig) {
        self.tessellate_path(&config);
    }

    fn draw_text(&mut self, _config: TextConfig) {
        // 空实现，避免未定义枚举
    }

    fn draw_line(&mut self, config: LineConfig) {
        let rgba = config.color.rgba();
        self.pending_lines.push(GpuLine {
            x1: config.x1 as f32,
            y1: config.y1 as f32,
            x2: config.x2 as f32,
            y2: config.y2 as f32,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            width: config.width as f32,
        });
    }

    fn draw_gradient_rect(&mut self, config: GradientRectConfig) {
        if config.stops.is_empty() {
            return;
        }
        let start = config.stops.first().unwrap();
        let end = config.stops.last().unwrap();
        let start_color = start.1.rgba();
        let end_color = end.1.rgba();
        
        self.pending_gradient_rects.push(GpuGradientRect {
            x: config.x as f32,
            y: config.y as f32,
            width: config.width as f32,
            height: config.height as f32,
            start_r: start_color[0],
            start_g: start_color[1],
            start_b: start_color[2],
            start_a: start_color[3],
            end_r: end_color[0],
            end_g: end_color[1],
            end_b: end_color[2],
            end_a: end_color[3],
            angle: if config.is_vertical { std::f32::consts::FRAC_PI_2 } else { 0.0 },
            opacity: 1.0,
        });
    }
}