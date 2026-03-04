#import "shaders/shared.wgsl"::{
    view, frame
}

@group(1) @binding(0)
var depth_tex: texture_2d<f32>;

@group(1) @binding(1)
var albedo_tex: texture_2d<f32>;

@group(1) @binding(2)
var normal_tex: texture_2d<f32>;

@group(1) @binding(3)
var material_tex: texture_2d<f32>;

@group(1) @binding(4)
var hdr_out_tex: texture_storage_2d<rgba16float, write>;


@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let coord = vec2<i32>(id.xy);
    let albedo_color = textureLoad(albedo_tex, coord, 0);
    textureStore(hdr_out_tex, coord, albedo_color);
}