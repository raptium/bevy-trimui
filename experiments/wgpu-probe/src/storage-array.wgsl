// Breadth: dynamic index into a runtime-sized storage buffer array
// (the `mesh[instance_index]` shape Bevy's Mesh2d vertex shader uses).
struct Data {
    positions: array<vec2<f32>>,
};
@group(0) @binding(0) var<storage, read> data: Data;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(data.positions[index], 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.25, 0.0, 1.0);
}
