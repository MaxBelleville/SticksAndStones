#import "shaders/shared.wgsl"::{
    view, frame, reconstruct_world_ray, reconstruct_world_position
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


fn sky_color(ray_dir: vec3<f32>) -> vec3<f32> {
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let sun_dir = normalize(frame.sun_direction);

    let height = clamp(dot(ray_dir, up), 0.0, 1.0);

    // Gradient
    let horizon_color = vec3(0.9, 0.6, 0.3);
    let zenith_color  = vec3(0.1, 0.2, 0.6);

    var sky = mix(horizon_color, zenith_color, pow(height, 0.5));

    // Sun disk
    let sun_dot = dot(ray_dir, sun_dir);
    let sun = smoothstep(0.995, 0.999, sun_dot);
    sky += sun * vec3(1.0, 0.9, 0.7) * 5.0;

    return sky;
}
fn quantize(color: vec3<f32>, levels: f32) -> vec3<f32> {
    return floor(color * levels) / levels;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let slice_index = id.z;
    let coord = vec2<i32>(id.xy);
    
    let resolution = frame.resolution;
    if (id.x >= u32(resolution.x) || id.y >= u32(resolution.y) || slice_index >= 6u) {
        return;
    }  

    // Fetch G-buffer data
    var albedo_color = textureLoad(albedo_tex, coord, 0);
    let depth = textureLoad(depth_tex, coord, 0).x;

    // Reconstruct ray
    let ray = reconstruct_world_ray(coord);

    // --- Stylized Sky ---
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let sun_dir = normalize(frame.sun_direction);
    let height = clamp(dot(ray, up), 0.0, 1.0);

    var sky = vec3<f32>(0.0);
    
    // Soft forward scattering toward sun
    let mu = dot(ray, sun_dir);
    let forward = pow(max(mu, 0.0), 8.0);
    sky += forward * vec3(1.0, 0.8, 0.6);

    // Warm rim near horizon
    let horizon = pow(1.0 - height, 3.0);
    sky += horizon * vec3(0.8, 0.3, 0.2);

    // Quantize for retro stylized look
    //sky = quantize(sky, 48.0);

    // --- Distance-based fog ---
    if (depth > 0.0) {
        let world_pos = reconstruct_world_position(depth,coord);
        let dist = length(world_pos - view.world_position);

        // Control fog blending (adjust multiplier for stronger/weaker effect)
        let fog_amount = 1.0 - exp(-dist * 0.005);

        // Blend lit surface toward sky based on distance
        // albedo_color will eventually be replaced by fully lit_surface later
        albedo_color = vec4<f32>(mix(albedo_color.rgb, sky, fog_amount),1.0);
    } else {
        // Far plane, fully sky
        albedo_color = vec4<f32>(sky, 1.0);
    }

    // Write final HDR color
    textureStore(hdr_out_tex, coord, albedo_color);
}