// ============================================================================
// (WGPU + Full Advanced SDF + One-Shot Instancing) Unified Master Shader
// ============================================================================

// ====================== 核心数据结构（WGSL标准语法：字段用逗号分隔）======================
struct PointData {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    radius: f32,
    shape_type: f32,
};

struct LineData {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    width: f32,
};

struct GradientRectData {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    start_r: f32,
    start_g: f32,
    start_b: f32,
    start_a: f32,
    end_r: f32,
    end_g: f32,
    end_b: f32,
    end_a: f32,
    angle: f32,
    opacity: f32,
};

struct PathVertex {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct TextVertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

// ====================== 绑定组布局 ======================
@group(0) @binding(0) var<storage, read> points: array<PointData>;
@group(0) @binding(0) var<storage, read> lines: array<LineData>;
@group(0) @binding(0) var<storage, read> gradient_rects: array<GradientRectData>;

struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
    _padding: f32,
};
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

@group(0) @binding(2) var text_atlas: texture_2d<f32>;
@group(0) @binding(3) var text_sampler: sampler;

// ====================== 输出结构体 ======================
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32,
    @location(1) screen_pos: vec2<f32>,
};

struct LineVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32,
};

struct RectVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32,
    @location(1) uv: vec2<f32>,
};

struct PathVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct TextVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

// ============================================================================
// SDF 函数库（你的原版代码，无修改、语法正确）
// ============================================================================
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_square(p: vec2<f32>, size: vec2<f32>, corner_radius: f32) -> f32 {
    let d = abs(p) - size + corner_radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - corner_radius;
}

fn sd_triangle(p: vec2<f32>, r: f32) -> f32 {
    const k = sqrt(3.0);
    var p_rot = vec2<f32>(p.x, -p.y);
    var p_mod = vec2<f32>(abs(p_rot.x) - r, p_rot.y + r / k);
    
    if (p_mod.x + k * p_mod.y > 0.0) {
        p_mod = vec2<f32>(p_mod.x - k * p_mod.y, -k * p_mod.x - p_mod.y) / 2.0;
    }
    p_mod.x = clamp(p_mod.x, -2.0 * r, 0.0);
    return -length(p_mod) * sign(p_mod.y);
}

fn sd_star(p: vec2<f32>, r: f32) -> f32 {
    let k1 = vec2<f32>(0.80901699437, -0.58778525229);
    let k2 = vec2<f32>(-0.30901699437, 0.95105651629);
    var p_mod = vec2<f32>(abs(p.x), p.y);
    p_mod -= 2.0 * max(dot(k1, p_mod), 0.0) * k1;
    p_mod -= 2.0 * max(dot(k2, p_mod), 0.0) * k2;
    p_mod.x = abs(p_mod.x);
    p_mod -= vec2<f32>(clamp(p_mod.x, r * 0.38196601125, r), r * 0.38196601125);
    return length(p_mod) * sign(p_mod.y);
}

fn sd_diamond(p: vec2<f32>, r: f32) -> f32 {
    let p_abs = abs(p);
    let h = clamp((-2.0 * p_abs.x + p_abs.y) / 2.0, -r, r);
    let d = length(p_abs - vec2<f32>(r, 0.0) + vec2<f32>(1.0, -1.0) * h);
    return d * sign(p_abs.x + p_abs.y - r);
}

fn sd_pentagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec3<f32>(0.809016994, 0.587785252, 0.324919696);
    var p_mod = vec2<f32>(abs(p.x), p.y);
    p_mod -= 2.0 * min(dot(vec2<f32>(-k.x, k.y), p_mod), 0.0) * vec2<f32>(-k.x, k.y);
    p_mod -= 2.0 * min(dot(vec2<f32>(k.x, k.y), p_mod), 0.0) * vec2<f32>(k.x, k.y);
    p_mod -= vec2<f32>(clamp(p_mod.x, -r * k.z, r * k.z), r);
    return length(p_mod) * sign(p_mod.y);
}

fn sd_hexagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec3<f32>(-0.866025404, 0.5, 0.577350269);
    var p_mod = abs(p);
    p_mod -= 2.0 * min(dot(k.xy, p_mod), 0.0) * k.xy;
    p_mod -= vec2<f32>(clamp(p_mod.x, -k.z * r, k.z * r), r);
    return length(p_mod) * sign(p_mod.y);
}

fn sd_octagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec3<f32>(-0.9238795325, 0.3826834323, 0.4142135623);
    var p_mod = abs(p);
    p_mod -= 2.0 * min(dot(vec2<f32>(k.x, k.y), p_mod), 0.0) * vec2<f32>(k.x, k.y);
    p_mod -= 2.0 * min(dot(vec2<f32>(k.y, k.x), p_mod), 0.0) * vec2<f32>(k.y, k.x);
    p_mod -= vec2<f32>(clamp(p_mod.x, -k.z * r, k.z * r), r);
    return length(p_mod) * sign(p_mod.y);
}

// ============================================================================
// 1. SDF 渲染管线（原版无修改）
// ============================================================================
@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32
) -> VertexOutput {
    let p = points[i_idx];
    
    var pos = vec2<f32>(0.0, 0.0);
    if (v_idx == 0u) { pos = vec2<f32>(-1.0, -1.0); } 
    if (v_idx == 1u) { pos = vec2<f32>( 1.0, -1.0); } 
    if (v_idx == 2u) { pos = vec2<f32>(-1.0,  1.0); } 
    if (v_idx == 3u) { pos = vec2<f32>( 1.0,  1.0); } 

    var box_scale = 1.0;
    if (p.shape_type < 1.5) { box_scale = 1.1; }
    else if (p.shape_type < 2.5) { box_scale = 1.3; }
    else if (p.shape_type < 3.5) { box_scale = 1.8; }
    else if (p.shape_type < 4.5) { box_scale = 1.4; }
    else if (p.shape_type < 5.5) { box_scale = 1.45; }
    else if (p.shape_type < 6.5) { box_scale = 1.45; }
    else if (p.shape_type < 7.5) { box_scale = 1.4; }

    let scaled_pos = vec2<f32>(p.x, p.y) * uniforms.scale_factor;
    let final_pos = scaled_pos + pos * (p.radius * box_scale * uniforms.scale_factor);
    let scaled_width = uniforms.screen_width * uniforms.scale_factor;
    let scaled_height = uniforms.screen_height * uniforms.scale_factor;
    let x_ndc = (final_pos.x / scaled_width) * 2.0 - 1.0;
    let y_ndc = 1.0 - (final_pos.y / scaled_height) * 2.0;
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x_ndc, y_ndc, 0.0, 1.0);
    out.instance_idx = i_idx;
    out.screen_pos = final_pos;  
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = points[in.instance_idx];
    let local_pos = in.screen_pos - vec2<f32>(p.x * uniforms.scale_factor, p.y * uniforms.scale_factor);
    let scaled_radius = p.radius * uniforms.scale_factor;
    
    var dist: f32 = 0.0;
    const SQUARE_CORNER_RADIUS_FACTOR: f32 = 0.1;
    
    if (p.shape_type < 0.5) { dist = sd_circle(local_pos, scaled_radius); }
    else if (p.shape_type < 1.5) { dist = sd_square(local_pos, vec2<f32>(scaled_radius), scaled_radius * SQUARE_CORNER_RADIUS_FACTOR); }
    else if (p.shape_type < 2.5) { dist = sd_triangle(local_pos, scaled_radius); }
    else if (p.shape_type < 3.5) { dist = sd_star(local_pos, scaled_radius); }
    else if (p.shape_type < 4.5) { dist = sd_diamond(local_pos, scaled_radius); }
    else if (p.shape_type < 5.5) { dist = sd_pentagon(local_pos, scaled_radius); }
    else if (p.shape_type < 6.5) { dist = sd_hexagon(local_pos, scaled_radius); }
    else if (p.shape_type < 7.5) { dist = sd_octagon(local_pos, scaled_radius); }
    else { dist = sd_circle(local_pos, scaled_radius); }
    
    let edge = fwidth(dist) * 1.2;
    let alpha = 1.0 - smoothstep(-edge, edge, dist);
    if (alpha <= 0.0) { discard; }
    
    return vec4<f32>(p.r, p.g, p.b, p.a * alpha);
}

// ============================================================================
// 2. 线渲染管线
// ============================================================================
@vertex
fn line_vs_main(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32
) -> LineVertexOutput {
    let line = lines[i_idx];
    let scale = uniforms.scale_factor;

    let p1 = vec2<f32>(line.x1, line.y1) * scale;
    let p2 = vec2<f32>(line.x2, line.y2) * scale;
    let dir = normalize(p2 - p1);
    let perp = vec2<f32>(-dir.y, dir.x) * (line.width * 0.5 * scale);

    var pos = vec2<f32>(0.0);
    switch v_idx {
        case 0u: { pos = p1 - perp; }
        case 1u: { pos = p1 + perp; }
        case 2u: { pos = p2 - perp; }
        case 3u: { pos = p2 + perp; }
        default: { pos = p1; }
    }

    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let x = (pos.x / sw) * 2.0 - 1.0;
    let y = 1.0 - (pos.y / sh) * 2.0;

    var out: LineVertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.instance_idx = i_idx;
    return out;
}

@fragment
fn line_fs_main(in: LineVertexOutput) -> @location(0) vec4<f32> {
    let line = lines[in.instance_idx];
    return vec4<f32>(line.r, line.g, line.b, line.a);
}

// ============================================================================
// 3. 渐变矩形渲染管线
// ============================================================================
@vertex
fn gradient_rect_vs_main(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32
) -> RectVertexOutput {
    let rect = gradient_rects[i_idx];
    let scale = uniforms.scale_factor;

    var pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    switch v_idx {
        case 0u: { pos = vec2<f32>(rect.x, rect.y); uv = vec2<f32>(0.0, 0.0); }
        case 1u: { pos = vec2<f32>(rect.x + rect.width, rect.y); uv = vec2<f32>(1.0, 0.0); }
        case 2u: { pos = vec2<f32>(rect.x, rect.y + rect.height); uv = vec2<f32>(0.0, 1.0); }
        case 3u: { pos = vec2<f32>(rect.x + rect.width, rect.y + rect.height); uv = vec2<f32>(1.0, 1.0); }
        default: { pos = vec2<f32>(rect.x, rect.y); uv = vec2<f32>(0.0, 0.0); }
    }

    pos *= scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let x = (pos.x / sw) * 2.0 - 1.0;
    let y = 1.0 - (pos.y / sh) * 2.0;

    var out: RectVertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.instance_idx = i_idx;
    out.uv = uv;
    return out;
}

@fragment
fn gradient_rect_fs_main(in: RectVertexOutput) -> @location(0) vec4<f32> {
    let rect = gradient_rects[in.instance_idx];
    let mix_val = in.uv.x;
    let r = mix(rect.start_r, rect.end_r, mix_val);
    let g = mix(rect.start_g, rect.end_g, mix_val);
    let b = mix(rect.start_b, rect.end_b, mix_val);
    let a = mix(rect.start_a, rect.end_a, mix_val) * rect.opacity;
    return vec4<f32>(r, g, b, a);
}

// ============================================================================
// 4. 路径渲染管线
// ============================================================================
@vertex
fn path_vs_main(vertex: PathVertex) -> PathVertexOutput {
    let scale = uniforms.scale_factor;
    let pos = vertex.pos * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let x = (pos.x / sw) * 2.0 - 1.0;
    let y = 1.0 - (pos.y / sh) * 2.0;

    var out: PathVertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = vertex.color;
    return out;
}

@fragment
fn path_fs_main(in: PathVertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// ============================================================================
// 5. 文本渲染管线（最终补齐，语法正确）
// ============================================================================
@vertex
fn text_vs_main(v: TextVertex) -> TextVertexOutput {
    let scale = uniforms.scale_factor;
    let pos = v.pos * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    
    var out: TextVertexOutput;
    out.clip_position = vec4<f32>((pos.x / sw)*2.0-1.0, 1.0-(pos.y / sh)*2.0, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

@fragment
fn text_fs_main(in: TextVertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(text_atlas, text_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}