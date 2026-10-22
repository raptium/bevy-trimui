// Element-size probe: vec3 elements instead of vec2.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec3<f32>, 3>(
        vec3<f32>(-0.7, -0.7, 0.0), vec3<f32>(0.7, -0.7, 0.0), vec3<f32>(0.0, 0.7, 0.0),
    );
    return vec4<f32>(p[index].xy, 0.0, 1.0);
}
@fragment
fn fragment() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.25, 0.0, 1.0); }
