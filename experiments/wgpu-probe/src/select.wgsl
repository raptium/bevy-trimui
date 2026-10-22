// Workaround candidate: select chain, no arrays or memory at all.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let a = vec2<f32>(-0.7, -0.7);
    let b = vec2<f32>(0.7, -0.7);
    let c = vec2<f32>(0.0, 0.7);
    let p = select(select(a, c, index == 2u), b, index == 1u);
    return vec4<f32>(p, 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.25, 0.0, 1.0);
}
