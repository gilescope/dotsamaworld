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
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    //TODO: maybe you can have 2 more f32s to b aligned.
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
    out.clip_position = camera.view_proj * vec4<f32>(instance.instance_position + model.position, 1.0);
    // Is texture?
    if model.color[2] == -2.0 {
        // TODO: inject this number
        let offset = (1.0 / 40.0) * f32(instance.instance_color);
        out.color = vec4<f32>(model.color[0], offset + model.color[1], 0.0, 0.0);
    } else {
        out.color = vec4<f32>(model.color, 1.0) + (
            vec4<f32>((vec4<u32>(instance.instance_color) >> vec4<u32>(0u, 8u, 16u, 24u)) &
            vec4<u32>(255u)) / 255.0);
    }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //TODO: we are sampling every pixel, even ones we don't need to.
    let z = textureSample(t_diffuse, s_diffuse, vec2<f32>(in.color[0], in.color[1]));//1. - 
    let y = in.color;
    if in.color[2] == 0.0 && in.color[3] == 0.0 {
        return z;
    } else {
        return y;
    }
}
