#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var<uniform> mesh: Mesh;

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_scale: vec4<f32>,
    @location(4) i_color: u32,
    @location(5) i_flags: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let position = vertex.position + vertex.i_pos_scale.xyz;
    //  * vertex.i_pos_scale.w; - scale used to store rain destination
    //TODO: don't need to copy it to shader.
    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(mesh.model, vec4<f32>(position, 1.0));
    // out.color = vertex.i_color;
    out.color = vec4<f32>((vec4<u32>(vertex.i_color) >> vec4<u32>(0u, 8u, 16u, 24u)) &
            vec4<u32>(255u)) / 255.0;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
