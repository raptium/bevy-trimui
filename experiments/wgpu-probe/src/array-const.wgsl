// Isolate: the same `let` array, but only constant indices read it.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-0.7, -0.7), vec2<f32>(0.7, -0.7), vec2<f32>(0.0, 0.7),
    );
    var p = positions[0];
    if index == 1u {
        p = positions[1];
    }
    if index == 2u {
        p = positions[2];
    }
    return vec4<f32>(p, 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.25, 0.0, 1.0);
}
