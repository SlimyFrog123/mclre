// Chunk vertex and fragment shaders

struct ViewUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> view: ViewUniform;

@group(1) @binding(0)
var atlas_texture: texture_2d<f32>;

@group(1) @binding(1)
var atlas_sampler: sampler;

@group(2) @binding(0)
var<uniform> atlas_uvs: AtlasUVs;

struct AtlasUVs {
    regions: array<vec4<f32>, 512>, // u_min, v_min, u_max, v_max
}

// Uniform for chunk section origin (3 i32 = 12 bytes)
struct ChunkOrigin {
    origin: vec3<i32>,
}

@group(3) @binding(0)
var<uniform> chunk_origin: ChunkOrigin;

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
    let face = u32(normal_ao.x);
    let ao = normal_ao.y;

    let tex = unpack_tex_coord(input.tex_coord);

    // Get chunk section origin from uniform
    // World position = section origin * 16 + local position
    let world_pos = pos + vec3<f32>(f32(chunk_origin.origin.x) * 16.0, f32(chunk_origin.origin.y) * 16.0, f32(chunk_origin.origin.z) * 16.0);

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
    // Get the atlas UV region for this texture layer
    let layer = input.layer;
    let uv_region = atlas_uvs.regions[layer];
    let u_min = uv_region.x;
    let v_min = uv_region.y;
    let u_max = uv_region.z;
    let v_max = uv_region.w;
    
    // Map face-local UV (0-1) to atlas UV region
    let atlas_u = u_min + input.tex_coord.x * (u_max - u_min);
    let atlas_v = v_min + input.tex_coord.y * (v_max - v_min);
    let atlas_uv = vec2<f32>(atlas_u, atlas_v);
    
    // Sample texture from atlas
    let tex_color = textureSample(atlas_texture, atlas_sampler, atlas_uv);
    
    // Simple lit shading with AO
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diffuse = max(dot(input.normal, light_dir), 0.0);

    // Ambient + diffuse with AO
    let ambient = 0.3;
    let color = tex_color.rgb; // Use texture color

    let lit = color * (ambient + (1.0 - ambient) * diffuse) * input.ao;

    return vec4<f32>(lit, tex_color.a);
}