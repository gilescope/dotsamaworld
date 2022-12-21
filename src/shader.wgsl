// Vertex shader



struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;
@group(0) @binding(3)
var t_diffuse_emoji: texture_2d<f32>;
@group(0) @binding(4)
var s_diffuse_emoji: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex: vec2<f32>,
};
struct InstanceInput {
    @location(3) instance_position: vec3<f32>,
    @location(4) instance_color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex: vec2<f32>,
    @location(2) tex_index: u32,
}

// TODO: add in global time, have rain happen in vertex shader.
// That way you only need to copy the buffer when new blocks appear.

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(instance.instance_position + model.position, 1.0);
    // Is texture?
    if model.color[2] == -2.0 {
        // TODO: inject height / width.
        let coords = (
            vec4<f32>((vec4<u32>(instance.instance_color) >> vec4<u32>(0u, 8u, 16u, 24u)) &
            vec4<u32>(255u)) );

        let offset_y = (1.0 / 40.0) * f32(coords[0]);
        let offset_x = (1.0 / 2.0) * f32(coords[1]);

        let unexplained_magic = 1.666; // TODO: why do we need to do this for it to look right?
        out.color = vec3<f32>(offset_x + (model.color[0] / 2.0), offset_y + (unexplained_magic * model.color[1]), 0.0);
    } else {
        // Alpha channel is at least 1.
        let v = (vec4<u32>(instance.instance_color) >> vec4<u32>(0u, 8u, 16u, 24u)) &
            vec4<u32>(255u);
        let inst_color = (
            vec4<f32>(v) / 255.0);

        out.color = model.color + vec3<f32>(inst_color[0], inst_color[1], inst_color[2]);

        if inst_color[3] < 1.0 {
            out.tex = model.tex;
            out.tex_index = u32(v[3]);
        }
    }
    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Fog constants
    let density = -(0.0004 * 0.001);
    let LOG2 = 1.442695;
    let log_density = density * LOG2;
    // let fog_color = vec4<f32>(230./255. * 1.0,0.0,122./255. * 1.0,1.0);
    let fog_color = vec4<f32>(1.0,1.0,1.0,1.0);

    // Calc fog factor    
    let z = in.clip_position[2] / in.clip_position[3];
    let fog_factor = exp2( log_density * z * z );
    let fog_factor = clamp(fog_factor, 0.0, 1.0);
    
    //TODO: we are sampling every pixel, even ones we don't need to.
    let z = textureSample(t_diffuse, s_diffuse, vec2<f32>(in.color[0], in.color[1]));//1. - 

//    let offset: f32;//u32(in.color[3]* 256.0);
    // if in.tex[0] != 0 && in.tex[1] != 0 {
    //     let offset = 0.0;
    // } else {
    //     let offset = 0.0;
    // }
    let offset: u32 = in.tex_index;
    let height: u32 = 9u;
    let heightf = f32(height);
    let off_col = offset % height;
    let off_row = offset / height;
    let offset_y = (1.0 / f32(height)) * f32(off_col);
    let offset_x = (1.0 / f32(height)) * f32(off_row);

    let y = vec4<f32>(in.color, 1.0) + (8.0 * textureSample(t_diffuse_emoji, s_diffuse_emoji, vec2<f32>(offset_x + (in.tex[0] / heightf), offset_y + (in.tex[1] / heightf))));//1. - 
    let selected = vec4<f32>(0., 0., 1., 0.4);
    //let y = vec4<f32>(in.color, 1.0);

    //is texture
    if in.color[2] == 0.0 {
        return mix(fog_color, z, fog_factor);
    } else if offset == 76u {//cold face emoji.
        return selected;
    } else {
        return mix(fog_color, y, fog_factor);
    }
}
