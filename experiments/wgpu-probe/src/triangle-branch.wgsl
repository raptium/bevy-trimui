@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var position: vec2<f32>;
    switch index {
        case 0u: { position = vec2<f32>(-0.7, -0.7); }
        case 1u: { position = vec2<f32>(0.7, -0.7); }
        default: { position = vec2<f32>(0.0, 0.7); }
    }
    return vec4<f32>(position, 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.25, 0.0, 1.0);
}
