// UI rendering shader
// Renders textured quads, nine-slice panels, and text

struct UiUniform {
    screen_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> ui_uniform: UiUniform;

// Texture atlas (block/item textures)
@group(1) @binding(0)
var ui_atlas: texture_2d<f32>;

// Font atlas (glyphon)
@group(1) @binding(2)
var font_atlas: texture_2d<f32>;

@group(1) @binding(1)
var ui_sampler: sampler;

@group(1) @binding(3)
var font_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) texture_index: u32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Convert screen coordinates to NDC (-1 to 1)
    // Screen coordinates: (0,0) = top-left, (width, height) = bottom-right
    // NDC: (-1, 1) = top-left, (1, -1) = bottom-right
    let screen_pos = vec2<f32>(
        (input.position.x / ui_uniform.screen_size.x) * 2.0 - 1.0,
        1.0 - (input.position.y / ui_uniform.screen_size.y) * 2.0
    );
    
    output.clip_position = vec4<f32>(screen_pos, 0.0, 1.0);
    output.uv = input.uv;
    output.color = vec4<f32>(
        f32((input.color >> 16) & 0xFF) / 255.0,
        f32((input.color >> 8) & 0xFF) / 255.0,
        f32((input.color >> 0) & 0xFF) / 255.0,
        f32((input.color >> 24) & 0xFF) / 255.0,
    );
    output.texture_index = 0u; // 0 = atlas, 1 = font
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Select texture based on texture_index
    // 0 = ui_atlas, 1 = font_atlas
    var color: vec4<f32>;
    if input.texture_index == 1u {
        color = textureSample(font_atlas, font_sampler, input.uv) * input.color;
    } else {
        color = textureSample(ui_atlas, ui_sampler, input.uv) * input.color;
    }
    return color;
}