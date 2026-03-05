
#import "shaders/shared.wgsl"::{
    view, frame,EPSILON,MAX_STEPS,MAX_DIST,PI
}
#import bevy_render::{
    view::{position_ndc_to_view,position_clip_to_view,position_world_to_view,view_z_to_depth_ndc,uv_to_ndc,direction_view_to_world},
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
fn sphere_sdf(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}
fn box_sdf(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn intersect_sdf(a: f32, b: f32) -> f32 {
    return max(a, b);
}

fn union_sdf(a: f32, b: f32) -> f32 {
    return min(a, b);
}

fn difference_sdf(a: f32, b: f32) -> f32 {
    return max(a, -b);
}

fn repeated(p: vec3<f32>, s:f32 ) ->f32
{
    let r = p - s*round(p/s);
    return sphere_sdf(r,0.2);
}

fn scene_sdf(p: vec3<f32>) -> f32 {
    return repeated(p, 10.0);
}

fn raymarch(origin: vec3<f32>, dir: vec3<f32>) -> f32 {
    var depth = 0.0;

    for (var i: u32 = 0u; i < MAX_STEPS; i = i + 1u) {
        let p = origin + dir * depth;
        let dist = scene_sdf(p);

        if (dist < EPSILON) {
            return depth;
        }

        depth = depth + dist;

        if (depth >= MAX_DIST) {
            break;
        }
    }

    return MAX_DIST;
}

fn estimate_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(EPSILON, 0.0);

    let n = vec3<f32>(
        scene_sdf(p + vec3<f32>(e.x, e.y, e.y)) - scene_sdf(p - vec3<f32>(e.x, e.y, e.y)),
        scene_sdf(p + vec3<f32>(e.y, e.x, e.y)) - scene_sdf(p - vec3<f32>(e.y, e.x, e.y)),
        scene_sdf(p + vec3<f32>(e.y, e.y, e.x)) - scene_sdf(p - vec3<f32>(e.y, e.y, e.x))
    );

    return normalize(n);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {

    let coord = vec2<i32>(id.xy);
    
    let resolution = frame.resolution;
    if (id.x >= u32(resolution.x) || id.y >= u32(resolution.y)) {
        return;
    }
    let raster_depth = textureLoad(depth_tex, coord, 0);


    //convert to NDC
    let uv = (vec2<f32>(id.xy) + 0.5) / resolution;
    let ndc_xy = uv_to_ndc(uv);
    let clip_pos = vec4<f32>(ndc_xy, 1.0, 1.0);
    let view_pos = position_ndc_to_view(vec3<f32>(ndc_xy, 1.0), view.view_from_clip);

    //view to ray dir
    let ray_dir_view = normalize(view_pos);
    let ray_origin_view = vec3<f32>(0.0);

    let ray_origin_world = view.world_position;
    let ray_dir_world = normalize(direction_view_to_world(ray_dir_view, view.world_from_view));

    //raymarch and get hit pos/raydepth
    let hit_dist = raymarch(ray_origin_world, ray_dir_world);
    let hit_pos_view = ray_origin_view + ray_dir_view * hit_dist;
    let ray_depth = view_z_to_depth_ndc(hit_pos_view.z, view.clip_from_view);

    if (hit_dist < MAX_DIST && ray_depth >= raster_depth) {
        let normal = estimate_normal(hit_pos_view);

        let albedo = vec3<f32>(0.8, 0.7, 0.6);
        textureStore(albedo_tex, coord, vec4<f32>(albedo, 1.0));
        textureStore(normal_tex, coord, vec4<f32>(normal, 1.0));
        textureStore(combined_depth_tex, coord, vec4<f32>(ray_depth));
    }
    else {
        textureStore(combined_depth_tex, coord, vec4<f32>(raster_depth));
    }
 
}