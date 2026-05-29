// ============================================================================
// (WGPU + Full Advanced SDF + One-Shot Instancing) Unified Master Shader
// ============================================================================

struct PointData {
    x: f32,          // Center X pixel coordinate positions
    y: f32,          // Center Y pixel coordinate positions
    r: f32,          // Normalized red color channel profile scale
    g: f32,          // Normalized green color channel profile scale
    b: f32,          // Normalized blue color channel profile scale
    a: f32,          // Blended transparency opacity scalar factor
    radius: f32,     // Calculated bounding half-extent size radius
    shape_type: f32, // Floating matching index indicating assigned PointShape ID
};

@group(0) @binding(0)
var<storage, read> points: array<PointData>;

struct Uniforms {
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,  // High-DPI hardware pixel scaling coefficient
}
@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32,
    @location(1) screen_pos: vec2<f32>,
};

// ============================================================================
// Analytical Geometric Signed Distance Fields (SDF Pure Math Implementation)
// ============================================================================

// 0.0 - Circle Signed Distance Field Equation Matrix
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

// 1.0 - Square Signed Distance Field Equation (with subtle rounded corner profile)
fn sd_square(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

// 2.0 - Equilateral Triangle Signed Distance Field (Tip oriented upwards)
fn sd_triangle(p: vec2<f32>, r: f32) -> f32 {
    let k = sqrt(3.0);
    let p_rot = vec2<f32>(p.x, -p.y); // Flip vertical orientation vector upwards
    var p_mod = vec2<f32>(abs(p_rot.x) - r, p_rot.y + r / k);
    if (p_mod.x + k * p_mod.y > 0.0) {
        p_mod = vec2<f32>(p_mod.x - k * p_mod.y, -k * p_mod.x - p_mod.y) / 2.0;
    }
    p_mod.x -= clamp(p_mod.x, -2.0 * r, 0.0);
    return -length(p_mod) * sign(p_mod.y);
}

// 3.0 - Regular 5-Pointed Geometric Star Signed Distance Field
fn sd_star(p: vec2<f32>, r: f32) -> f32 {
    let k1 = vec2<f32>(0.80901699437, -0.58778525229); // cos(18), sin(18)
    let k2 = vec2<f32>(-0.30901699437, 0.95105651629); // cos(108), sin(108)
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

// ============================================================================
// Pipeline Lifecycle Architectures (Vertex + Fragment Core Entry Points)
// ============================================================================

@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32
) -> VertexOutput {
    let p = points[i_idx];
    
    // Generates a structural quad bounding box mesh canvas layout utilizing standard Triangle Strip indices
    var pos = vec2<f32>(0.0, 0.0);
    if (v_idx == 0u) { pos = vec2<f32>(-1.0, -1.0); } 
    if (v_idx == 1u) { pos = vec2<f32>( 1.0, -1.0); } 
    if (v_idx == 2u) { pos = vec2<f32>(-1.0,  1.0); } 
    if (v_idx == 3u) { pos = vec2<f32>( 1.0,  1.0); } 

    // Scale up structural quad dimensions for multi-sided polygons to prevent mathematical edge clipping
    var box_scale = 1.0;
    if (p.shape_type > 0.5) {
        box_scale = 1.45; 
    }

    let scaled_pos = vec2<f32>(p.x, p.y) * uniforms.scale_factor;
    let final_pos = scaled_pos + pos * (p.radius * box_scale * uniforms.scale_factor);
    
    let scaled_width = uniforms.screen_width * uniforms.scale_factor;
    let scaled_height = uniforms.screen_height * uniforms.scale_factor;
    let x = (final_pos.x / scaled_width) * 2.0 - 1.0;
    let y = (1.0 - final_pos.y / scaled_height) * 2.0 - 1.0;
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
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
    
    // High-performance Fragment branch execution matrix selector mapping the PointShape Enum fields
    if (p.shape_type < 0.5) {
        dist = sd_circle(local_pos, scaled_radius);
    } else if (p.shape_type < 1.5) {
        dist = sd_square(local_pos, vec2<f32>(scaled_radius), scaled_radius * 0.1);
    } else if (p.shape_type < 2.5) {
        dist = sd_triangle(local_pos, scaled_radius);
    } else if (p.shape_type < 3.5) {
        dist = sd_star(local_pos, scaled_radius);
    } else if (p.shape_type < 4.5) {
        dist = sd_diamond(local_pos, scaled_radius);
    } else if (p.shape_type < 5.5) {
        dist = sd_pentagon(local_pos, scaled_radius);
    } else if (p.shape_type < 6.5) {
        dist = sd_hexagon(local_pos, scaled_radius);
    } else {
        dist = sd_octagon(local_pos, scaled_radius);
    }
    
    // Single unified hardware anti-aliasing kernel block
    let edge = fwidth(dist);
    let alpha = 1.0 - smoothstep(-edge, edge, dist);
    
    // Terminate alpha execution flows instantly to maximize hardware raster fill-rate parameters
    if (alpha <= 0.0) {
        discard;
    }
    
    // Swap Red (R) and Blue (B) channels to align with the underlying BGRA texture format.
    // This prevents color inversion (e.g., standard Tab10 Blue rendering as brown/orange)
    // caused by the mismatch between RGBA shader inputs and BGRA hardware output targets.
    return vec4<f32>(p.b, p.g, p.r, p.a * alpha);
}