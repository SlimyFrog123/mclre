// Chunk vertex and fragment shaders

struct ViewUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> view: ViewUniform;

struct VertexInput {
    @location(0) position: u32,
    @location(1) normal_ao: u32,
    @location(2) tex_coord: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) ao: f32,
    @location(2) tex_coord: vec2<f32>,
    @location(3) layer: u32,
}

// Unpacks the packed position (10 bits per component)
fn unpack_position(packed: u32) -> vec3<u32> {
    let x = packed & 0x3FFu;
    let y = (packed >> 10) & 0x3FFu;
    let z = (packed >> 20) & 0x3FFu;
    return vec3<u32>(x, y, z);
}

// Unpacks the normal + AO
fn unpack_normal_ao(packed: u32) -> vec2<f32> {
    let face = packed & 0x3u;
    let ao = (packed >> 2) & 0x3Fu;
    let face_normal = select_face_normal(face);
    return vec2<f32>(f32(face), f32(ao) / 63.0);
}

// Unpacks the texture coordinates
fn unpack_tex_coord(packed: u32) -> vec3<u32> {
    let u = packed & 0xFFu;
    let v = (packed >> 8) & 0xFFu;
    let layer = (packed >> 16) & 0xFFFFu;
    return vec3<u32>(u, v, layer);
}

fn select_face_normal(face: u32) -> vec3<f32> {
    // Face normals: +X, -X, +Y, -Y, +Z, -Z
    switch (face) {
        case 0u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, 1.0, 0.0); }
        case 3u: { return vec3<f32>(0.0, -1.0, 0.0); }
        case 4u: { return vec3<f32>(0.0, 0.0, 1.0); }
        case 5u: { return vec3<f32>(0.0, 0.0, -1.0); }
        default: { return vec3<f32>(0.0, 1.0, 0.0); }
    }
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Unpack vertex data
    let pos_u32 = unpack_position(input.position);
    let pos = vec3<f32>(f32(pos_u32.x), f32(pos_u32.y), f32(pos_u32.z));

    let normal_ao = unpack_normal_ao(input.normal_ao);
    let face = normal_ao.x;
    let ao = normal_ao.y;

    let tex = unpack_tex_coord(input.tex_coord);

    // World position = section origin + local position
    // For now, assume section origin is (0,0,0) - will be handled by instancing or push constants
    let world_pos = pos;

    // Transform to clip space
    output.clip_position = view.view_proj * vec4<f32>(world_pos, 1.0);

    // Output normal, AO, and texture coordinates
    output.normal = select_face_normal(face);
    output.ao = ao;
    output.tex_coord = vec2<f32>(f32(tex.x) / 255.0, f32(tex.y) / 255.0);
    output.layer = tex.z;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lit shading with AO
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diffuse = max(dot(input.normal, light_dir), 0.0);

    // Ambient + diffuse with AO
    let ambient = 0.3;
    let color = vec3<f32>(0.8, 0.7, 0.6); // Placeholder block color

    let lit = color * (ambient + (1.0 - ambient) * diffuse) * input.ao;

    return vec4<f32>(lit, 1.0);
}