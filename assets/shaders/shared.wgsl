
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
const PI: f32 = 3.141592653589793;

struct RayEnd {
    pos: vec3<f32>,
    normal: vec3<f32>,
    material: u32,
    depth: f32     
};

fn dither(frag_coord: vec2<f32>) -> vec3<f32> {
	// Iestyn's RGB dither (7 asm instructions) from Portal 2 X360, slightly modified for VR.
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), frag_coord));
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
	// Subtract 0.5 to avoid slightly brightening the whole viewport.
    return (dither.rgb - 0.5) / 255.0;
}

fn intersect_aabb(origin: vec3<f32>, dir: vec3<f32>, min: vec3<f32>, max: vec3<f32>) -> vec2<f32> {
    let inv_dir = 1.0 / dir;
    let t0 = (min - origin) * inv_dir;
    let t1 = (max - origin) * inv_dir;
    let tmin = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
    let tmax = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
    return vec2<f32>(tmin, tmax);
}
fn get_camera_rotation_matrix() -> mat3x3<f32> {
    return mat3x3<f32>(
        env_data.cam_rotx,
        env_data.cam_roty,
        env_data.cam_rotz
    );
}

fn screen_uv_to_ray_dir(uv: vec2<f32>, fov_y_deg: f32, aspect: f32) -> vec3<f32> {
    let fov_y = radians(fov_y_deg);
    let y = tan(fov_y * 0.5);
    let x = y * aspect;

    // flip Y for NDC
    let screen = vec2<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0);
    let dir_camera = normalize(vec3<f32>(screen.x * x, screen.y * y, -1.0));
    
    // Adjust the ray direction based on the camera's rotation
    return normalize(get_camera_rotation_matrix() * dir_camera);
}

@group(1) @binding(0) var depth_tex: texture_depth_2d;
@group(1) @binding(1) var motion_tex: texture_2d<f32>;
@group(1) @binding(2) var<uniform> env_data: EnviromentData;
@group(1) @binding(4) var<uniform> push: PushConstants;

