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

/// Polygon data (draw_polygon: symmetric markers - triangle, hexagon, diamond)
struct PolygonData {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    radius: f32,
    sides: f32,        // 3=triangle, 4=square, 5=pentagon, 6=hexagon
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
    @location(1) @interpolate(flat) is_fill: f32,
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
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_regular_polygon(p: vec2<f32>, r: f32, sides: f32) -> f32 {
    let pi = 3.14159265359;
    let angle = 2.0 * pi / sides;
    let a = atan2(p.y, p.x) + pi;
    let sector = floor(0.5 + a / angle) * angle - a;
    let seg = vec2(cos(sector), sin(sector)) * length(p);
    return length(seg - vec2(clamp(seg.x, -r, r), 0.0)) * sign(seg.y - r);
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
    
    let alpha = 1.0 - smoothstep(-fwidth(dist), fwidth(dist), dist);
    if (alpha <= 0.0) { discard; }
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
    let dist = sd_regular_polygon(local, r, poly.sides);
    
    let alpha = 1.0 - smoothstep(-fwidth(dist), fwidth(dist), dist);
    if (alpha <= 0.0) { discard; }
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