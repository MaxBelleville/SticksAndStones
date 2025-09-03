#import "shaders/shared.wgsl"::{
    push,env_data,screen_uv_to_ray_dir,RayEnd,MAX_STEPS,MAX_DIST,EPSILON,intersect_aabb,dither
}
#import "shaders/sky.wgsl"::{
    render_nishita
}

@group(0) @binding(0) var albedo: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var normal: texture_storage_2d<rgba16float, read>;
@group(0) @binding(2) var raster_depth: texture_storage_2d<r32float, read>;
@group(0) @binding(3) var final_out: texture_storage_2d<rgba8unorm, write>;

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
    let scale = 512 / 2f;
    // Tint albedo by depth (closer = full color, farther = darker)
    var out_color = albedo_val * (1.0 - depth_val);

    if(depth_val==1.0) {
       
         let uv = ((vec2<f32>(id.xy)) / push.screen_size);
        //Reduce skew due to percision issue on fullscreen.
        let aspect = push.screen_size.x / push.screen_size.y;

        // Calculate the ray direction
        let ray_dir = screen_uv_to_ray_dir(uv, push.fov_y_deg, aspect);
        let ray_org = env_data.cam_pos+ ray_dir*EPSILON;
        let sun_org = vec3<f32>(env_data.sunx,env_data.suny,env_data.sunz);
        out_color = vec4<f32>(render_nishita(ray_dir,ray_org,sun_org),1.0);
    }
    out_color += vec4(dither(vec2<f32>(id.xy)),0.0);

    // Write to output (force alpha=1)
    textureStore(final_out, coord, out_color);
}