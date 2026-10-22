// Breadth: dynamic index into a uniform buffer array.
struct Data {
    positions: array<vec4<f32>, 4>,
};
@group(0) @binding(0) var<uniform> data: Data;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(data.positions[index].xy, 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.25, 0.0, 1.0);
}
