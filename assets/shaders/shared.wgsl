#import bevy_render::{
    view::View,
}
struct FrameUniform {
    resolution: vec2<f32>,
    frame_index: u32,
    sun_direction: vec3<f32>,
};
const MAX_STEPS: u32 = 1024u;
const EPSILON: f32 = 1e-5;
const MAX_DIST: f32 = 2000.0;
const PI: f32 = 3.141592653589793;

@group(0) @binding(0)
var<uniform> view: View;

@group(0) @binding(1)
var<uniform> frame: FrameUniform;