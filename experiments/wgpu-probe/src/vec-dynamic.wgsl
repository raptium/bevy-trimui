// Breadth: dynamic index into a vector value.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let xs = vec3<f32>(-0.7, 0.7, 0.0);
    let ys = vec3<f32>(-0.7, -0.7, 0.7);
    return vec4<f32>(xs[index], ys[index], 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.25, 0.0, 1.0);
}
