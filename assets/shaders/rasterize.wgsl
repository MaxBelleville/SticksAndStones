#import bevy_pbr::{
    mesh_functions,
    mesh_bindings,
    mesh_view_bindings::view,
    forward_io::{Vertex,VertexOutput},
    view_transformations,
    prepass_utils,
}




const MAX_STEPS: u32 = 1024u;
const EPSILON: f32 = 1e-5;
const MAX_DIST: f32 = 2000.0;



@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    #ifdef VERTEX_NORMALS
        out.world_normal = mesh_functions::mesh_normal_local_to_world(
            vertex.normal,
            // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
            // See https://github.com/gfx-rs/naga/issues/2416
            vertex.instance_index
        );
    #endif

    #ifdef VERTEX_POSITIONS
        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
        out.position = view_transformations::position_world_to_clip(out.world_position.xyz); 
    #endif

    #ifdef VERTEX_UVS_A
        out.uv = vertex.uv;
    #endif
    #ifdef VERTEX_UVS_B
        out.uv_b = vertex.uv_b;
    #endif

    #ifdef VERTEX_TANGENTS
        out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
            world_from_local,
            vertex.tangent,
            // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
            // See https://github.com/gfx-rs/naga/issues/2416
            vertex.instance_index
        );
    #endif

    #ifdef VERTEX_COLORS
        out.color = vertex.color;
    #endif

    #ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        out.instance_index = vertex.instance_index;
    #endif

    #ifdef VISIBILITY_RANGE_DITHER
        out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
            vertex.instance_index, mesh_world_from_local[3]);
    #endif
        return out;
}


@group(2) @binding(0) var color_tex: texture_2d<f32>;
@group(2) @binding(1) var color_tex_sampler: sampler;
@group(2) @binding(2) var ray_depth_tex: texture_storage_2d<r32float, read>;

struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) raster_depth:f32,
};


@fragment
fn fragment(in: VertexOutput)-> FragmentOutput    
{
    var out: FragmentOutput;

    let uv = view_transformations::frag_coord_to_uv(in.position.xy);
        // Motion vector in screen space (normalized to viewport)
   let motion_vec = prepass_utils::prepass_motion_vector(in.position, 0);

    // Reproject UV backwards into where this pixel came from last frame
    let prev_uv = uv - motion_vec;

    let color = textureSample(color_tex, color_tex_sampler, prev_uv);
   

    let dims: vec2<u32> = textureDimensions(ray_depth_tex);
    //Probably not needed but just to be safe clamp depth
    let texel_f = uv * vec2<f32>(vec2<i32>(dims));
    let texel_x = clamp(u32(floor(texel_f.x)), 0u, dims.x - 1);
    let texel_y = clamp(u32(floor(texel_f.y)), 0u, dims.y - 1);
    let tex_coord = vec2<u32>(texel_x, texel_y);
    let ray_depth =  textureLoad(ray_depth_tex, tex_coord).r;

    let view_pos = view_transformations::position_world_to_view(in.world_position.xyz);
    let linear_depth = -view_pos.z;              // positive forward
    var raster_depth = linear_depth / MAX_DIST;
    let depth_bias = 0.001 + 0.002 * length(motion_vec*1.5);
    if(ray_depth!=1.0&&ray_depth<raster_depth+depth_bias) {
        discard;
    }
 
    out.albedo = color;
    
    // World-space normal, remapped to 0..1 for storage
    out.normal  = vec4(normalize(in.world_normal) * 0.5 + 0.5,1.0);

    out.raster_depth = raster_depth;

    return out;
}