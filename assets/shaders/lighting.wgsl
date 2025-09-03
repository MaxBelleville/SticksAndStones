struct EnviromentData {
    cam_rotx: vec3<f32>,
    sunx: f32,
    cam_roty: vec3<f32>,
    suny: f32,
    cam_rotz: vec3<f32>,
    sunz: f32,
    cam_pos: vec3<f32>,
    time: f32,
};

struct PushConstants {
    screen_size: vec2<f32>,
    fov_y_deg: f32,
    root_size: f32,
    root_min: vec3<f32>,
    mode: f32,
};
const MAX_STEPS: u32 = 1024u;
const EPSILON: f32 = 1e-5;
const MAX_DIST: f32 = 2000.0;

@group(0) @binding(0) var albedo: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var normal: texture_storage_2d<rgba16float, read>;
@group(0) @binding(2) var raster_depth: texture_storage_2d<r32float, read>;
@group(0) @binding(3) var final_out: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(0) var depth_tex: texture_depth_2d;
@group(1) @binding(1) var motion_tex: texture_2d<f32>;
@group(1) @binding(2) var<uniform> env_data: EnviromentData;
@group(1) @binding(3) var<storage, read> svt_data: array<i32>;
@group(1) @binding(4) var<uniform> push: PushConstants;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u32(push.screen_size.x) || id.y >= u32(push.screen_size.y)) {
        return;
    }

    let coord = vec2<i32>(id.xy);

    // Load GBuffer data
    let albedo_val = textureLoad(albedo, coord);
    let normal_val = textureLoad(normal, coord);
    let depth_val  = textureLoad(raster_depth, coord).r;

    // Tint albedo by depth (closer = full color, farther = darker)
    let shaded = albedo_val * (1.0 - depth_val);

    // Write to output (force alpha=1)
    textureStore(final_out, coord, vec4<f32>(shaded));
}