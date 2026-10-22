// Initializer-style probe: `var` with an array initializer.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-0.7, -0.7), vec2<f32>(0.7, -0.7), vec2<f32>(0.0, 0.7),
    );
    return vec4<f32>(p[index], 0.0, 1.0);
}
@fragment
fn fragment() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.25, 0.0, 1.0); }
