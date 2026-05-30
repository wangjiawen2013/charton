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
/// Matches GpuPoint layout: uses shape_type instead of sides
struct PolygonData {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    radius: f32,
    shape_type: f32,   // Maps 1:1 to Rust PointShape enum via vertex count
                       // 0.0=Circle, 1.0=Square, 2.0=Triangle, 3.0=Star,
                       // 4.0=Diamond, 5.0=Pentagon, 6.0=Hexagon, 7.0=Octagon
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
    @location(0) @interpolate(flat) instance_idx: u32,
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

// ---------------------------
// SDF Helper Functions (For Primitives)
// ---------------------------
/// Signed distance function for perfect circle
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

/// Signed distance function for square (axis-aligned)
fn sd_square(p: vec2<f32>, r: f32) -> f32 {
    let d = abs(p) - vec2(r, r);
    // Fixed: use vec2(0.0) instead of scalar 0.0 for type consistency
    return length(max(d, vec2(0.0, 0.0))) + min(max(d.x, d.y), 0.0);
}

/// Signed distance function for diamond (rotated square)
fn sd_diamond(p: vec2<f32>, r: f32) -> f32 {
    return abs(p.x) + abs(p.y) - r;
}

/// Signed distance function for equilateral triangle
fn sd_triangle(p: vec2<f32>, r: f32) -> f32 {
    let k = vec2(-0.8660254, 0.5);
    var pt = abs(p);
    pt -= 2.0 * min(dot(pt, k), 0.0) * k;
    pt -= vec2(clamp(pt.x, -r * k.y, r * k.y), r);
    return length(pt) * sign(pt.y);
}

/// Signed distance function for 5-pointed star (matches Rust calculate_star)
fn sd_star(p: vec2<f32>, r: f32) -> f32 {
    let k1 = vec2(-0.9511, 0.3090);
    let k2 = vec2(0.5878, 0.8090);
    var pt = abs(p);
    pt -= 2.0 * min(dot(pt, k1), 0.0) * k1;
    pt -= 2.0 * min(dot(pt, k2), 0.0) * k2;
    pt -= vec2(clamp(pt.x, -r * 0.4, r * 0.4), r * 0.85);
    return length(pt) * sign(pt.y);
}

/// Signed distance function for regular pentagon
fn sd_pentagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec2(-0.6882, 0.7265);
    var pt = abs(p);
    pt -= 2.0 * min(dot(pt, k), 0.0) * k;
    pt -= vec2(clamp(pt.x, -r * k.y, r * k.y), r);
    return length(pt) * sign(pt.y);
}

/// Signed distance function for regular hexagon
fn sd_hexagon(p: vec2<f32>, r: f32) -> f32 {
    let k = vec2(-0.8660, 0.5);
    let kk = vec2(0.8660, 0.5);
    var pt = abs(p);
    pt -= 2.0 * min(dot(pt, k), 0.0) * k;
    pt -= 2.0 * min(dot(pt, kk), 0.0) * kk;
    pt -= vec2(clamp(pt.x, -r * 0.5, r * 0.5), r);
    return length(pt) * sign(pt.y);
}

/// Signed distance function for regular octagon
fn sd_octagon(p: vec2<f32>, r: f32) -> f32 {
    let s = vec2(0.7071, 0.7071);
    var pt = abs(p);
    pt -= 2.0 * min(dot(pt, s), 0.0) * s;
    pt -= vec2(clamp(pt.x, -r * 0.4142, r * 0.4142), r);
    return length(pt) * sign(pt.y);
}

/// Unified shape selector using shape_type (1:1 match with Rust GpuPoint)
fn sd_shape(p: vec2<f32>, radius: f32, shape_type: f32) -> f32 {
    if (shape_type == 0.0) { return sd_circle(p, radius); }
    if (shape_type == 1.0) { return sd_square(p, radius); }
    if (shape_type == 2.0) { return sd_triangle(p, radius); }
    if (shape_type == 3.0) { return sd_star(p, radius); }
    if (shape_type == 4.0) { return sd_diamond(p, radius); }
    if (shape_type == 5.0) { return sd_pentagon(p, radius); }
    if (shape_type == 6.0) { return sd_hexagon(p, radius); }
    if (shape_type == 7.0) { return sd_octagon(p, radius); }
    
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
        case 0u: { quad = vec2(0.0, 0.0); }
        case 1u: { quad = vec2(1.0, 0.0); }
        case 2u: { quad = vec2(0.0, 1.0); }
        case 3u: { quad = vec2(1.0, 1.0); }
        default: { quad = vec2(0.0); }
    }

    let scale = uniforms.scale_factor;
    let pos = vec2(r.x, r.y) * scale + quad * vec2(r.width, r.height) * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    let ndc = vec4((pos.x/sw)*2.0-1.0, 1.0-(pos.y/sh)*2.0, 0.0, 1.0);

    var out: RectOutput;
    out.clip_pos = ndc;
    out.instance_idx = ii;
    return out;
}

@fragment
fn rect_fs(in: RectOutput) -> @location(0) vec4<f32> {
    let r = rects[in.instance_idx];
    return vec4(r.r, r.g, r.b, r.a);
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