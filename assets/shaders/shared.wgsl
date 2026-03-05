#import bevy_render::{
    view::View,
}
struct FrameUniform {
    resolution: vec2<f32>,
    sun_direction: vec3<f32>,
};
#import bevy_render::{
    view::{uv_to_ndc,direction_view_to_world,position_ndc_to_world},
}

const MAX_STEPS: u32 = 1024u;
const EPSILON: f32 = 1e-5;
const MAX_DIST: f32 = 2000.0;
const PI: f32 = 3.141592653589793;

@group(0) @binding(0)
var<uniform> view: View;

@group(0) @binding(1)
var<uniform> frame: FrameUniform;

fn reconstruct_world_ray(coord: vec2<i32>) -> vec3<f32> {
    let uv = (vec2<f32>(coord) + 0.5) / frame.resolution;
    let ndc_xy = uv_to_ndc(uv);

    // View space direction (z = -1 because -Z is forward)
    let view_dir = normalize(vec3<f32>(ndc_xy, -1.0));

    return normalize(direction_view_to_world(view_dir, view.world_from_view));
}

fn reconstruct_world_position(depth: f32, coord: vec2<i32>) -> vec3<f32> {
    // 1. Convert pixel to UV
    let uv = (vec2<f32>(coord) + 0.5) / frame.resolution;

    // 2. Convert UV to NDC space [-1, 1]
    let ndc_xy = uv_to_ndc(uv);

    // 3. Build full NDC position
    // Depth in NDC: far plane = 0.0, near plane = 1.0
    let ndc = vec3<f32>(ndc_xy, depth);

    // 4. Convert NDC to world position using Bevy helper
    return position_ndc_to_world(ndc, view.world_from_clip);
}