// Blit the offscreen frame onto the display surface.
//
// A fullscreen triangle with uv (0,0) at the top-left of the output, so the
// source image row 0 lands on the top row of the panel. Format choice keeps
// this a pure copy: the source is Rgba8UnormSrgb and the display is
// Bgra8UnormSrgb, so sampling decodes and the target re-encodes.

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct BlitOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> BlitOutput {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var output: BlitOutput;
    output.uv = uv;
    output.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return output;
}

@fragment
fn fragment(input: BlitOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_sampler, input.uv);
}
