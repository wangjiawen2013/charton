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
    pub shape_type: f32, // 0:圆 1:方 2:三角 3:星 4:菱形 5:五边形 6:六边形 7:八边形
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
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,

    // SDF图元（圆/矩形/多边形）相关
    sdf_pipeline: wgpu::RenderPipeline,
    sdf_bind_group: wgpu::BindGroup,
    sdf_bind_group_layout: wgpu::BindGroupLayout,
    sdf_buffer: wgpu::Buffer,
    pending_sdf_points: Vec<GpuPoint>,
    uploaded_sdf_count: u32,

    // 线图元相关
    line_pipeline: wgpu::RenderPipeline,
    line_bind_group: wgpu::BindGroup,
    line_bind_group_layout: wgpu::BindGroupLayout,
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
    gradient_rect_bind_group: wgpu::BindGroup,
    gradient_rect_bind_group_layout: wgpu::BindGroupLayout,
    gradient_rect_buffer: wgpu::Buffer,
    pending_gradient_rects: Vec<GpuGradientRect>,
    uploaded_gradient_rect_count: u32,

    // 文本渲染相关
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    text_bind_group_layout: wgpu::BindGroupLayout,
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

        // 创建Uniform绑定组布局
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                // 修复：BufferBinding 包装为 BindingResource
                resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
            }],
        });

        // 创建所有渲染管线
        let (sdf_bind_group_layout, sdf_pipeline) = Self::create_sdf_pipeline(&device, &shader, &uniform_bind_group_layout);
        let (line_bind_group_layout, line_pipeline) = Self::create_line_pipeline(&device, &shader, &uniform_bind_group_layout);
        let path_pipeline = Self::create_path_pipeline(&device, &shader, &uniform_bind_group_layout);
        let (gradient_rect_bind_group_layout, gradient_rect_pipeline) = Self::create_gradient_rect_pipeline(&device, &shader, &uniform_bind_group_layout);
        let (text_bind_group_layout, text_pipeline, text_atlas_texture, text_atlas_view, text_atlas_sampler) = Self::create_text_pipeline(&device, &shader, &uniform_bind_group_layout).await;

        // 创建占位缓冲区
        let sdf_buffer = Self::create_dummy_buffer::<GpuPoint>(&device);
        let line_buffer = Self::create_dummy_buffer::<GpuLine>(&device);
        let path_vertex_buffer = Self::create_dummy_buffer::<PathVertex>(&device);
        let path_index_buffer = Self::create_dummy_buffer::<u16>(&device);
        let gradient_rect_buffer = Self::create_dummy_buffer::<GpuGradientRect>(&device);
        let text_vertex_buffer = Self::create_dummy_buffer::<TextVertex>(&device);

        // 创建绑定组
        let sdf_bind_group = Self::create_sdf_bind_group(&device, &sdf_bind_group_layout, &sdf_buffer, &uniform_buffer);
        let line_bind_group = Self::create_line_bind_group(&device, &line_bind_group_layout, &line_buffer, &uniform_buffer);
        let gradient_rect_bind_group = Self::create_gradient_rect_bind_group(&device, &gradient_rect_bind_group_layout, &gradient_rect_buffer, &uniform_buffer);
        let text_bind_group = Self::create_text_bind_group(&device, &text_bind_group_layout, &uniform_buffer, &text_atlas_view, &text_atlas_sampler);

        Self {
            device,
            queue,
            uniform_buffer,
            uniforms,
            uniform_bind_group,
            uniform_bind_group_layout,

            sdf_pipeline,
            sdf_bind_group,
            sdf_bind_group_layout,
            sdf_buffer,
            pending_sdf_points: Vec::with_capacity(30_000),
            uploaded_sdf_count: 0,

            line_pipeline,
            line_bind_group,
            line_bind_group_layout,
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
            gradient_rect_bind_group,
            gradient_rect_bind_group_layout,
            gradient_rect_buffer,
            pending_gradient_rects: Vec::with_capacity(10_000),
            uploaded_gradient_rect_count: 0,

            text_pipeline,
            text_bind_group,
            text_bind_group_layout,
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
        _uniform_layout: &wgpu::BindGroupLayout,
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
            bind_group_layouts: &[Some(&sdf_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
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
        _uniform_layout: &wgpu::BindGroupLayout,
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
            bind_group_layouts: &[Some(&line_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("line_vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("line_fs_main"),
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

    fn create_path_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Path Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Path Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("path_vs_main"),
                buffers: &[PathVertex::DESC],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("path_fs_main"),
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
        _uniform_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
        let gradient_rect_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            bind_group_layouts: &[Some(&gradient_rect_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gradient Rect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("gradient_rect_vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("gradient_rect_fs_main"),
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

    async fn create_text_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        _uniform_layout: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
        wgpu::Texture,
        wgpu::TextureView,
        wgpu::Sampler,
    ) {
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
        let text_atlas_view = text_atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let text_atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            // 修复：mipmap_filter 类型修正
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let text_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(&text_bind_group_layout)],
            immediate_size: 0,
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("text_vs_main"),
                buffers: &[TextVertex::DESC],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("text_fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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
    // Bind Group Creation Helpers
    // ============================================================================

    fn create_sdf_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sdf_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(sdf_buffer.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
                },
            ],
        })
    }

    fn create_line_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        line_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Line Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(line_buffer.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
                },
            ],
        })
    }

    fn create_gradient_rect_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        gradient_rect_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gradient Rect Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(gradient_rect_buffer.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
                },
            ],
        })
    }

    fn create_text_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform_buffer: &wgpu::Buffer,
        atlas_view: &wgpu::TextureView,
        atlas_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(atlas_sampler),
                },
            ],
        })
    }

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
            self.sdf_bind_group = Self::create_sdf_bind_group(
                &self.device,
                &self.sdf_bind_group_layout,
                &self.sdf_buffer,
                &self.uniform_buffer,
            );
            self.uploaded_sdf_count = points.len() as u32;
        }

        // 更新线缓冲区
        if !self.pending_lines.is_empty() {
            let lines = std::mem::take(&mut self.pending_lines);
            self.line_buffer = self.create_buffer(&lines);
            self.line_bind_group = Self::create_line_bind_group(
                &self.device,
                &self.line_bind_group_layout,
                &self.line_buffer,
                &self.uniform_buffer,
            );
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
            self.gradient_rect_bind_group = Self::create_gradient_rect_bind_group(
                &self.device,
                &self.gradient_rect_bind_group_layout,
                &self.gradient_rect_buffer,
                &self.uniform_buffer,
            );
            self.uploaded_gradient_rect_count = rects.len() as u32;
        }

        // 处理文本
        self.process_text();

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

            // 渲染SDF图元
            if self.uploaded_sdf_count > 0 {
                rpass.set_pipeline(&self.sdf_pipeline);
                rpass.set_bind_group(0, &self.sdf_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_sdf_count);
            }

            // 渲染线图元
            if self.uploaded_line_count > 0 {
                rpass.set_pipeline(&self.line_pipeline);
                rpass.set_bind_group(0, &self.line_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_line_count);
            }

            // 渲染路径图元
            if self.uploaded_path_index_count > 0 {
                rpass.set_pipeline(&self.path_pipeline);
                rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.path_vertex_buffer.slice(..));
                rpass.set_index_buffer(self.path_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..self.uploaded_path_index_count, 0, 0..1);
            }

            // 渲染渐变矩形
            if self.uploaded_gradient_rect_count > 0 {
                rpass.set_pipeline(&self.gradient_rect_pipeline);
                rpass.set_bind_group(0, &self.gradient_rect_bind_group, &[]);
                rpass.draw(0..4, 0..self.uploaded_gradient_rect_count);
            }

            // 渲染文本
            if self.uploaded_text_vertex_count > 0 {
                rpass.set_pipeline(&self.text_pipeline);
                rpass.set_bind_group(0, &self.text_bind_group, &[]);
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
            shape_type: 0.0,
        });
    }

    fn draw_rect(&mut self, config: RectConfig) {
        let rgba = config.fill.rgba();
        self.pending_sdf_points.push(GpuPoint {
            x: (config.x + config.width / 2.0) as f32,
            y: (config.y + config.height / 2.0) as f32,
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3] * config.opacity as f32,
            radius: (config.width / 2.0) as f32,
            shape_type: 1.0,
        });
    }

    fn draw_polygon(&mut self, config: PolygonConfig) {
        let vertex_count = config.points.len();
        if vertex_count < 3 {
            return;
        }

        let fill_color = config.fill.rgba();
        let alpha = config.fill_opacity as f32;

        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        for &(x, y) in &config.points {
            sum_x += x as f32;
            sum_y += y as f32;
        }
        let cx = sum_x / vertex_count as f32;
        let cy = sum_y / vertex_count as f32;

        let mut max_r = 0.0f32;
        for &(x, y) in &config.points {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d > max_r {
                max_r = d;
            }
        }

        let shape_type = match vertex_count {
            3 => 2.0,
            4 => 1.0,
            5 => 5.0,
            6 => 6.0,
            8 => 7.0,
            _ => 0.0,
        };

        self.pending_sdf_points.push(GpuPoint {
            x: cx,
            y: cy,
            r: fill_color[0],
            g: fill_color[1],
            b: fill_color[2],
            a: fill_color[3] * alpha,
            radius: max_r,
            shape_type,
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