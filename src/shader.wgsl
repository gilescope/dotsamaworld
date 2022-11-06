// Vertex shader

struct InstanceInput {
    @location(5) instance_position: vec3<f32>,
    @location(6) instance_color: u32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = vec4<f32>(model.color, 1.0) + (
        vec4<f32>((vec4<u32>(instance.instance_color) >> vec4<u32>(0u, 8u, 16u, 24u)) &
            vec4<u32>(255u)) / 255.0);
//     out.clip_position = vec4<f32>(model.position, 1.0);
     out.clip_position = camera.view_proj * vec4<f32>(instance.instance_position + model.position, 1.0);
   // let x = f32(1 - i32(in_vertex_index)) * 0.5;
    // let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    // out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    // out.vert_pos = out.clip_position.xyz;
    return out;
}
 

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
     return in.color;
}
 