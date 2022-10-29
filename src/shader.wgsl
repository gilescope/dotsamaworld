// Vertex shader

struct InstanceInput {
    @location(5) instance_position: vec3<f32>,
    // @location(6) model_matrix_1: vec4<f32>,
    // @location(7) model_matrix_2: vec4<f32>,
    // @location(8) model_matrix_3: vec4<f32>,
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
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // let model_matrix = mat4x4<f32>(
    //     instance.model_matrix_0,
    //     // instance.model_matrix_1,
    //     // instance.model_matrix_2,
    //     // instance.model_matrix_3,
    // );

    var out: VertexOutput;
     out.color = model.color;
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
     return vec4<f32>(in.color, 1.0);
}
 