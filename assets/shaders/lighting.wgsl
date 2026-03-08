#import "shaders/shared.wgsl"::{
    view,env, frame, reconstruct_world_ray, reconstruct_world_position, 
}

#import "shaders/enviroment.wgsl"::{
    sky_color, quantize, sun_disk, forward_scattering,
    horizon_rim, env_height, fog, sun_glow, alpha_moon_disk, alpha_moon_glow,
    beta_moon_disk, beta_moon_glow,sky_compensation, horizon_scattering, sky_lights
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
    let slice_index = id.z;
    let coord = vec2<i32>(id.xy);
    
    let resolution = frame.resolution;
    if (id.x >= u32(resolution.x) || id.y >= u32(resolution.y) || slice_index >= 6u) {
        return;
    }  

    // Fetch G-buffer data
    var albedo_color = textureLoad(albedo_tex, coord, 0);
    let normal_sample = textureLoad(normal_tex, coord, 0).xyz;
    let normal = normalize(normal_sample * 2.0 - 1.0);

    var material = textureLoad(material_tex, coord, 0);

    let depth = textureLoad(depth_tex, coord, 0).x;

    // Reconstruct ray
    let ray = reconstruct_world_ray(coord);

    let height = env_height(ray);
    var sky = sky_color(ray, height);

    sky+= horizon_scattering(ray, height);

    let compensation = sky_compensation(ray);
    sky = mix(sky, sky * compensation, 1.0 - height);

    // Quantize for retro stylized look
    sky = quantize(sky);


    
    let ambient = sky * 0.08;

    let horizon_light = env.dynamic.horizon_color * 0.15;
    let sky_fill = env.dynamic.zenith_color * 0.05;

    let sky_light = sky_lights(normal);
    let total = ambient +horizon_light + sky_fill  + sky_light;

    var lit_surface = albedo_color.rgb* total;

 
    // --- Distance-based fog ---
    if (depth > 0.0) {
        let world_pos = reconstruct_world_position(depth,coord);
        let dist = length(world_pos - view.world_position);
        let fog_amount = fog(world_pos, dist);

        // Blend lit surface toward sky based on distance
        // albedo_color will eventually be replaced by fully lit_surface later
        lit_surface = mix(lit_surface, sky, fog_amount);
    } else {
        //Add sun and moons
        sky+= sun_disk(ray);
        sky+= sun_glow(ray);

        sky+= alpha_moon_disk(ray);
        sky+= alpha_moon_glow(ray);

        sky+= beta_moon_disk(ray);
        sky+= beta_moon_glow(ray);
        // Far plane, fully sky
        lit_surface = sky;
    }

    // Write final HDR color
    textureStore(hdr_out_tex, coord, vec4<f32>(lit_surface, 1.0));
}