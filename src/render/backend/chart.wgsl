// ============================================================================
// (WGPU + SDF + Instancing) Scatter Plot Shader
// ============================================================================

// 1. Data Structure Definitions
// This structure must perfectly match the memory layout of the Rust struct.
struct PointData {
    x: f32,          // Center X position in screen pixels
    y: f32,          // Center Y position in screen pixels
    r: f32,          // Color Red channel   (0.0 to 1.0)
    g: f32,          // Color Green channel (0.0 to 1.0)
    b: f32,          // Color Blue channel  (0.0 to 1.0)
    a: f32,          // Color Alpha channel (0.0 to 1.0)
    radius: f32,     // Radius of the point in pixels
    shape_type: f32, // 0.0 for Circle, 1.0 for Rounded Rectangle
};

// 2. Resource Bindings (GPU Memory Slots)
// Group 0, Binding 0: High-performance Storage Buffer containing all scatter points.
@group(0) @binding(0)
var<storage, read> points: array<PointData>;

// Group 0, Binding 1: Global Uniform Buffer providing current canvas dimensions.
struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,  // HiDPI scaling factor
}
@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

// 3. Pipeline Interstage Bridge (Vertex to Fragment)
struct VertexOutput {
    // Required built-in output for the GPU rasterizer (clipspace coordinates)
    @builtin(position) clip_position: vec4<f32>,
    
    // Flat interpolation ensures the instance ID is not blended across the quad
    @location(0) @interpolate(flat) instance_idx: u32,
    
    // Pass screen-space position (in physical pixels) for SDF calculation
    @location(1) screen_pos: vec2<f32>,
};

// ============================================================================
// Signed Distance Field (SDF) Mathematical Functions
// ============================================================================

// Evaluates the distance to a perfect circle.
// Returns negative inside the circle, zero on the edge, positive outside.
fn sd_circle(p: vec2<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

// Evaluates the distance to a rounded box.
// 'size' represents the half-extents (half width, half height) of the box.
fn sd_rounded_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

// ============================================================================
// Vertex Shader Stage: Generates a quad (billboard) per point instance
// ============================================================================
@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32
) -> VertexOutput {
    // Fetch the data for the current scatter point instance
    let p = points[i_idx];
    
    // Generate local quad coordinates [-1.0, 1.0] dynamically based on vertex index
    var pos = vec2<f32>(0.0, 0.0);
    if (v_idx == 0u) { pos = vec2<f32>(-1.0, -1.0); } // Bottom-Left
    if (v_idx == 1u) { pos = vec2<f32>( 1.0, -1.0); } // Bottom-Right
    if (v_idx == 2u) { pos = vec2<f32>(-1.0,  1.0); } // Top-Left
    if (v_idx == 3u) { pos = vec2<f32>( 1.0,  1.0); } // Top-Right

    // Fix: Expand the bounding box for rounded rectangles (sqrt(2) ~= 1.415)
    // to prevent the sharp corners of the rectangle from being clipped by the quad boundaries.
    var box_scale = 1.0;
    if (p.shape_type > 0.5) {
        box_scale = 1.415; 
    }

    // Scale the quad local position by radius and translate it to screen (x, y)
    // Apply scale_factor transformation (similar to RasterBackend's Transform::from_scale)
    let scaled_pos = vec2<f32>(p.x, p.y) * uniforms.scale_factor;
    let final_pos = scaled_pos + pos * (p.radius * box_scale * uniforms.scale_factor);
    
    // Transform screen pixel coordinates to WebGPU Normalized Device Coordinates [-1.0, 1.0]
    // Note: Reverses the Y-axis because screen space is top-left, WebGPU is bottom-left.
    // Use scaled dimensions for proper normalization (logical_size * scale_factor = physical_size)
    let scaled_width = uniforms.screen_width * uniforms.scale_factor;
    let scaled_height = uniforms.screen_height * uniforms.scale_factor;
    let x = (final_pos.x / scaled_width) * 2.0 - 1.0;
    let y = (1.0 - final_pos.y / scaled_height) * 2.0 - 1.0;
    
    // Pack data into the bridge structure and pass to the fragment stage
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.instance_idx = i_idx;
    out.screen_pos = final_pos;  // Pass physical pixel coordinates
    return out;
}

// ============================================================================
// Fragment Shader Stage: Math-based geometric clipping and coloring
// ============================================================================
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Retrieve the point's properties using the safely passed instance ID
    let p = points[in.instance_idx];
    
    // Calculate the pixel's relative offset vector from the center of the point
    // Use screen-space coordinates (physical pixels) for correct SDF calculation
    let local_pos = in.screen_pos - vec2<f32>(p.x * uniforms.scale_factor, p.y * uniforms.scale_factor);
    
    var dist: f32 = 0.0;
    
    // Select the appropriate SDF specification manual
    if (p.shape_type < 0.5) {
        dist = sd_circle(local_pos, p.radius);
    } else {
        // Assume a square aspect ratio; corner radius set to 20% of the half-size
        dist = sd_rounded_box(local_pos, vec2<f32>(p.radius), p.radius * 0.2);
    }
    
    // Hardware Anti-Aliasing:
    // fwidth(dist) dynamically calculates the change rate between adjacent pixels,
    // ensuring a crisp 1-pixel anti-aliasing filter regardless of display DPI / Retina screens.
    let edge = fwidth(dist);
    let alpha = 1.0 - smoothstep(-edge, edge, dist);
    
    // Geometric Clipping: 
    // Discard the fragment entirely if it falls outside the shape boundary
    if (alpha <= 0.0) {
        discard;
    }
    
    // Output final color mixed with the procedural anti-aliasing alpha mask
    return vec4<f32>(p.r, p.g, p.b, p.a * alpha);
}