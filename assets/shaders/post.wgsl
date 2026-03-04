#import "shaders/shared.wgsl"::{
    view, frame
}

@group(1) @binding(0)
var hdr_tex: texture_2d<f32>;

@group(1) @binding(1)
var ldr_out_tex: texture_storage_2d<rgba8unorm, write>;


@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let coord = vec2<i32>(id.xy);
    let hdr_color = textureLoad(hdr_tex, coord, 0);
    textureStore(ldr_out_tex, coord, hdr_color);
}