use std::borrow::Cow;

use bevy::{
    mesh::MeshVertexBufferLayoutRef, pbr::{MaterialPipeline, MaterialPipelineKey}, prelude::*, render::{
        extract_component::ExtractComponent,
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayoutDescriptor, CachedComputePipelineId, ColorTargetState, ColorWrites, ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureFormat
        },
        renderer::RenderContext,
        texture::CachedTexture,
    }, shader::ShaderRef
};
use bytemuck::{Pod, Zeroable};

use crate::window::WindowSettings;
pub const RASTER_SHADER_PATH: &str = "shaders/raster.wgsl";

#[derive(Resource, Default)]

pub struct GBufferTargets {
    pub albedo: Option<CachedTexture>,
    pub normal: Option<CachedTexture>,
    pub material: Option<CachedTexture>,
    pub depth: Option<CachedTexture>,
}
#[derive(Component)]
pub struct RasterMaterial3d(pub Handle<RasterMaterial>);

#[derive(ShaderType, Clone, Debug)]
pub struct RasterMaterialUniform {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
}
#[derive(Asset,TypePath,AsBindGroup, Debug, Clone)]
pub struct RasterMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub base_color_texture: Option<Handle<Image>>,

    #[texture(2)]
    #[sampler(3)]
    pub normal_texture: Option<Handle<Image>>,

    #[texture(4)]
    #[sampler(5)]
    pub metallic_roughness_texture: Option<Handle<Image>>,

    #[uniform(6)]
    pub material_uniform: RasterMaterialUniform,
}
impl Material for RasterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/geom_internal.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}


#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct EnviromentData {
    cam_rotx: Vec3,
    sunx: f32,
    cam_roty: Vec3,
    suny: f32,
    cam_rotz: Vec3,
    sunz: f32,
    cam_pos: Vec3,
    time: f32,
}

pub fn compute_pipeline_descriptor(
    shader: Handle<Shader>,
    entry_point: &str,
    bind_group_layouts: &Vec<BindGroupLayoutDescriptor>,
) -> ComputePipelineDescriptor {
    ComputePipelineDescriptor {
        label: None,
        layout: bind_group_layouts.clone(),
        shader,
        shader_defs: vec![],
        entry_point: Some(Cow::from(entry_point.to_owned())),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    }
}

pub fn run_compute_pass(
    render_context: &mut RenderContext,
    bind_groups: Vec<&BindGroup>,
    pipeline_cache: &PipelineCache,
    settings: WindowSettings,
    pipeline: CachedComputePipelineId,
) {
    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());
    for (index, bind_group) in bind_groups.iter().enumerate() {
        pass.set_bind_group(index as u32, *bind_group, &[]);
    }
    let pipeline = pipeline_cache.get_compute_pipeline(pipeline).unwrap();
    pass.set_pipeline(pipeline);

    pass.dispatch_workgroups(
        (settings.dim.z.round() as u32 + 7) / 8,
        (settings.dim.w.round() as u32 + 7) / 8,
        1,
    );
}
