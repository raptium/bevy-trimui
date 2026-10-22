// Element-size probe: scalar elements.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let xs = array<f32, 3>(-0.7, 0.7, 0.0);
    let ys = array<f32, 3>(-0.7, -0.7, 0.7);
    return vec4<f32>(xs[index], ys[index], 0.0, 1.0);
}
@fragment
fn fragment() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.25, 0.0, 1.0); }
