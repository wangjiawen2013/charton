# WGPU TEXT
纯wgpu文本渲染是非常难的，这是一个不太成功的尝试，能显示文本，但位置、色彩、清晰度、角度都不太理想，作为备份放在这儿了，以备以后参考。

## chart.wgsl

```wgsl
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
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    radius: f32,
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

/// Individual Glyph Instance (Instanced Text Rendering)
/// Strictly maps to your CPU batching structure with strict alignment
struct GlyphInstanceData {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    uv_min_x: f32,
    uv_min_y: f32,
    uv_max_x: f32,
    uv_max_y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
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

// Group 1: High-Throughput Stream (Pure GPU Line Extrusion)
@group(1) @binding(0) var<storage, read> path_points: array<PathPointData>;
@group(1) @binding(1) var<storage, read> path_styles: array<PathStyle>;
@group(1) @binding(2) var<storage, read> path_args: array<PathArgs>;

// Group 2: High-Performance Font Cache & Instance Stream
// Maps perfectly to the architecture requested in your wgpu.rs note
@group(2) @binding(0) var<storage, read> glyph_instances: array<GlyphInstanceData>;
@group(2) @binding(1) var font_atlas_texture: texture_2d<f32>;
@group(2) @binding(2) var font_atlas_sampler: sampler;

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

// Text Output
struct TextOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
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
    let dist = sd_circle(local, r);
    
    // Smooth anti-aliasing
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    if (alpha <= 0.01) { discard; }
    
    // Swap Red and Blue channels to match the Bgra8Unorm surface format.
    return vec4(circle.b, circle.g, circle.r, circle.a * alpha);
}

// ---------------------------
// 2. Rectangle Pipeline (draw_rect: Pure Filled Bars/Boxes/Backgrounds)
// ---------------------------
@vertex
fn rect_vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> RectOutput {
    let r = rects[ii];
    var pos = vec2<f32>();
    
    // Strictly align with the actual rectangle bounds without any inflation for perfect hardware rasterization.
    switch vi {
        case 0u: { pos = vec2(r.x, r.y); }
        case 1u: { pos = vec2(r.x + r.width, r.y); }
        case 2u: { pos = vec2(r.x, r.y + r.height); }
        case 3u: { pos = vec2(r.x + r.width, r.y + r.height); }
        default: { pos = vec2(r.x, r.y); }
    }

    let scale = uniforms.scale_factor;
    let screen_pos = pos * scale;
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
    
    // Removed all bounds checks and discards to maximize GPU fill-rate performance.
    // Swap Red and Blue channels to match the Bgra8Unorm surface format.
    return vec4(r.b, r.g, r.r, r.a);
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
    // Swap Red and Blue channels to match the Bgra8Unorm surface format.
    return vec4(line.b, line.g, line.r, line.a);
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
    return vec4(in.color.b, in.color.g, in.color.r, in.color.a);
}

// ---------------------------
// 5. Gradient Rectangle Pipeline (draw_gradient_rect)
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

// ============================================================================
// 6. Pure GPU Polyline Extrusion Pipeline (draw_path - Simple Branch)
// ============================================================================
@vertex
fn path_simple_vs(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32 // 🌟 Forwarded path_idx from pass.draw boundaries
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
    out.color = vec4<f32>(path_style.b, path_style.g, path_style.r, path_style.a);
    return out;
}

@fragment
fn path_simple_fs(in: PathOutput) -> @location(0) vec4<f32> {
    // Strictly adheres to semantic separation contract
    return in.color;
}

// ---------------------------
// 7. Text Pipeline (draw_text)
// ---------------------------
// Text Instanced Vertex Shader
// Processes full string blocks into single-pass hardware drawn instanced quads
@vertex
fn vs_text(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32,
) -> TextOutput {
    let glyph = glyph_instances[instance_idx];
    
    // Generate a standard quad using vertex_idx (0 to 5)
    var local_pos: vec2<f32>;
    var uv: vec2<f32>;
    
    switch vertex_idx {
        case 0u: { local_pos = vec2(0.0, 0.0); uv = vec2(glyph.uv_min_x, glyph.uv_min_y); } // Top-Left
        case 1u: { local_pos = vec2(0.0, 1.0); uv = vec2(glyph.uv_min_x, glyph.uv_max_y); } // Bottom-Left
        case 2u: { local_pos = vec2(1.0, 0.0); uv = vec2(glyph.uv_max_x, glyph.uv_min_y); } // Top-Right
        
        case 3u: { local_pos = vec2(1.0, 0.0); uv = vec2(glyph.uv_max_x, glyph.uv_min_y); } // Top-Right
        case 4u: { local_pos = vec2(0.0, 1.0); uv = vec2(glyph.uv_min_x, glyph.uv_max_y); } // Bottom-Left
        case 5u: { local_pos = vec2(1.0, 1.0); uv = vec2(glyph.uv_max_x, glyph.uv_max_y); } // Bottom-Right
        default: {}
    }
    
    // Compute absolute screen position
    let world_pos = vec2<f32>(
        glyph.x + local_pos.x * glyph.width,
        glyph.y + local_pos.y * glyph.height
    );
    
    // Coordinate transformation to Clip Space
    let scale = uniforms.scale_factor;
    let screen_pos = world_pos * scale;
    let sw = uniforms.screen_width * scale;
    let sh = uniforms.screen_height * scale;
    
    var out: TextOutput;
    out.clip_pos = vec4<f32>(
        (screen_pos.x / sw) * 2.0 - 1.0,
        1.0 - (screen_pos.y / sh) * 2.0,
        0.0,
        1.0
    );
    out.uv = uv;
    out.color = vec4<f32>(glyph.r, glyph.g, glyph.b, glyph.a);
    return out;
}

// Text Fragment Shader
// Highly optimized text sampler. Pulls alpha channel from texture cache and shades smoothly.
@fragment
fn fs_text(in: TextOutput) -> @location(0) vec4<f32> {
    // We sample a single channel texture (A8/R8Unorm) allocated as our font atlas cache
    let tex_color = textureSample(font_atlas_texture, font_atlas_sampler, in.uv);
    
    // Font atlas stores alpha inside the Red component (R8_Unorm format)
    let alpha_mask = tex_color.r;
    
    // Perform unified hardware tinting and alpha blending
    return vec4<f32>(in.color.rgb, in.color.a * alpha_mask);
}
```

## wgpu.rs

```rust
//! WGPU rendering backend implementation for 2D primitive rendering (circles, lines, rects, polygons, gradients, text)
//! Provides GPU-optimized data structures and render pipelines aligned with WGSL shaders

use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use crate::visual::color::SingleColor;
use ab_glyph::{Font, FontArc, ScaleFont};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
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

// ----------------------------------------------------------------------------
// Pure GPU Polyline Extrusion Layouts (Group 1 Spec)
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
    pub _pad0: f32, // Structural alignment padding (16-byte boundary)
    pub _pad1: f32,
    pub _pad2: f32,
}

/// Meta arguments guiding the Vertex Shader where to look up data
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPathArgs {
    pub start_point_idx: u32,
    pub style_idx: u32,
    pub _pad0: u32, // Structural alignment padding
    pub _pad1: u32,
}

/// Individual Glyph Instance Data (Strict 1:1 WGSL Alignment)
/// Matches `GlyphInstanceData` in chart.wgsl
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuGlyph {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv_min_x: f32,
    pub uv_min_y: f32,
    pub uv_max_x: f32,
    pub uv_max_y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    // Bearing offsets: distance from pen origin (baseline) to glyph top-left
    pub o_x: f32,
    pub o_y: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

/// Helper structure to track metadata and texture coordinates
/// for a single rasterized glyph inside the global GPU font atlas.
#[derive(Clone, Debug)]
pub struct CachedGlyphInfo {
    /// Normalized UV coordinates of the top-left corner in the texture atlas [u, v]
    pub uv_min: [f32; 2],
    /// Normalized UV coordinates of the bottom-right corner in the texture atlas [u, v]
    pub uv_max: [f32; 2],
    /// Physical pixel width of the glyph rectangle bounding box
    pub width: f32,
    /// Physical pixel height of the glyph rectangle bounding box
    pub height: f32,
    /// Horizontal bearing offset (distance from pen origin to glyph left edge)
    pub o_x: f32,
    /// Vertical bearing offset (distance from pen origin to glyph top edge)
    pub o_y: f32,
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
    /// Monolithic GPU indexed simple path batch token
    PathSimple {
        /// Lookup offset index into the global GpuPathArgs array
        path_idx: u32,
        /// Number of raw points contained within this single polyline
        point_count: u32,
    },
    Text {
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
    #[allow(dead_code)]
    PathSimple,
    Text,
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

    // Line primitive resources
    line_pipeline: wgpu::RenderPipeline,
    line_buffer: wgpu::Buffer,
    pending_lines: Vec<GpuLine>,
    uploaded_line_count: u32,

    // Polygon primitive resources
    polygon_pipeline: wgpu::RenderPipeline,
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

    // Specialized render pipeline for text instancing quads
    text_pipeline: wgpu::RenderPipeline,
    text_instance_buffer: wgpu::Buffer,
    pending_glyphs: Vec<GpuGlyph>,
    text_atlas_texture: wgpu::Texture,
    text_atlas_view: wgpu::TextureView,
    text_atlas_sampler: wgpu::Sampler,
    text_bind_group_layout: wgpu::BindGroupLayout,

    /// CPU font parser and glyph metric computer via ab_glyph
    font: FontArc,
    /// Fast deduplication dictionary map for caching character raster results
    font_cache: HashMap<(char, u32), CachedGlyphInfo>,
    /// Incremental texture coordinate trackers for custom packing layout
    atlas_current_x: u32,
    atlas_current_y: u32,
    atlas_max_row_height: u32,

    /// Interleaved batch queue to preserve rendering order
    batches: Vec<DrawBatch>,

    /// Running instance count for rendering primitives (used as buffer offset)
    current_circle_count: u32,
    current_rect_count: u32,
    current_line_count: u32,
    current_polygon_index_count: u32,
    current_grad_rect_count: u32,
    /// Total number of individual characters processed in the current frame
    current_glyph_count: u32,
    /// Device pixel ratio / HiDPI scale factor passed from caller
    scale_factor: f32,
}

impl WgpuBackend {
    /// Creates a new WGPU rendering backend with multi-group high-throughput pipelines.
    ///
    /// # Resource Binding Architecture:
    /// - `@group(0)`: Global Batched Primitives (Circles, Rectangles, Standard Lines, Uniforms)
    /// - `@group(1)`: Dedicated High-Throughput Stream (Pure GPU Path Extrusion via Raw Coordinates)
    /// - `@group(2)`: Dedicated Text & Glyph Atlas Stream (Reserved for future SDF/Atlas rendering)
    pub async fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        screen_width: u32,
        screen_height: u32,
        scale_factor: f32,
    ) -> Self {
        // 1. Load WGSL shader module (chart.wgsl contains all primitive shaders)
        let shader = device.create_shader_module(wgpu::include_wgsl!("chart.wgsl"));

        // 2. Initialize global uniform buffer with screen dimensions and scaling factor
        let uniforms = Uniforms {
            screen_width: screen_width as f32,
            screen_height: screen_height as f32,
            scale_factor,
            _padding: 0.0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer - Group 0 Binding 5"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ====================================================================
        // RESOURCE BIND GROUP LAYOUT DECLARATIONS (Hardware Contracts)
        // ====================================================================

        // Create main bind group layout (@group(0) in chart.wgsl)
        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Main Primitive Bind Group Layout 0"),
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

        // Create high-throughput path stream bind group layout (@group(1) in chart.wgsl)
        let path_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Path Global Storage Bind Group Layout 1"),
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

        // 🌟 CRITICAL FIX: Create text pipeline and get the REAL text layout here,
        // BEFORE we assemble the global render_pipeline_layout.
        let (
            text_bind_group_layout,
            text_pipeline,
            text_atlas_texture,
            text_atlas_view,
            text_atlas_sampler,
        ) = Self::create_text_pipeline(&device, &main_bind_group_layout);

        // ====================================================================
        // GLOBAL PIPELINE LAYOUT ARCHITECTURE (Multi-Group Mapping)
        // ====================================================================
        let path_simple_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Path Simple Pipeline Layout"),
                bind_group_layouts: &[Some(&main_bind_group_layout), Some(&path_bind_group_layout)],
                immediate_size: 0,
            });

        // ====================================================================
        // RENDER PIPELINES COMPILATION
        // ====================================================================
        let circle_pipeline =
            Self::create_circle_pipeline(&device, &shader, &main_bind_group_layout);
        let (_line_bg_layout, line_pipeline) =
            Self::create_line_pipeline(&device, &shader, &main_bind_group_layout);
        let rect_pipeline = Self::create_rect_pipeline(&device, &shader, &main_bind_group_layout);
        let polygon_pipeline =
            Self::create_polygon_pipeline(&device, &shader, &main_bind_group_layout);
        let (_grad_bg_layout, gradient_rect_pipeline) =
            Self::create_gradient_rect_pipeline(&device, &shader, &main_bind_group_layout);

        // Compile the pure on-chip vertex generation line extrusion pipeline (Simple Path Pipeline)
        let texture_format = wgpu::TextureFormat::Rgba8Unorm;
        let path_simple_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Path Simple Hardware Extrusion Pipeline"),
            layout: Some(&path_simple_pipeline_layout),
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
                    format: texture_format,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // ====================================================================
        // DEVICE STORAGE VRAM BACKING BUFFERS INITIAL ALLOCATIONS
        // ====================================================================
        let dummy_circles = Self::create_dummy_buffer::<GpuPoint>(&device);
        let dummy_rects = Self::create_dummy_buffer::<GpuRect>(&device);
        let dummy_lines = Self::create_dummy_buffer::<GpuLine>(&device);
        let dummy_grad_rects = Self::create_dummy_buffer::<GpuGradientRect>(&device);
        let dummy_polygon_vertices = Self::create_dummy_buffer::<PathVertex>(&device);
        let dummy_polygon_indices = Self::create_dummy_buffer::<u16>(&device);

        // Bind main resources
        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Primitive Bind Group 0"),
            layout: &main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(dummy_circles.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(dummy_rects.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(dummy_lines.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(dummy_grad_rects.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
                },
            ],
        });

        // Pre-allocate GPU memory for text instances (up to 4096 characters per frame)
        let text_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Instance Storage Buffer"),
            size: (std::mem::size_of::<GpuGlyph>() * 4096) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Load fallback scalable TTF font data from the embedded binary payload
        let font_bytes = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        let font = ab_glyph::FontArc::try_from_slice(font_bytes)
            .expect("CRITICAL: Failed to parse embedded fallback TTF font data");

        // Initialize empty runtime glyph deduplication dictionary cache
        let font_cache = std::collections::HashMap::new();

        // ====================================================================
        // BACKEND STRUCTURE ASSEMBLY
        // ====================================================================
        Self {
            device,
            queue,
            uniform_buffer,
            main_bind_group,
            main_bind_group_layout,

            circle_pipeline,
            circle_buffer: dummy_circles,
            pending_circles: Vec::with_capacity(30_000),
            uploaded_circle_count: 0,

            rect_pipeline,
            rect_buffer: dummy_rects,
            pending_rects: Vec::with_capacity(10_000),
            uploaded_rect_count: 0,

            polygon_pipeline,
            polygon_vertex_buffer: dummy_polygon_vertices,
            polygon_index_buffer: dummy_polygon_indices,
            pending_polygon_vertices: Vec::with_capacity(50_000),
            pending_polygon_indices: Vec::with_capacity(100_000),
            uploaded_polygon_index_count: 0,

            line_pipeline,
            line_buffer: dummy_lines,
            pending_lines: Vec::with_capacity(10_000),
            uploaded_line_count: 0,

            gradient_rect_pipeline,
            gradient_rect_buffer: dummy_grad_rects,
            pending_gradient_rects: Vec::with_capacity(10_000),
            uploaded_gradient_rect_count: 0,

            path_simple_pipeline,
            path_bind_group_layout,
            pending_path_points: Vec::with_capacity(100_000),
            pending_path_styles: Vec::with_capacity(1024),
            pending_path_args: Vec::with_capacity(1024),

            text_pipeline,
            text_instance_buffer,
            pending_glyphs: Vec::with_capacity(512),
            text_atlas_texture,
            text_atlas_view,
            text_atlas_sampler,
            text_bind_group_layout,

            font,
            font_cache,
            atlas_current_x: 4,
            atlas_current_y: 4,
            atlas_max_row_height: 0,

            batches: Vec::with_capacity(1024),
            current_circle_count: 0,
            current_rect_count: 0,
            current_line_count: 0,
            current_polygon_index_count: 0,
            current_grad_rect_count: 0,
            current_glyph_count: 0,
            scale_factor,
        }
    }

    fn push_batch(&mut self, batch_type: BatchType, count: u32) {
        // Attempt to merge with the last batch if the type is the same
        let merged = match (self.batches.last_mut(), batch_type) {
            (Some(DrawBatch::Circle { count: c, .. }), BatchType::Circle) => { *c += count; true }
            (Some(DrawBatch::Rect { count: c, .. }), BatchType::Rect) => { *c += count; true }
            (Some(DrawBatch::Line { count: c, .. }), BatchType::Line) => { *c += count; true }
            (Some(DrawBatch::Polygon { index_count: c, .. }), BatchType::Polygon) => { *c += count; true }
            (Some(DrawBatch::GradientRect { count: c, .. }), BatchType::GradientRect) => { *c += count; true }
            (Some(DrawBatch::Text { count: c, .. }), BatchType::Text) => { *c += count; true }
            (_, BatchType::PathSimple) => false,
            _ => false,
        };

        if !merged {
            let new_batch = match batch_type {
                BatchType::Circle => DrawBatch::Circle { start: self.current_circle_count.saturating_sub(count), count },
                BatchType::Rect => DrawBatch::Rect { start: self.current_rect_count.saturating_sub(count), count },
                BatchType::Line => DrawBatch::Line { start: self.current_line_count.saturating_sub(count), count },
                BatchType::Polygon => DrawBatch::Polygon { index_start: self.current_polygon_index_count.saturating_sub(count), index_count: count },
                BatchType::GradientRect => DrawBatch::GradientRect { start: self.current_grad_rect_count.saturating_sub(count), count },
                BatchType::PathSimple => DrawBatch::PathSimple {
                    path_idx: (self.pending_path_args.len() as u32).saturating_sub(1),
                    point_count: count,
                },
                // Merge contiguous text glyph instances to minimize hardware draw dispatches
                BatchType::Text => DrawBatch::Text { start: self.current_glyph_count.saturating_sub(count), count },
            };
            self.batches.push(new_batch);
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
        self.pending_glyphs.clear();

        // Assuming you have a corresponding pending buffer for text
        // self.pending_text_vertices.clear();

        // 4. Reset uploaded counters
        // This is crucial! It tells the system that no data has been uploaded to GPU
        // for the new frame yet.
        self.uploaded_circle_count = 0;
        self.uploaded_rect_count = 0;
        self.uploaded_line_count = 0;
        self.uploaded_polygon_index_count = 0;
        self.uploaded_gradient_rect_count = 0;
        self.current_glyph_count = 0;

        // Clear our pure GPU path dynamic streaming queues:
        self.pending_path_points.clear();
        self.pending_path_styles.clear();
        self.pending_path_args.clear();
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

    // ============================================================================
    // Text Pipeline & Resource Initialization
    // ============================================================================
    /// 🌟 一体化创建文字系统所需的全部管线、布局、纹理图集与采样器
    /// 移除了旧版的 async 异步修饰，完美契合标准同步初始化流
    pub fn create_text_pipeline(
        device: &wgpu::Device,
        main_layout: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::BindGroupLayout, // _text_bg_layout_real
        wgpu::RenderPipeline,  // text_pipeline
        wgpu::Texture,         // text_atlas_texture
        wgpu::TextureView,     // text_atlas_view
        wgpu::Sampler,         // text_atlas_sampler
    ) {
        // --------------------------------------------------------------------
        // A. 定义文字本地绑定组布局 (@group(1))
        // --------------------------------------------------------------------
        let text_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Local Bind Group Layout (@group(1))"),
            entries: &[
                // Binding 0: 动态字形实例 Storage Buffer
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
                // Binding 1: 字体大图集单通道 Alpha 遮罩纹理
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // Binding 2: 线性纹理采样器
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // --------------------------------------------------------------------
        // B. 内联现代高 throughput 的 WGSL 文本着色器
        // --------------------------------------------------------------------
        let text_wgsl = r#"
            struct EngineGlobalData {
                screen_width: f32,
                screen_height: f32,
                scale_factor: f32,
                _padding: f32,
            };

            struct GpuGlyph {
                x: f32,
                y: f32,
                width: f32,
                height: f32,
                uv_min_x: f32,
                uv_min_y: f32,
                uv_max_x: f32,
                uv_max_y: f32,
                r: f32,
                g: f32,
                b: f32,
                a: f32,
                o_x: f32,
                o_y: f32,
                _pad0: f32,
                _pad1: f32,
            };

            struct TextInstances {
                glyphs: array<GpuGlyph>,
            };

            @group(0) @binding(5) var<uniform> global_data: EngineGlobalData;
            
            @group(1) @binding(0) var<storage, read> instance_data: TextInstances;
            @group(1) @binding(1) var t_atlas: texture_2d<f32>;
            @group(1) @binding(2) var s_atlas: sampler;

            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) uv: vec2<f32>,
                @location(1) color: vec4<f32>,
            };

            @vertex
            fn text_vs(
                @builtin(vertex_index) v_idx: u32,
                @builtin(instance_index) i_idx: u32,
            ) -> VertexOutput {
                let glyph = instance_data.glyphs[i_idx];
                var local_pos = vec2<f32>(0.0, 0.0);
                var uv = vec2<f32>(0.0, 0.0);

                // Interpret glyph.x,y as baseline pen origin. Convert to top-left by adding bearing offsets.
                let top_left = vec2<f32>(glyph.x + glyph.o_x, glyph.y + glyph.o_y);
                let corners = array<vec2<f32>, 6>(
                    vec2<f32>(0.0, 0.0),
                    vec2<f32>(0.0, 1.0),
                    vec2<f32>(1.0, 0.0),
                    vec2<f32>(0.0, 0.0),
                    vec2<f32>(1.0, 0.0),
                    vec2<f32>(1.0, 1.0),
                );
                let uvs = array<vec2<f32>, 6>(
                    vec2<f32>(glyph.uv_min_x, glyph.uv_min_y),
                    vec2<f32>(glyph.uv_min_x, glyph.uv_max_y),
                    vec2<f32>(glyph.uv_max_x, glyph.uv_min_y),
                    vec2<f32>(glyph.uv_min_x, glyph.uv_min_y),
                    vec2<f32>(glyph.uv_max_x, glyph.uv_min_y),
                    vec2<f32>(glyph.uv_max_x, glyph.uv_max_y),
                );
                local_pos = top_left + vec2<f32>(corners[v_idx].x * glyph.width, corners[v_idx].y * glyph.height);
                uv = uvs[v_idx];

                // Convert logical local_pos into physical pixels using scale_factor
                let final_pos = local_pos * global_data.scale_factor;
                let sw = global_data.screen_width * global_data.scale_factor;
                let sh = global_data.screen_height * global_data.scale_factor;
                let ndc_x = (final_pos.x / sw) * 2.0 - 1.0;
                let ndc_y = 1.0 - (final_pos.y / sh) * 2.0;

                var out: VertexOutput;
                out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
                out.uv = uv;
                out.color = vec4<f32>(glyph.r, glyph.g, glyph.b, glyph.a);
                return out;
            }

            @fragment
            fn text_fs(in: VertexOutput) -> @location(0) vec4<f32> {
                let alpha_mask = textureSample(t_atlas, s_atlas, in.uv).r;
                return vec4<f32>(in.color.rgb, in.color.a * alpha_mask);
            }
        "#;

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Charton Text WGSL Module"),
            source: wgpu::ShaderSource::Wgsl(text_wgsl.into()),
        });

        // --------------------------------------------------------------------
        // C. 组装多图层渲染管线布局 (Layout)
        // --------------------------------------------------------------------
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(main_layout), Some(&text_bind_group_layout)],
            immediate_size: 0,
        });

        // --------------------------------------------------------------------
        // D. 建立渲染状态机管线 (Pipeline)
        // --------------------------------------------------------------------
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("text_vs"),
                buffers: &[], // 现代 Instancing 架构空出传统顶点缓冲区
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("text_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm, // 根据你的实际表面调整格式
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // 开启文本透明度混合
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // --------------------------------------------------------------------
        // E. 统一分配 1024x1024 物理动态字形图集资源 (Atlas)
        // --------------------------------------------------------------------
        let atlas_width = 2048;
        let atlas_height = 2048;

        let text_atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Charton Monolithic Font Atlas Texture"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // 💡 完美的、不易报错的通用现代写法
        let text_atlas_view = text_atlas_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Text Font Atlas Texture View"),
            format: Some(wgpu::TextureFormat::R8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            usage: Default::default(), // 👈 用 Default 让编译器根据上下文自动推导正确的枚举值！
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        // 💡 完美对齐后的采样器（使用你已经修正好的 MipmapFilterMode）
        let text_atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Font Atlas Nearest Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // 完美返回外部所需的 5 个资源元组（注意：去掉了原先没有意义的 6 个解构残留）
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

    /// Injects a raw continuous path into the high-throughput GPU streaming queues.
    /// This removes the legacy CPU-bound Lyon tessellator entirely and schedules a
    /// vertex-buffer-less hardware line expansion on the GPU.
    pub fn tessellate_path(&mut self, config: PathConfig) {
        // A valid polyline segment requires at least a starting point and a destination
        if config.points.len() < 2 {
            return;
        }

        // Record lookup offsets inside the global contiguous arrays
        let start_point_idx = self.pending_path_points.len() as u32;
        let point_count = config.points.len() as u32;
        let style_idx = self.pending_path_styles.len() as u32;
        let path_idx = self.pending_path_args.len() as u32;

        // 1. Stream raw (x, y) coordinates into the global points pool
        for &(x, y) in &config.points {
            self.pending_path_points.push(GpuPathPoint { x, y });
        }

        // 2. Format and push the styling configurations with opacity multiplier
        let stroke_color = config.stroke.rgba();
        self.pending_path_styles.push(GpuPathStyle {
            r: stroke_color[0],
            g: stroke_color[1],
            b: stroke_color[2],
            a: stroke_color[3] * config.opacity,
            thickness: config.stroke_width,
            _pad0: 0.0, // Satisfy 16-byte layout alignment boundaries explicitly
            _pad1: 0.0,
            _pad2: 0.0,
        });

        // 3. Compress layout parameters into the global routing lookup map
        self.pending_path_args.push(GpuPathArgs {
            start_point_idx,
            style_idx,
            _pad0: 0, // Satisfy structural padding constraints
            _pad1: 0,
        });

        // 4. Commit a zero-alignment-overhead batch token into the deferred render queue
        self.batches.push(DrawBatch::PathSimple {
            path_idx,
            point_count,
        });
    }

    // ============================================================================
    // Render & Flush
    // ============================================================================
    /// Flushes pending render data to GPU buffers and renders to the target texture view
    pub fn flush_and_render(&mut self, view: &wgpu::TextureView) {
        // --------------------------------------------------------------------
        // PHASE 1: DATA UPLOAD
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

        if !self.pending_polygon_vertices.is_empty() || !self.pending_polygon_indices.is_empty() {
            let vertices = std::mem::take(&mut self.pending_polygon_vertices);
            let indices = std::mem::take(&mut self.pending_polygon_indices);
            self.polygon_vertex_buffer = self.create_buffer(&vertices);
            self.polygon_index_buffer = self.create_buffer(&indices);
            self.uploaded_polygon_index_count = indices.len() as u32;
        }

        let has_paths = !self.pending_path_points.is_empty();
        let path_bind_group = if has_paths {
            use wgpu::util::DeviceExt;
            let points_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Path Points Global Storage Buffer"),
                contents: bytemuck::cast_slice(&self.pending_path_points),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
            let styles_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Path Styles Global Storage Buffer"),
                contents: bytemuck::cast_slice(&self.pending_path_styles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
            let args_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Path Routing Args Global Storage Buffer"),
                contents: bytemuck::cast_slice(&self.pending_path_args),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Global Monolithic Path Bind Group 1"),
                layout: &self.path_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(points_buf.as_entire_buffer_binding()) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(styles_buf.as_entire_buffer_binding()) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(args_buf.as_entire_buffer_binding()) },
                ],
            }))
        } else {
            None
        };

        if !self.pending_gradient_rects.is_empty() {
            let grad_rects = std::mem::take(&mut self.pending_gradient_rects);
            self.gradient_rect_buffer = self.create_buffer(&grad_rects);
            self.uploaded_gradient_rect_count = grad_rects.len() as u32;
        }

        // 🌟 [文本流数据实时上树上传]
        if !self.pending_glyphs.is_empty() {
            self.queue.write_buffer(
                &self.text_instance_buffer,
                0,
                bytemuck::cast_slice(&self.pending_glyphs),
            );
        }

        // --------------------------------------------------------------------
        // PHASE 2: BIND GROUP SETUP
        // --------------------------------------------------------------------
        self.main_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group (Updated)"),
            layout: &self.main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(self.circle_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(self.rect_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(self.line_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Buffer(self.gradient_rect_buffer.as_entire_buffer_binding()) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Buffer(self.uniform_buffer.as_entire_buffer_binding()) },
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

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // --------------------------------------------------------------------
        // PHASE 3: ORCHESTRATED DRAWING
        // --------------------------------------------------------------------
        {
            let mut pass = encoder.begin_render_pass(&render_pass_desc);
            pass.set_bind_group(0, Some(&self.main_bind_group), &[]);

            for batch in &self.batches {
                match batch {
                    DrawBatch::Circle { start, count } => {
                        pass.set_pipeline(&self.circle_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Rect { start, count } => {
                        pass.set_pipeline(&self.rect_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Line { start, count } => {
                        pass.set_pipeline(&self.line_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::Polygon { index_start, index_count } => {
                        pass.set_pipeline(&self.polygon_pipeline);
                        pass.set_vertex_buffer(0, self.polygon_vertex_buffer.slice(..));
                        pass.set_index_buffer(self.polygon_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(*index_start..(*index_start + *index_count), 0, 0..1);
                    }
                    DrawBatch::GradientRect { start, count } => {
                        pass.set_pipeline(&self.gradient_rect_pipeline);
                        pass.draw(0..4, *start..(*start + *count));
                    }
                    DrawBatch::PathSimple { path_idx, point_count } => {
                        if let Some(global_path_bg) = &path_bind_group {
                            pass.set_pipeline(&self.path_simple_pipeline);
                            pass.set_bind_group(1, Some(global_path_bg), &[]);
                            let virtual_vertex_count = (*point_count - 1) * 6;
                            pass.draw(0..virtual_vertex_count, *path_idx..(*path_idx + 1));
                        }
                    }
                    DrawBatch::Text { start, count } => {
                        let glyph_count = *count as u32;
                        if glyph_count > 0 {
                            let text_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("Dynamic Local Text Batch Bind Group (@group(1))"),
                                layout: &self.text_bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::Buffer(self.text_instance_buffer.as_entire_buffer_binding()),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::TextureView(&self.text_atlas_view),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 2,
                                        resource: wgpu::BindingResource::Sampler(&self.text_atlas_sampler),
                                    },
                                ],
                            });

                            pass.set_pipeline(&self.text_pipeline);
                            pass.set_bind_group(1, Some(&text_bind_group), &[]);
                            pass.draw(0..6, *start..(*start + glyph_count));
                        }
                    }
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
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

    fn draw_text(&mut self, config: TextConfig) {
        // 1. Rasterize at device pixels (HiDPI aware) then expose logical coordinates to shader
        let font_size = config.font_size;
        let scale_px = ab_glyph::PxScale::from(font_size * self.scale_factor);
        let scaled_font_px = self.font.as_scaled(scale_px);

        // Compute total width in logical units (divide physical advances by scale_factor)
        let mut total_width = 0.0f32;
        let mut last_glyph_id = None;
        const TRACKING: f32 = 0.3;

        let mut text_top = f32::INFINITY;
        let mut text_bottom = f32::NEG_INFINITY;
        let mut text_glyphs: Vec<(ab_glyph::GlyphId, CachedGlyphInfo)> = Vec::new();

        for ch in config.text.chars() {
            if ch.is_control() { continue; }
            let gid = self.font.glyph_id(ch);

            if let Some(prev) = last_glyph_id {
                total_width += scaled_font_px.kern(prev, gid) / self.scale_factor;
            }

            let cached_glyph = if let Some(info) = self.font_cache.get(&(ch, (font_size * self.scale_factor) as u32)) {
                info.clone()
            } else {
                let gid = self.font.glyph_id(ch);
                let glyph = gid.with_scale(scale_px);
                let cached = if let Some(outlined) = self.font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    let mut width = bounds.width().ceil() as u32;
                    let mut height = bounds.height().ceil() as u32;
                    if width == 0 { width = 1; }
                    if height == 0 { height = 1; }

                    const ATLAS_WIDTH: u32 = 2048;
                    const ATLAS_HEIGHT: u32 = 2048;

                    if self.atlas_current_x + width + 4 > ATLAS_WIDTH {
                        self.atlas_current_y += self.atlas_max_row_height + 4;
                        self.atlas_current_x = 4;
                        self.atlas_max_row_height = 0;
                    }
                    if self.atlas_current_y + height + 4 > ATLAS_HEIGHT {
                        eprintln!("[WARN] GPU Font Atlas Cache full! Skipping glyph rasterization.");
                        CachedGlyphInfo { uv_min: [0.0, 0.0], uv_max: [0.0, 0.0], width: 0.0, height: 0.0, o_x: 0.0, o_y: 0.0 }
                    } else {
                        let mut alpha_pixels = vec![0u8; (width * height) as usize];
                        outlined.draw(|x, y, alpha| {
                            let idx = (y * width + x) as usize;
                            if idx < alpha_pixels.len() {
                                let alpha = alpha.clamp(0.0, 1.0);
                                alpha_pixels[idx] = (alpha * 255.0).round() as u8;
                            }
                        });

                        let bytes_per_row = ((width + 255) / 256) * 256;
                        let mut padded_pixels = vec![0u8; (bytes_per_row * height) as usize];
                        for row in 0..height as usize {
                            let src_start = row * width as usize;
                            let dst_start = row * bytes_per_row as usize;
                            padded_pixels[dst_start..dst_start + width as usize].copy_from_slice(
                                &alpha_pixels[src_start..src_start + width as usize],
                            );
                        }

                        self.queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &self.text_atlas_texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d { x: self.atlas_current_x, y: self.atlas_current_y, z: 0 },
                                aspect: wgpu::TextureAspect::All,
                            },
                            &padded_pixels,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(bytes_per_row),
                                rows_per_image: Some(height),
                            },
                            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                        );

                        let atlas_w = 2048.0f32;
                        let atlas_h = 2048.0f32;
                        let uv_min = [
                            (self.atlas_current_x as f32 + 0.5) / atlas_w,
                            (self.atlas_current_y as f32 + 0.5) / atlas_h,
                        ];
                        let uv_max = [
                            (self.atlas_current_x as f32 + (width as f32) - 0.5) / atlas_w,
                            (self.atlas_current_y as f32 + (height as f32) - 0.5) / atlas_h,
                        ];

                        let info = CachedGlyphInfo {
                            uv_min,
                            uv_max,
                            width: width as f32,
                            height: height as f32,
                            o_x: bounds.min.x,
                            o_y: bounds.min.y,
                        };

                        self.font_cache.insert((ch, (font_size * self.scale_factor) as u32), info.clone());
                        self.atlas_current_x += width + 4;
                        if height > self.atlas_max_row_height {
                            self.atlas_max_row_height = height;
                        }
                        info
                    }
                } else {
                    CachedGlyphInfo { uv_min: [0.0, 0.0], uv_max: [0.0, 0.0], width: 0.0, height: 0.0, o_x: 0.0, o_y: 0.0 }
                };
                cached
            };

            let top = cached_glyph.o_y;
            let bottom = cached_glyph.o_y + cached_glyph.height;
            text_top = text_top.min(top);
            text_bottom = text_bottom.max(bottom);
            text_glyphs.push((gid, cached_glyph.clone()));

            total_width += scaled_font_px.h_advance(gid) / self.scale_factor + TRACKING;
            last_glyph_id = Some(gid);
        }

        if total_width > 0.0 {
            total_width -= TRACKING;
        }

        let mut dx = 0.0f32;
        match config.text_anchor.as_str() {
            "middle" => dx -= total_width / 2.0,
            "end" => dx -= total_width,
            _ => {}
        }

        let mut dy = 0.0f32;
        let ascent = scaled_font_px.ascent() / self.scale_factor;
        let descent = scaled_font_px.descent() / self.scale_factor;
        match config.dominant_baseline.as_str() {
            "hanging" => dy += ascent,
            "central" | "middle" => {
                dy += (ascent + descent) / 2.0;
            }
            _ => {}
        }

        let mut current_x = config.x + dx;
        let current_y = config.y + dy;

        let mut glyphs_in_this_call = 0usize;
        let mut last_glyph_id = None;

        for (gid, cached_glyph) in text_glyphs {
            let render_x = current_x;
            let render_y = current_y;

            if self.pending_glyphs.len() < 4096 {
                let color_arr = config.color.rgba();
                self.pending_glyphs.push(GpuGlyph {
                    x: render_x,
                    y: render_y,
                    width: cached_glyph.width / self.scale_factor,
                    height: cached_glyph.height / self.scale_factor,
                    uv_min_x: cached_glyph.uv_min[0],
                    uv_min_y: cached_glyph.uv_min[1],
                    uv_max_x: cached_glyph.uv_max[0],
                    uv_max_y: cached_glyph.uv_max[1],
                    r: color_arr[0] as f32,
                    g: color_arr[1] as f32,
                    b: color_arr[2] as f32,
                    a: color_arr[3] as f32,
                    o_x: cached_glyph.o_x / self.scale_factor,
                    o_y: -cached_glyph.o_y / self.scale_factor,
                    _pad0: 0.0,
                    _pad1: 0.0,
                });

                self.current_glyph_count += 1;
                glyphs_in_this_call += 1;
            }

            if let Some(prev_gid) = last_glyph_id {
                current_x += scaled_font_px.kern(prev_gid, gid) / self.scale_factor;
            }
            current_x += scaled_font_px.h_advance(gid) / self.scale_factor + TRACKING;
            last_glyph_id = Some(gid);
        }

        if glyphs_in_this_call > 0 {
            self.push_batch(BatchType::Text, glyphs_in_this_call as u32);
        }
    }
}
```
