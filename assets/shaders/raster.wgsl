//! A shader showing how to use the vertex position data to output the 
//! stencil in the right position

// First we import everything we need from bevy_pbr
// A 2d shader would be vevry similar but import from bevy_sprite instead
#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip
}

struct Vertex {
    // This is needed if you are using batching and/or gpu preprocessing
    // It's a built in so you don't need to define it in the vertex layout
    @builtin(instance_index) instance_index: u32,
    // Like we defined for the vertex layout
    // position is at location 0
    @location(0) position: vec3<f32>,
    // uv is at location 1
    @location(1) uv: vec2<f32>,
};

// This is the output of the vertex shader and we also use it as the input for the fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    out.world_position =
        mesh_functions::mesh_position_local_to_world(
            world_from_local,
            vec4(vertex.position, 1.0)
        );

    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.uv = vertex.uv;

    return out;
}

struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@group(3) @binding(0)
var base_color_texture: texture_2d<f32>;
@group(3) @binding(1)
var base_color_sampler: sampler;
@group(3) @binding(2)
var normal_texture: texture_2d<f32>;
@group(3) @binding(3)
var normal_sampler: sampler;
@group(3) @binding(4)
var metallic_roughness_texture: texture_2d<f32>;
@group(3) @binding(5)
var metallic_roughness_sampler: sampler;

@group(3) @binding(6)
var<uniform> material: RasterMaterialUniform;

struct RasterMaterialUniform {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _padding: vec2<f32>,
};

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    let tex_color = textureSample(
        base_color_texture,
        base_color_sampler,
        in.uv // TEMP, replace with UV later
    );

     let normal_color = textureSample(
        normal_texture,
        normal_sampler,
        in.uv // TEMP, replace with UV later
    );

    out.albedo = tex_color * material.base_color;
    out.normal = normal_color;

    return out;
}