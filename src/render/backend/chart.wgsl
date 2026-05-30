// ============================================================================
// Charton WGPU Shader
// Unified Rendering Primitives - Strictly Compliant with RenderBackend Contract
// Primitives: Circle, Line, Path(Polyline/Area), Rect, Polygon, GradientRect
// NO TEXT IMPLEMENTATION | NO REDUNDANCY | SEMANTIC SEPARATION
// ============================================================================

// ---------------------------
// Storage Buffer Data Structures (Semantically Separated)
// ---------------------------
/// Circle data (draw_circle: exclusive for circular markers/points)
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

/// Single line segment data (draw_line: axis, grid, ticks, whiskers)
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

/// Path vertex data (draw_path: LineMark, AreaMark, continuous geometry)
struct PathVertex {
    pos: vec2<f32>,
    color: vec4<f32>,
    is_fill: f32,      // 0.0 = stroke vertex, 1.0 = area fill vertex
};

/// Rectangle data (draw_rect: bars, boxes, axis backgrounds)
struct RectData {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    corner_radius: f32,
};

/// Polygon data (draw_polygon: symmetric markers - triangle, hexagon, diamond, star)
/// Matches GpuPolygon layout: uses shape_type instead of sides
struct PolygonData {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    radius: f32,
    shape_type: f32,   // Maps 1:1 to Rust WgpuBackend.draw_polygon()
                       // 0.0=Circle(fallback), 1.0=Triangle, 2.0=Star,
                       // 3.0=Diamond, 4.0=Pentagon, 5.0=Hexagon, 6.0=Octagon
};

/// Gradient rectangle data (draw_gradient_rect: heatmaps, themed panels)
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

// ---------------------------
// Uniform Buffer (Global Render State)
// ---------------------------
struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
    _padding: f32,
};

// ---------------------------
// Bind Group Layout
// ---------------------------
@group(0) @binding(0) var<storage, read> circles: array<PointData>;
@group(0) @binding(1) var<storage, read> lines: array<LineData>;
@group(0) @binding(2) var<storage, read> rects: array<RectData>;
@group(0) @binding(3) var<storage, read> polygons: array<PolygonData>;
@group(0) @binding(4) var<storage, read> gradient_rects: array<GradientRectData>;
@group(0) @binding(5) var<uniform> uniforms: Uniforms;

// ---------------------------
// Vertex Output Structures
// ---------------------------
struct CircleOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

struct LineOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32,
};

struct PathOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(2) @interpolate(flat) is_fill: f32,
};

struct RectOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

struct PolygonOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

struct GradientRectOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance_idx: u32,
};

// ============================================================================
// Analytical Geometric Signed Distance Fields (SDF Pure Math Implementation)
// ============================================================================

// 0.0 - Circle Signed Distance Field Equation Matrix
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

// 1.0 - Square Signed Distance Field Equation (optimized with corner radius support)
fn sd_square(p: vec2<f32>, size: vec2<f32>, corner_radius: f32) -> f32 {
    let d = abs(p) - size + corner_radius;
    return length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0) - corner_radius;
}

// 2.0 - Equilateral Triangle Signed Distance Field (corrected orientation and clipping)
fn sd_triangle(p: vec2<f32>, r: f32) -> f32 {
    const k: f32 = 1.73205080757; // sqrt(3.0)
    // Flip Y axis vertically to ensure triangle points upward, matching visual expectations
    var p_rot = vec2<f32>(p.x, -p.y);
    var p_mod = vec2<f32>(abs(p_rot.x) - r, p_rot.y + r / k);
    
    if (p_mod.x + k * p_mod.y > 0.0) {
        p_mod = vec2<f32>(p_mod.x - k * p_mod.y, -k * p_mod.x - p_mod.y) / 2.0;
    }
    // Corrected clamp range to prevent shape distortion
    p_mod.x = clamp(p_mod.x, -2.0 * r, 0.0);
    return -length(p_mod) * sign(p_mod.y);
}

// 3.0 - Regular 5-Pointed Geometric Star Signed Distance Field
fn sd_star(p: vec2<f32>, r: f32) -> f32 {
    let k1 = vec2<f32>(0.80901699437, -0.58778525229); // cos(18°), sin(18°)
    let k2 = vec2<f32>(-0.30901699437, 0.95105651629); // cos(108°), sin(108°)
    var p_mod = vec2<f32>(abs(p.x), p.y);
    p_mod -= 2.0 * max(dot(k1, p_mod), 0.0) * k1;
    p_mod -= 2.0 * max(dot(k2, p_mod), 0.0) * k2;
    p_mod.x = abs(p_mod.x);
    p_mod -= vec2<f32>(clamp(p_mod.x, r * 0.38196601125, r), r * 0.38196601125);
    return length(p_mod) * sign(p_mod.y);
}

// 4.0 - Diamond / Rhombus Geometric Signed Distance Field
fn sd_diamond(p: vec2<f32>, r: f32) -> f32 {
    let p_abs = abs(p);
    let h = clamp((-2.0 * p_abs.x + p_abs.y) / 2.0, -r, r);
    let d = length(p_abs - vec2<f32>(r, 0.0) + vec2<f32>(1.0, -1.0) * h);
    return d * sign(p_abs.x + p_abs.y - r);
}

// 5.0 - Regular Pentagon Geometric Signed Distance Field
fn sd_pentagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec3<f32>(0.809016994, 0.587785252, 0.324919696); // Constant axis symmetry constraints
    var p_mod = vec2<f32>(abs(p.x), p.y);
    p_mod -= 2.0 * min(dot(vec2<f32>(-k.x, k.y), p_mod), 0.0) * vec2<f32>(-k.x, k.y);
    p_mod -= 2.0 * min(dot(vec2<f32>(k.x, k.y), p_mod), 0.0) * vec2<f32>(k.x, k.y);
    p_mod -= vec2<f32>(clamp(p_mod.x, -r * k.z, r * k.z), r);
    return length(p_mod) * sign(p_mod.y);
}

// 6.0 - Regular Hexagon Geometric Signed Distance Field
fn sd_hexagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec3<f32>(-0.866025404, 0.5, 0.577350269); // Hexagonal coordinate layout configurations
    var p_mod = abs(p);
    p_mod -= 2.0 * min(dot(k.xy, p_mod), 0.0) * k.xy;
    p_mod -= vec2<f32>(clamp(p_mod.x, -k.z * r, k.z * r), r);
    return length(p_mod) * sign(p_mod.y);
}

// 7.0 - Regular Octagon Geometric Signed Distance Field
fn sd_octagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec3<f32>(-0.9238795325, 0.3826834323, 0.4142135623); // Octagonal reflection symmetry vectors
    var p_mod = abs(p);
    p_mod -= 2.0 * min(dot(vec2<f32>(k.x, k.y), p_mod), 0.0) * vec2<f32>(k.x, k.y);
    p_mod -= 2.0 * min(dot(vec2<f32>(k.y, k.x), p_mod), 0.0) * vec2<f32>(k.y, k.x);
    p_mod -= vec2<f32>(clamp(p_mod.x, -k.z * r, k.z * r), r);
    return length(p_mod) * sign(p_mod.y);
}

/// Unified shape selector using shape_type (1:1 match with Rust WgpuBackend)
fn sd_shape(p: vec2<f32>, radius: f32, shape_type: f32) -> f32 {
    if (shape_type == 0.0) { return sd_circle(p, radius); }
    if (shape_type == 1.0) { return sd_triangle(p, radius); }
    if (shape_type == 2.0) { return sd_star(p, radius); }
    if (shape_type == 3.0) { return sd_diamond(p, radius); }
    if (shape_type == 4.0) { return sd_pentagon(p, radius); }
    if (shape_type == 5.0) { return sd_hexagon(p, radius); }
    if (shape_type == 6.0) { return sd_octagon(p, radius); }
    
    // Fallback to circle for unknown shape types
    return sd_circle(p, radius);
}

// ---------------------------
// 1. Circle Pipeline (draw_circle: Scatter Plot Markers)
// ---------------------------
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
    
    // Smooth anti-aliasing
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    if (alpha <= 0.01) { discard; }
    
    return vec4(circle.r, circle.g, circle.b, circle.a * alpha);
}

// ---------------------------
// 2. Line Segment Pipeline (draw_line: Axis/Grid/Ticks)
// ---------------------------
@vertex
fn line_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> LineOutput {
    let line = lines[ii];
    let scale = uniforms.scale_factor;
    let p1 = vec2(line.x1, line.y1) * scale;
    let p2 = vec2(line.x2, line.y2) * scale;
    let dir = normalize(p2 - p1);
    let perp = vec2(-dir.y, dir.x) * (line.width * 0.5 * scale);

    var pos = vec2<f32>();
    switch vi {
        case 0u: { pos = p1 - perp; }
        case 1u: { pos = p1 + perp; }
        case 2u: { pos = p2 - perp; }
        case 3u: { pos = p2 + perp; }
        default: { pos = p1; }
    }

    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((pos.x/sw)*2.0-1.0, 1.0-(pos.y/sh)*2.0, 0.0, 1.0);

    var out: LineOutput;
    out.clip_pos = ndc;
    out.instance_idx = ii;
    return out;
}

@fragment
fn line_fs(in: LineOutput) -> @location(0) vec4<f32> {
    let line = lines[in.instance_idx];
    return vec4(line.r, line.g, line.b, line.a);
}

// ---------------------------
// 3. Path Pipeline (draw_path: LineMark / AreaMark)
// ---------------------------
@vertex
fn path_vs(
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) is_fill: f32
) -> PathOutput {
    let scale = uniforms.scale_factor;
    let screen_pos = pos * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((screen_pos.x/sw)*2.0-1.0, 1.0-(screen_pos.y/sh)*2.0, 0.0, 1.0);

    var out: PathOutput;
    out.clip_pos = ndc;
    out.color = color;
    out.is_fill = is_fill;
    return out;
}

@fragment
fn path_fs(in: PathOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// ---------------------------
// 4. Rectangle Pipeline (draw_rect: Bars/Boxes)
// ---------------------------
@vertex
fn rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> RectOutput {
    let r = rects[ii];
    var quad = vec2<f32>();
    switch vi {
        case 0u: { quad = vec2(-1.0, -1.0); }
        case 1u: { quad = vec2(1.0, -1.0); }
        case 2u: { quad = vec2(-1.0, 1.0); }
        case 3u: { quad = vec2(1.0, 1.0); }
        default: { quad = vec2(0.0); }
    }

    let scale = uniforms.scale_factor;
    let center = vec2(r.x + r.width/2.0, r.y + r.height/2.0) * scale;
    let half_size = vec2(r.width/2.0, r.height/2.0) * scale;
    let final_pos = center + quad * half_size * 1.5;
    
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((final_pos.x/sw)*2.0-1.0, 1.0-(final_pos.y/sh)*2.0, 0.0, 1.0);

    var out: RectOutput;
    out.clip_pos = ndc;
    out.screen_pos = final_pos;
    out.instance_idx = ii;
    return out;
}

@fragment
fn rect_fs(in: RectOutput) -> @location(0) vec4<f32> {
    let r = rects[in.instance_idx];
    let scale = uniforms.scale_factor;
    let center = vec2(r.x + r.width/2.0, r.y + r.height/2.0) * scale;
    let local = in.screen_pos - center;
    let half_size = vec2(r.width/2.0, r.height/2.0) * scale;
    let corner_radius = r.corner_radius * scale;

    let dist = sd_square(local, half_size, corner_radius);
    
    // Smooth anti-aliasing
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    if (alpha <= 0.01) { discard; }
    
    return vec4(r.r, r.g, r.b, r.a * alpha);
}

// ---------------------------
// 5. Polygon Pipeline (draw_polygon: Symmetric Markers)
// COMPLETELY REVISED: uses shape_type instead of sides, supports Star/Diamond/Triangle
// Matches 1:1 with Rust WgpuBackend.draw_polygon() implementation
// ---------------------------
@vertex
fn polygon_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> PolygonOutput {
    let poly = polygons[ii];
    var quad = vec2<f32>();
    switch vi {
        case 0u: { quad = vec2(-1.0, -1.0); }
        case 1u: { quad = vec2(1.0, -1.0); }
        case 2u: { quad = vec2(-1.0, 1.0); }
        case 3u: { quad = vec2(1.0, 1.0); }
        default: { quad = vec2(0.0); }
    }

    let scale = uniforms.scale_factor;
    let final_pos = vec2(poly.x, poly.y) * scale + quad * (poly.radius * 1.5 * scale);
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((final_pos.x/sw)*2.0-1.0, 1.0-(final_pos.y/sh)*2.0, 0.0, 1.0);

    var out: PolygonOutput;
    out.clip_pos = ndc;
    out.screen_pos = final_pos;
    out.instance_idx = ii;
    return out;
}

@fragment
fn polygon_fs(in: PolygonOutput) -> @location(0) vec4<f32> {
    let poly = polygons[in.instance_idx];
    let local = in.screen_pos - vec2(poly.x, poly.y) * uniforms.scale_factor;
    let r = poly.radius * uniforms.scale_factor;

    // Render symmetric marker using shape_type (no sides parameter)
    // Fully compliant with Rust backend: Triangle, Star, Diamond, Pentagon, Hexagon, Octagon
    let dist = sd_shape(local, r, poly.shape_type);

    // Subpixel anti-aliasing for sharp, clean edges
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    if (alpha <= 0.01) { discard; }

    return vec4(poly.r, poly.g, poly.b, poly.a * alpha);
}

// ---------------------------
// 6. Gradient Rectangle Pipeline (draw_gradient_rect)
// ---------------------------
@vertex
fn grad_rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> GradientRectOutput {
    let r = gradient_rects[ii];
    var quad = vec2<f32>();
    var uv = vec2<f32>();
    switch vi {
        case 0u: { quad = vec2(r.x, r.y); uv = vec2(0.0, 0.0); }
        case 1u: { quad = vec2(r.x + r.width, r.y); uv = vec2(1.0, 0.0); }
        case 2u: { quad = vec2(r.x, r.y + r.height); uv = vec2(0.0, 1.0); }
        case 3u: { quad = vec2(r.x + r.width, r.y + r.height); uv = vec2(1.0, 1.0); }
        default: { quad = vec2(r.x, r.y); }
    }

    let scale = uniforms.scale_factor;
    let screen_pos = quad * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((screen_pos.x/sw)*2.0-1.0, 1.0-(screen_pos.y/sh)*2.0, 0.0, 1.0);

    var out: GradientRectOutput;
    out.clip_pos = ndc;
    out.uv = uv;
    out.instance_idx = ii;
    return out;
}

@fragment
fn grad_rect_fs(in: GradientRectOutput) -> @location(0) vec4<f32> {
    let r = gradient_rects[in.instance_idx];
    let mix_val = in.uv.x;
    return vec4(
        mix(r.start_r, r.end_r, mix_val),
        mix(r.start_g, r.end_g, mix_val),
        mix(r.start_b, r.end_b, mix_val),
        mix(r.start_a, r.end_a, mix_val) * r.opacity
    );
}