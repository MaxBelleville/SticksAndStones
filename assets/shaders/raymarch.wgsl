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

@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(0) var depth_tex: texture_depth_2d;
@group(1) @binding(1) var<uniform> env_data: EnviromentData;
@group(1) @binding(2) var<storage, read> svt_data: array<i32>;
@group(1) @binding(3) var<uniform> push: PushConstants;

fn sdf_sphere(p: vec3<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

fn sdf_scene(p: vec3<f32>) -> f32 {
    return min(
        sdf_sphere(p, 0.5),
        sdf_sphere(p + vec3(0.0, 0.5, 0.0), 0.5)
    );
}

fn estimate_normal(p: vec3<f32>) -> vec3<f32> {
    let eps = 0.001;
    let dx = vec3<f32>(eps, 0.0, 0.0);
    let dy = vec3<f32>(0.0, eps, 0.0);
    let dz = vec3<f32>(0.0, 0.0, eps);
    let d = sdf_scene(p); // Use a wrapper SDF that handles all geometry

    return normalize(vec3<f32>(
        sdf_scene(p + dx) - d,
        sdf_scene(p + dy) - d,
        sdf_scene(p + dz) - d
    ));
}

fn get_camera_rotation_matrix() -> mat3x3<f32> {
    return mat3x3<f32>(
        env_data.cam_rotx,
        env_data.cam_roty,
        env_data.cam_rotz
    );
}


fn raymarch(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var t = 0.0;
    let max_dist = 100.0;
    let min_dist = 0.001;
    let max_steps = 128;

    for (var i = 0; i < max_steps; i++) {
        let p = ro + t * rd;
        let d = sdf_scene(p);
        if (d < min_dist) {
            return t;
        }
        t += d;
        if (t > max_dist) {
            return -1.0;
        }
       
    }

    return -1.0; // no hit
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

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u32(push.screen_size.x) || id.y >= u32(push.screen_size.y)) {
        return;
    }

    let uv = ((vec2<f32>(id.xy)) / push.screen_size);
    //Reduce skew due to percision issue on fullscreen.
    let camera_distance = length(env_data.cam_pos); // Assuming camera_data.pos is vec3<f32>
    let aspect = push.screen_size.x / push.screen_size.y;

    //Some sort of aspect ratio calculation here if needs to be more accurate
    var scale_factor = max(1.0 + (camera_distance * 0.0001), 1.075);
    if (push.mode == 1.0) {
        scale_factor = 1.0;
    }
    // Scale the UV coordinates dynamically
    let scaled_uv = uv * scale_factor;

    // Calculate the ray direction
    let dir_camera = screen_uv_to_ray_dir(scaled_uv, push.fov_y_deg, aspect);

    let origin = env_data.cam_pos; // Camera position remains the same

    let dist = raymarch(origin, dir_camera); // Use adjusted_ray_dir

    var out_val = 0.0;
    if (dist > 0.0) {
        out_val = 1.0; // Indicate a hit
    }
    
    let sun_dir = normalize(vec3<f32>(
        env_data.sunx,
        env_data.suny,
        env_data.sunz
    ));

    let hit_point = origin + dist * dir_camera;
    let normal = estimate_normal(hit_point);
    let light = max(dot(normal, -sun_dir), 0.0);

    let t = env_data.time * 20.0;
    let angle = dot(uv, vec2(12.0, 7.0)) + t; // or use dist from center
    let r = 0.5 + 0.5 * sin(angle);
    let g = 0.5 + 0.5 * sin(angle + 2.0);
    let b = 0.5 + 0.5 * sin(angle + 4.0);
    let color = vec3<f32>(r, g, b);

    textureStore(output, vec2<i32>(id.xy), vec4<f32>(color, out_val));
}