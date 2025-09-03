#import bevy_pbr::{
    mesh_functions,
    mesh_bindings,
    mesh_view_bindings::view,
    forward_io::{Vertex, VertexOutput},
    view_transformations,
    prepass_utils,
    utils::coords_to_viewport_uv,
}

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

    let uv = coords_to_viewport_uv(in.position.xy,view.viewport);

    let color = textureSample(color_tex, color_tex_sampler, uv);
    let ray_depth =  textureLoad(ray_depth_tex, vec2<i32>(in.position.xy)).r;

    if(ray_depth!=0.0&&ray_depth>=in.position.z) {
        discard;
    }

    out.albedo =  color;
    // World-space normal, remapped to 0..1 for storage
    out.normal  = vec4(normalize(in.world_normal) * 0.5 + 0.5,1.0);

    out.raster_depth = in.position.z;

    return out;
}