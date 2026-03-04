
#import "shaders/shared.wgsl"::{
    view, frame
}

@group(1) @binding(0)
var depth_tex: texture_depth_2d;

@group(1) @binding(1)
var albedo_tex: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(2)
var normal_tex: texture_storage_2d<rgba16float, write>;

@group(1) @binding(3)
var material_tex: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(4)
var combined_depth_tex: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {

    let coord = vec2<i32>(id.xy);
    let raster_depth = textureLoad(depth_tex, coord, 0);

    let fake_depth = 0.000;
    if (fake_depth > raster_depth) {
         textureStore(albedo_tex, coord, vec4<f32>(1.0, 0.0, 1.0, 1.0));
         textureStore(normal_tex, coord, vec4<f32>(0.0, 0.0, 1.0, 0.0));
         textureStore(material_tex, coord, vec4<f32>(1.0, 0.0, 0.0, 0.0));
         textureStore(combined_depth_tex, coord, vec4<f32>(fake_depth, 0.0, 0.0, 0.0));
    }
    else {

    textureStore(combined_depth_tex, coord, vec4<f32>(raster_depth, 0.0, 0.0, 0.0));
    }
}