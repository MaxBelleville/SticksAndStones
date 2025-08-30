#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,   // pull normal from mesh

};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
   var out: VertexOutput;
    // This is how bevy computes the world position
    // The vertex.instance_index is very important. Especially if you are using batching and gpu preprocessing
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal,vertex.instance_index);
    return out;

}

struct FragmentOutput {
    @builtin(frag_depth) depth_attach: f32,
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) ray_depth:f32,
};


@fragment
fn fragment(in: VertexOutput)-> FragmentOutput    
{
    var out: FragmentOutput;
    // write depth attachment (NDC z/w in WebGPU is 0..1)
   


    // Simple albedo (flat color)
    out.albedo = vec4(1.0, 0.0, 1.0, 1.0);

    // World-space normal, remapped to 0..1 for storage
    out.normal  = vec4(normalize(in.world_normal) * 0.5 + 0.5,1.0);

    // Linear depth (you can choose world_z, distance from camera, etc.)
    out.ray_depth = length(in.world_position.xyz) - out.depth_attach;

    return out;
}