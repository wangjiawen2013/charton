// ============================================================================
// Charton WGPU Shader: Unified Rendering Primitives
// Primitives: Circle, Rect, Line, Polygon, GradientRect, Path(Polyline/Area)
// Strictly Compliant with RenderBackend Contract
// ============================================================================

// ----------------------------------------------------------------------------
// Storage Buffer Data Structures (Semantically Separated)
// ----------------------------------------------------------------------------

/// Circle data (draw_circle: exclusive for circular markers/points)
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

/// Rectangle data (draw_rect: bars, boxes, axis backgrounds)
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
    pad1: f32,
    pad2: f32,
    pad3: f32,
};

/// Polygon data (draw_polygon: symmetric markers - triangle, hexagon, diamond, star)
/// Fed directly via traditional Vertex Buffer stream (no dedicated storage slot needed)
struct PolygonData {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    radius: f32,
    shape_type: f32,
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

// ----------------------------------------------------------------------------
// Uniform Buffer (Global Render State)
// ----------------------------------------------------------------------------
struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
    _padding: f32,
};

// ============================================================================
// Resource Bind Group Layouts
// ============================================================================

// ----------------------------------------------------------------------------
// Group 0: Global Instanced & Batched Primitives (Kept intact to preserve indices)
// ----------------------------------------------------------------------------
@group(0) @binding(0) var<storage, read> circles: array<PointData>;
@group(0) @binding(1) var<storage, read> rects: array<RectData>;
@group(0) @binding(2) var<storage, read> lines: array<LineData>;
// Note: binding(3) is skipped intentionally to match the traditional Vertex Buffer Polygon input
@group(0) @binding(4) var<storage, read> gradient_rects: array<GradientRectData>;
@group(0) @binding(5) var<uniform> uniforms: Uniforms;

// ----------------------------------------------------------------------------
// Group 1: Dedicated High-Throughput Stream (Exclusive for Pure GPU Line Extrusion)
// ----------------------------------------------------------------------------

/// Represents a raw coordinate vertex along the dynamic polyline path
struct PathPointData {
    x: f32,
    y: f32,
};

/// Global rendering aesthetics for the paths (Shifted to Storage-compliant layout)
struct PathStyle {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    thickness: f32,
    _pad0: f32, // Padding fields to guarantee 16-byte structural boundaries
    _pad1: f32,
    _pad2: f32,
};

/// Structural draw arguments routing layout for monolithic global lookup
struct PathArgs {
    start_point_idx: u32,
    style_idx: u32,
    _pad0: u32, // Structural padding fields
    _pad1: u32,
};

// Monolithic Global Storage Bindings (Matches Scheme A Host Layout Contract)
@group(1) @binding(0) var<storage, read> path_points: array<PathPointData>;
@group(1) @binding(1) var<storage, read> path_styles: array<PathStyle>;
@group(1) @binding(2) var<storage, read> path_args: array<PathArgs>;

// ---------------------------
// Vertex Output Structures
// ---------------------------
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

/// Output structure passed from the vertex shader through rasterization into the fragment shader
struct PathOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// ============================================================================
// Analytical SDF Implementation – ONLY for Circle
// All other shapes (triangle, star, hexagon, etc.) use CPU-generated vertices.
// ============================================================================

/// Signed Distance Field for a perfect circle.
/// p: Local fragment position relative to shape center
/// r: Radius of the circle
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
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
    // Use a slightly larger quad than the circle itself to avoid clipping SDF anti-aliasing.
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
    
    // Base SDF: Distance to the circle boundary (inside < 0, boundary = 0, outside > 0)
    let dist = length(local) - r;
    
    // Filter width for hardware-based anti-aliasing (AA)
    let aa = fwidth(dist);
    
    // ========================================================================
    // 1. Calculate Fill Layer
    // ========================================================================
    let fill_alpha = 1.0 - smoothstep(-aa, aa, dist);
    let fill_color = vec4(circle.fill_r, circle.fill_g, circle.fill_b, circle.fill_a * fill_alpha);
    
    // ========================================================================
    // 2. Calculate Stroke Layer (Centered Alignment)
    // ========================================================================
    let half_stroke = (circle.stroke_width * uniforms.scale_factor) * 0.5;
    // abs(dist) gives the distance to the boundary shell. 
    // Subtracting half_stroke yields the SDF for a hollow ring/annulus.
    let stroke_dist = abs(dist) - half_stroke;
    let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_dist);
    let stroke_color = vec4(circle.stroke_r, circle.stroke_g, circle.stroke_b, circle.stroke_a * stroke_alpha);
    
    // ========================================================================
    // 3. Alpha Compositing (Stroke Over Fill)
    // ========================================================================
    // Standard Porter-Duff Over formula: out_a = src_a + dst_a * (1.0 - src_a)
    let out_a = stroke_color.a + fill_color.a * (1.0 - stroke_color.a);
    
    // Early discard for fully transparent fragments to optimize depth/blend performance
    if (out_a <= 0.01) {
        discard;
    }
    
    // Combine RGB colors using premultiplied alpha math, then normalize by out_a
    let out_rgb = (stroke_color.rgb * stroke_color.a + fill_color.rgb * fill_color.a * (1.0 - stroke_color.a)) / out_a;
    
    return vec4(out_rgb, out_a);
}

// ---------------------------
// 2. Rectangle Pipeline (draw_rect: Pure Filled Bars/Boxes/Backgrounds)
// ---------------------------
/// Signed Distance Field for a rectangle with optional rounded corners.
/// p: Local fragment position relative to the rectangle center.
/// b: Half-extents of the rectangle (width/2, height/2).
/// r: Corner radius.
fn sd_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let d = abs(p) - b + vec2(r);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2(0.0))) - r;
}

@vertex
fn rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> RectOutput {
    let r = rects[ii];
    let scale = uniforms.scale_factor;
    
    // Inflate the quad slightly beyond physical bounds to prevent stroke clipping and enable smooth AA.
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
    
    // Calculate the geometric center and half-extents of the rectangle
    let center = vec2(r.x + r.width * 0.5, r.y + r.height * 0.5) * scale;
    let half_extents = vec2(r.width * 0.5, r.height * 0.5) * scale;
    
    // Local fragment coordinates transformed relative to the rect center
    let local = in.screen_pos - center;
    
    // Compute the base SDF for the rectangle (handles corner radius automatically)
    let dist = sd_rounded_rect(local, half_extents, r.corner_radius * scale);
    
    // Filter width for hardware-based anti-aliasing (AA)
    let aa = fwidth(dist);

    // ========================================================================
    // 1. Calculate Fill Layer
    // ========================================================================
    let fill_alpha = 1.0 - smoothstep(-aa, aa, dist);
    let fill_color = vec4(r.fill_r, r.fill_g, r.fill_b, r.fill_a * fill_alpha);

    // ========================================================================
    // 2. Calculate Stroke Layer (Centered Alignment)
    // ========================================================================
    let half_stroke = (r.stroke_width * scale) * 0.5;
    // Absolutizing the distance field creates an interior/exterior bounding corridor for the frame
    let stroke_dist = abs(dist) - half_stroke;
    let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_dist);
    let stroke_color = vec4(r.stroke_r, r.stroke_g, r.stroke_b, r.stroke_a * stroke_alpha);

    // ========================================================================
    // 3. Alpha Compositing (Stroke Over Fill)
    // ========================================================================
    let out_a = stroke_color.a + fill_color.a * (1.0 - stroke_color.a);
    
    if (out_a <= 0.01) {
        discard;
    }

    // Blend RGB channels based on premultiplied alpha math
    let out_rgb = (stroke_color.rgb * stroke_color.a + fill_color.rgb * fill_color.a * (1.0 - stroke_color.a)) / out_a;

    return vec4(out_rgb, out_a);
}

// ---------------------------
// 3. Line Segment Pipeline (draw_line: Axis/Grid/Ticks)
// ---------------------------
@vertex
fn line_vs(
    @builtin(vertex_index) vi: u32,       // Current vertex index within the primitive (0 to 3 for a quad)
    @builtin(instance_index) ii: u32      // Index of the current line segment in the Storage Buffer
) -> LineOutput {
    // 1. Fetch data and apply High-DPI / Retargeting scaling factor
    let line = lines[ii];
    let scale = uniforms.scale_factor;
    let p1 = vec2(line.x1, line.y1) * scale;
    let p2 = vec2(line.x2, line.y2) * scale;
    
    // 2. Compute direction vector with a safety guard against zero-length segments (prevents NaN)
    var dir = p2 - p1;
    if (length(dir) < 0.0001) {
        dir = vec2<f32>(1.0, 0.0); // Fallback direction to prevent division by zero
    }
    dir = normalize(dir);
    
    // 3. Calculate perpendicular normal vector, scaled by half-width to project outward
    let perp = vec2(-dir.y, dir.x) * (line.width * 0.5 * scale);

    // 4. Extrude vertices dynamically on-chip using TriangleStrip topology
    var pos = vec2<f32>();
    switch vi {
        case 0u: { pos = p1 + perp; } // Start point: left expansion
        case 1u: { pos = p1 - perp; } // Start point: right expansion
        case 2u: { pos = p2 + perp; } // End point: left expansion
        case 3u: { pos = p2 - perp; } // End point: right expansion
        default: { pos = p1; }
    }

    // 5. Convert screen-space pixel coordinates to Normalized Device Coordinates (NDC)
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    // Map X to [-1, 1], and invert Y axis to match WebGPU specifications
    let ndc = vec4((pos.x / sw) * 2.0 - 1.0, 1.0 - (pos.y / sh) * 2.0, 0.0, 1.0);

    // 6. Assemble output payload for the rasterizer
    var out: LineOutput;
    out.clip_pos = ndc;
    out.instance_idx = ii; // Forward instance ID so the Fragment Shader can resolve colors
    return out;
}

@fragment
fn line_fs(in: LineOutput) -> @location(0) vec4<f32> {
    let line = lines[in.instance_idx];
    return vec4(line.r, line.g, line.b, line.a);
}

// ---------------------------
// 4. Polygon Pipeline (draw_polygon: triangle/star/diamond/hexagon etc.)
// Receives CPU-precomputed vertices - NO GPU-side shape generation
// ---------------------------
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
    return vec4(in.color.r, in.color.g, in.color.b, in.color.a);
}

// ----------------------------------------------------------------------------
// 5. Gradient Rectangle Pipeline (draw_gradient_rect)
// ----------------------------------------------------------------------------

@vertex
fn grad_rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> GradientRectOutput {
    let r = gradient_rects[ii];
    var quad = vec2<f32>();
    var uv = vec2<f32>();
    
    // Physical geometry alignment with SVG specifications:
    // Top-Left / Top-Right vertices correspond to uv.y = 0.0
    // Bottom-Left / Bottom-Right vertices correspond to uv.y = 1.0
    switch vi {
        case 0u: { quad = vec2(r.x, r.y);            uv = vec2(0.0, 0.0); } // Top-Left
        case 1u: { quad = vec2(r.x + r.width, r.y);    uv = vec2(1.0, 0.0); } // Top-Right
        case 2u: { quad = vec2(r.x, r.y + r.height);   uv = vec2(0.0, 1.0); } // Bottom-Left
        case 3u: { quad = vec2(r.x + r.width, r.y + r.height); uv = vec2(1.0, 1.0); } // Bottom-Right
        default: { quad = vec2(r.x, r.y); }
    }

    let scale = uniforms.scale_factor;
    let screen_pos = quad * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    
    // Coordinate transformation into global Normalized Device Coordinates (NDC) space
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
    
    // Rely exclusively on the explicit 'angle' parameter passed from the Rust CPU side.
    // If angle > 0.0 (e.g., FRAC_PI_2 for vertical bars), interpolate along the Y-axis (uv.y).
    // Otherwise, interpolate along the X-axis (uv.x).
    var mix_val = in.uv.x;
    if (r.angle > 0.0) {
        mix_val = in.uv.y;
    }

    // Direct linear color interpolation to prevent mid-tone gamma color distortion
    let src_r = mix(r.start_r, r.end_r, mix_val);
    let src_g = mix(r.start_g, r.end_g, mix_val);
    let src_b = mix(r.start_b, r.end_b, mix_val);
    let src_a = mix(r.start_a, r.end_a, mix_val) * r.opacity;

    // Alpha Premultiplication: Multiplies RGB components by Alpha 
    // to comply with Web Canvas alpha blending guidelines and prevent dark fringing artifacts.
    return vec4<f32>(src_r * src_a, src_g * src_a, src_b * src_a, src_a);
}

// ============================================================================
// 6. Pure GPU Polyline Extrusion Pipeline (draw_path - Simple Branch)
// ============================================================================
@vertex
fn path_simple_vs(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32 // Forwarded path_idx from pass.draw boundaries
) -> PathOutput {
    // 1. Resolve monolithic absolute addresses via the routing table
    let args = path_args[ii];
    let path_style = path_styles[args.style_idx];

    // Each line segment (quad) is constructed via 6 virtual vertices (2 triangles)
    let segment_idx = vi / 6u;
    let local_vertex_idx = vi % 6u;

    // Fetch p0 and p1 using calculated global offsets from the streaming queues
    let p0_idx = args.start_point_idx + segment_idx;
    let p1_idx = p0_idx + 1u;

    let p0 = path_points[p0_idx];
    let p1 = path_points[p1_idx];

    // Data defense: if any coordinate is NaN, collapse the triangle to eliminate rendering artifacts
    if (p0.x != p0.x || p0.y != p0.y || p1.x != p1.x || p1.y != p1.y) {
        var out: PathOutput;
        out.clip_pos = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    }

    // Calculate the direction vector of the current segment
    let delta = vec2<f32>(p1.x - p0.x, p1.y - p0.y);
    var current_dir = normalize(delta);
    
    // Prevent division-by-zero if the two points overlap perfectly
    if (length(delta) == 0.0) {
        current_dir = vec2<f32>(1.0, 0.0);
    }
    
    // Calculate the right-hand orthogonal normal vector
    let normal = vec2<f32>(-current_dir.y, current_dir.x);

    var raw_pos = vec2<f32>(0.0, 0.0);
    var extrusion_side = 0.0; // 1.0 extends along the normal, -1.0 extends opposite to the normal

    // Finite state machine: Map the 6 virtual vertices to a structured Triangle List Quad
    switch local_vertex_idx {
        case 0u: { raw_pos = vec2(p0.x, p0.y); extrusion_side = 1.0; }  // p0 Left
        case 1u: { raw_pos = vec2(p0.x, p0.y); extrusion_side = -1.0; } // p0 Right
        case 2u: { raw_pos = vec2(p1.x, p1.y); extrusion_side = 1.0; }  // p1 Left
        
        case 3u: { raw_pos = vec2(p1.x, p1.y); extrusion_side = 1.0; }  // p1 Left
        case 4u: { raw_pos = vec2(p0.x, p0.y); extrusion_side = -1.0; } // p0 Right
        case 5u: { raw_pos = vec2(p1.x, p1.y); extrusion_side = -1.0; } // p1 Right
        default: {}
    }

    // Extrude outward by half of the thickness in physical pixel space
    let ext_pos = raw_pos + normal * (path_style.thickness * 0.5 * extrusion_side);

    // Coordinate transformation into global uniform space
    let scale = uniforms.scale_factor;
    let screen_pos = ext_pos * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;

    // Standard Normalized Device Coordinate (NDC) conversion with Y-axis inversion
    let ndc_x = (screen_pos.x / sw) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_pos.y / sh) * 2.0;

    var out: PathOutput;
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = vec4<f32>(path_style.r, path_style.g, path_style.b, path_style.a);
    return out;
}

@fragment
fn path_simple_fs(in: PathOutput) -> @location(0) vec4<f32> {
    // Strictly adheres to semantic separation contract
    return in.color;
}