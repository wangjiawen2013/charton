// ============================================================================
// Charton WGPU Shader: Unified Rendering Primitives (Scientific Architecture)
// ----------------------------------------------------------------------------
// Architecture: Two-Tier Bind Group Design
// - @group(0): Global Environment (Uniforms shared across all pipelines)
// - @group(1): Isolated Instance Data (Exclusive storage buffers per pipeline)
// ============================================================================

// ============================================================================
// SECTION 1: GLOBAL ENVIRONMENT (GROUP 0)
// ============================================================================

struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
    _padding: f32, // Preserves strict 16-byte alignment for std140 layout
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;


// ============================================================================
// SECTION 2: INSTANCE DATA STRUCTURES & ISOLATED BINDINGS (GROUP 1)
// ============================================================================

// ----------------------------------------------------------------------------
// 2.1 Circle Data & Binding
// ----------------------------------------------------------------------------
struct PointData {
    x: f32,
    y: f32,
    fill_r: f32,
    fill_g: f32,
    fill_b: f32,
    fill_a: f32,
    stroke_r: f32,
    stroke_g: f32,
    stroke_b: f32,
    stroke_a: f32,
    radius: f32,
    stroke_width: f32,
};
@group(1) @binding(0) var<storage, read> circles: array<PointData>;

// ----------------------------------------------------------------------------
// 2.2 Rectangle Data & Binding
// ----------------------------------------------------------------------------
struct RectData {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fill_r: f32,
    fill_g: f32,
    fill_b: f32,
    fill_a: f32,
    stroke_r: f32,
    stroke_g: f32,
    stroke_b: f32,
    stroke_a: f32,
    stroke_width: f32,
    corner_radius: f32,
};
@group(1) @binding(0) var<storage, read> rects: array<RectData>;

// ----------------------------------------------------------------------------
// 2.3 Line Data & Binding
// ----------------------------------------------------------------------------
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
    _pad1: f32, // Padding fields to guarantee 16-byte structural boundaries
    _pad2: f32,
    _pad3: f32,
};
@group(1) @binding(0) var<storage, read> lines: array<LineData>;

// ----------------------------------------------------------------------------
// 2.4 Gradient Rectangle Data & Binding
// ----------------------------------------------------------------------------
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
@group(1) @binding(0) var<storage, read> gradient_rects: array<GradientRectData>;

// ----------------------------------------------------------------------------
// 2.5 Polyline Path Extrusion Data & Binding
// Note: Requires 3 distinct storage slots to handle decoupled streaming queues
// ----------------------------------------------------------------------------
struct PathPointData {
    x: f32,
    y: f32,
};

struct PathStyle {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    thickness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct PathArgs {
    start_point_idx: u32,
    style_idx: u32,
    _pad0: u32, 
    _pad1: u32,
};

@group(1) @binding(0) var<storage, read> path_points: array<PathPointData>;
@group(1) @binding(1) var<storage, read> path_styles: array<PathStyle>;
@group(1) @binding(2) var<storage, read> path_args: array<PathArgs>;


// ============================================================================
// SECTION 3: VERTEX OUTPUT PIPELINE STRUCTURES
// ============================================================================

struct CircleOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

struct RectOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

struct LineOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32,
    @location(1) v_offset: f32,    // Signed distance from the central axis for AA profiling
    @location(2) half_width: f32,  // Hard boundary limit for the line thickness
};

struct PolygonOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct GradientRectOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

struct PathOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};


// ============================================================================
// SECTION 4: SIGNED DISTANCE FIELD (SDF) CORE MATH LIBRARY
// ============================================================================

/// Computes the Signed Distance Field for a perfect circle.
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

/// Computes the Signed Distance Field for a rectangle with optional rounded corners.
fn sd_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let d = abs(p) - b + vec2(r);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2(0.0))) - r;
}


// ============================================================================
// SECTION 5: SHADER PIPELINES (VERTEX & FRAGMENT)
// ============================================================================

// ----------------------------------------------------------------------------
// 5.1 Circle Pipeline
// ----------------------------------------------------------------------------
@vertex
fn circle_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> CircleOutput {
    var quad = vec2<f32>();
    switch vi {
        case 0u: { quad = vec2(-1.0, -1.0); }
        case 1u: { quad = vec2(1.0, -1.0); }
        case 2u: { quad = vec2(-1.0, 1.0); }
        case 3u: { quad = vec2(1.0, 1.0); }
        default: { quad = vec2(0.0); }
    }

    let scale = uniforms.scale_factor;
    let circle = circles[ii];
    
    // Inflate the quad bounding box slightly to secure enough canvas for SDF anti-aliasing
    let final_pos = vec2(circle.x, circle.y) * scale + quad * (circle.radius * 1.5 * scale);
    
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((final_pos.x/sw)*2.0-1.0, 1.0-(final_pos.y/sh)*2.0, 0.0, 1.0);

    var out: CircleOutput;
    out.clip_pos = ndc;
    out.screen_pos = final_pos;
    out.instance_idx = ii;
    return out;
}

@fragment
fn circle_fs(in: CircleOutput) -> @location(0) vec4<f32> {
    let circle = circles[in.instance_idx];
    let local = in.screen_pos - vec2(circle.x, circle.y) * uniforms.scale_factor;
    let r = circle.radius * uniforms.scale_factor;
    
    let dist = sd_circle(local, r);
    let aa = fwidth(dist);
    
    // Compute Core Fill
    let fill_alpha = 1.0 - smoothstep(-aa, aa, dist);
    let fill_color = vec4(circle.fill_r, circle.fill_g, circle.fill_b, circle.fill_a * fill_alpha);
    
    // Compute Boundary Stroke (Centered)
    let half_stroke = (circle.stroke_width * uniforms.scale_factor) * 0.5;
    let stroke_dist = abs(dist) - half_stroke;
    let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_dist);
    let stroke_color = vec4(circle.stroke_r, circle.stroke_g, circle.stroke_b, circle.stroke_a * stroke_alpha);
    
    // Standard Porter-Duff Over Alpha Compositing
    let out_a = stroke_color.a + fill_color.a * (1.0 - stroke_color.a);
    if (out_a <= 0.01) { discard; }
    
    let out_rgb = (stroke_color.rgb * stroke_color.a + fill_color.rgb * fill_color.a * (1.0 - stroke_color.a)) / out_a;
    return vec4(out_rgb, out_a);
}

// ----------------------------------------------------------------------------
// 5.2 Rectangle Pipeline
// ----------------------------------------------------------------------------
@vertex
fn rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> RectOutput {
    let r = rects[ii];
    let scale = uniforms.scale_factor;
    
    // Dynamic quad inflation to prevent clipping of the external stroke perimeter
    let padding = (r.stroke_width + 2.0) * scale;
    
    var local_pos = vec2<f32>();
    switch vi {
        case 0u: { local_pos = vec2(-padding, -padding); }
        case 1u: { local_pos = vec2(r.width * scale + padding, -padding); }
        case 2u: { local_pos = vec2(-padding, r.height * scale + padding); }
        case 3u: { local_pos = vec2(r.width * scale + padding, r.height * scale + padding); }
        default: { local_pos = vec2(0.0); }
    }

    let base_pos = vec2(r.x, r.y) * scale;
    let screen_pos = base_pos + local_pos;
    
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((screen_pos.x / sw) * 2.0 - 1.0, 1.0 - (screen_pos.y / sh) * 2.0, 0.0, 1.0);

    var out: RectOutput;
    out.clip_pos = ndc;
    out.screen_pos = screen_pos;
    out.instance_idx = ii;
    return out;
}

@fragment
fn rect_fs(in: RectOutput) -> @location(0) vec4<f32> {
    let r = rects[in.instance_idx];
    let scale = uniforms.scale_factor;
    
    let center = vec2(r.x + r.width * 0.5, r.y + r.height * 0.5) * scale;
    let half_extents = vec2(r.width * 0.5, r.height * 0.5) * scale;
    let local = in.screen_pos - center;
    
    let dist = sd_rounded_rect(local, half_extents, r.corner_radius * scale);
    let aa = fwidth(dist);

    // Compute Core Fill
    let fill_alpha = 1.0 - smoothstep(-aa, aa, dist);
    let fill_color = vec4(r.fill_r, r.fill_g, r.fill_b, r.fill_a * fill_alpha);

    // Compute Boundary Stroke (Centered)
    let half_stroke = (r.stroke_width * scale) * 0.5;
    let stroke_dist = abs(dist) - half_stroke;
    let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_dist);
    let stroke_color = vec4(r.stroke_r, r.stroke_g, r.stroke_b, r.stroke_a * stroke_alpha);

    // Alpha Compositing
    let out_a = stroke_color.a + fill_color.a * (1.0 - stroke_color.a);
    if (out_a <= 0.01) { discard; }

    let out_rgb = (stroke_color.rgb * stroke_color.a + fill_color.rgb * fill_color.a * (1.0 - stroke_color.a)) / out_a;
    return vec4(out_rgb, out_a);
}

// ----------------------------------------------------------------------------
// 5.3 Line Segment Pipeline
// ----------------------------------------------------------------------------
@vertex
fn line_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> LineOutput {
    let line = lines[ii];
    let scale = uniforms.scale_factor;
    let p1 = vec2(line.x1, line.y1) * scale;
    let p2 = vec2(line.x2, line.y2) * scale;
    
    var dir = p2 - p1;
    if (length(dir) < 0.0001) {
        dir = vec2<f32>(1.0, 0.0);
    }
    dir = normalize(dir);
    
    // Extrude geometry to create sub-pixel feathering margins
    let aa_padding = 1.5; 
    let h_width = line.width * 0.5 * scale;
    let total_extruding = h_width + aa_padding;
    let perp = vec2(-dir.y, dir.x) * total_extruding;

    var pos = vec2<f32>();
    var offset = 0.0;
    switch vi {
        case 0u: { pos = p1 + perp; offset =  total_extruding; }
        case 1u: { pos = p1 - perp; offset = -total_extruding; }
        case 2u: { pos = p2 + perp; offset =  total_extruding; }
        case 3u: { pos = p2 - perp; offset = -total_extruding; }
        default: { pos = p1; }
    }

    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((pos.x / sw) * 2.0 - 1.0, 1.0 - (pos.y / sh) * 2.0, 0.0, 1.0);

    var out: LineOutput;
    out.clip_pos = ndc;
    out.instance_idx = ii;
    out.v_offset = offset;   
    out.half_width = h_width; 
    return out;
}

@fragment
fn line_fs(in: LineOutput) -> @location(0) vec4<f32> {
    let line = lines[in.instance_idx];
    
    // Formulate analytical edge distance
    let dist = abs(in.v_offset) - in.half_width;
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    
    if (alpha <= 0.01) { discard; }
    
    return vec4(line.r, line.g, line.b, line.a * alpha);
}

// ----------------------------------------------------------------------------
// 5.4 Polygon Pipeline (Triangle/Star/Diamond via Vertex Buffer)
// ----------------------------------------------------------------------------
@vertex
fn polygon_vs(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) is_fill: f32
) -> PolygonOutput {
    let scale = uniforms.scale_factor;
    let screen_pos = position * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((screen_pos.x/sw)*2.0-1.0, 1.0-(screen_pos.y/sh)*2.0, 0.0, 1.0);

    var out: PolygonOutput;
    out.clip_pos = ndc;
    out.color = color;
    return out;
}

@fragment
fn polygon_fs(in: PolygonOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// ----------------------------------------------------------------------------
// 5.5 Gradient Rectangle Pipeline
// ----------------------------------------------------------------------------
@vertex
fn grad_rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> GradientRectOutput {
    let r = gradient_rects[ii];
    var quad = vec2<f32>();
    var uv = vec2<f32>();
    
    // UV Mapping: Top=0.0 to Bottom=1.0, Left=0.0 to Right=1.0
    switch vi {
        case 0u: { quad = vec2(r.x, r.y);                uv = vec2(0.0, 0.0); }
        case 1u: { quad = vec2(r.x + r.width, r.y);      uv = vec2(1.0, 0.0); }
        case 2u: { quad = vec2(r.x, r.y + r.height);     uv = vec2(0.0, 1.0); }
        case 3u: { quad = vec2(r.x + r.width, r.y + r.height); uv = vec2(1.0, 1.0); }
        default: { quad = vec2(r.x, r.y); }
    }

    let scale = uniforms.scale_factor;
    let screen_pos = quad * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((screen_pos.x / sw) * 2.0 - 1.0, 1.0 - (screen_pos.y / sh) * 2.0, 0.0, 1.0);

    var out: GradientRectOutput;
    out.clip_pos = ndc;
    out.uv = uv;
    out.instance_idx = ii;
    return out;
}

@fragment
fn grad_rect_fs(in: GradientRectOutput) -> @location(0) vec4<f32> {
    let r = gradient_rects[in.instance_idx];
    
    // Resolve interpolation axis based on CPU-provided angle parameter
    var mix_val = in.uv.x;
    if (r.angle > 0.0) {
        mix_val = in.uv.y;
    }

    let src_r = mix(r.start_r, r.end_r, mix_val);
    let src_g = mix(r.start_g, r.end_g, mix_val);
    let src_b = mix(r.start_b, r.end_b, mix_val);
    let src_a = mix(r.start_a, r.end_a, mix_val) * r.opacity;

    // Apply alpha premultiplication to respect standard blending pipelines
    return vec4<f32>(src_r * src_a, src_g * src_a, src_b * src_a, src_a);
}

// ----------------------------------------------------------------------------
// 5.6 Pure GPU Polyline Extrusion Pipeline
// ----------------------------------------------------------------------------
@vertex
fn path_simple_vs(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32 
) -> PathOutput {
    let args = path_args[ii];
    let path_style = path_styles[args.style_idx];

    let segment_idx = vi / 6u;
    let local_vertex_idx = vi % 6u;

    let p0_idx = args.start_point_idx + segment_idx;
    let p1_idx = p0_idx + 1u;

    let p0 = path_points[p0_idx];
    let p1 = path_points[p1_idx];

    // Data verification: collapse broken triangles safely
    if (p0.x != p0.x || p0.y != p0.y || p1.x != p1.x || p1.y != p1.y) {
        var out: PathOutput;
        out.clip_pos = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    }

    let delta = vec2<f32>(p1.x - p0.x, p1.y - p0.y);
    var current_dir = normalize(delta);
    
    if (length(delta) == 0.0) {
        current_dir = vec2<f32>(1.0, 0.0);
    }
    
    let normal = vec2<f32>(-current_dir.y, current_dir.x);
    var raw_pos = vec2<f32>(0.0, 0.0);
    var extrusion_side = 0.0;

    switch local_vertex_idx {
        case 0u: { raw_pos = vec2(p0.x, p0.y); extrusion_side =  1.0; }  
        case 1u: { raw_pos = vec2(p0.x, p0.y); extrusion_side = -1.0; } 
        case 2u: { raw_pos = vec2(p1.x, p1.y); extrusion_side =  1.0; }  
        
        case 3u: { raw_pos = vec2(p1.x, p1.y); extrusion_side =  1.0; }  
        case 4u: { raw_pos = vec2(p0.x, p0.y); extrusion_side = -1.0; } 
        case 5u: { raw_pos = vec2(p1.x, p1.y); extrusion_side = -1.0; } 
        default: {}
    }

    let ext_pos = raw_pos + normal * (path_style.thickness * 0.5 * extrusion_side);
    let scale = uniforms.scale_factor;
    let screen_pos = ext_pos * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;

    let ndc_x = (screen_pos.x / sw) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_pos.y / sh) * 2.0;

    var out: PathOutput;
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = vec4<f32>(path_style.r, path_style.g, path_style.b, path_style.a);
    return out;
}

@fragment
fn path_simple_fs(in: PathOutput) -> @location(0) vec4<f32> {
    return in.color;
}