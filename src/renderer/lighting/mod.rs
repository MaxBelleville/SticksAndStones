use bevy::{
    core_pipeline::core_3d::graph::Core3d,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_graph::{RenderGraphExt, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, PipelineCache,
            ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType,
            binding_types::{texture_2d, texture_storage_2d},
        },
        renderer::RenderDevice,
    },
};
pub struct LightingPlugin;

use super::compute;
use super::structs;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                RenderStartup,
                compute::init_pipeline::<LightingPass>.after(super::prepare_shared_layout),
            )
            .add_systems(
                Render,
                (prepare_bind_group.after(super::prepare_shared_group))
                    .in_set(RenderSystems::PrepareBindGroups),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<compute::ComputeNode<LightingPass>>>(
                Core3d,
                structs::LightingLabel,
            )
            .add_render_graph_edges(Core3d, (structs::RaymarchLabel, structs::LightingLabel));
    }
}
fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<compute::ComputePipelineRes<LightingPass>>,
    scene: Res<structs::SceneTargets>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(pipeline.get_layout()),
        &BindGroupEntries::sequential((
            scene.combined_depth_view().unwrap(),
            scene.albedo_view().unwrap(),
            scene.normal_view().unwrap(),
            scene.material_view().unwrap(),
            scene.hdr_view().unwrap(),
        )),
    );
    commands.insert_resource(structs::BindersRes::<structs::Lighting>::new_empty(bind_group));
}
#[derive(Default)]
pub struct LightingPass;

impl compute::ComputePassSpec for LightingPass {
    type Marker = structs::Lighting;
    type Frame = structs::Frame;

    const SHADER_PATH: &'static str = structs::LIGHTING_SHADER_PATH;

    fn create_layout() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "LightingBindGroup",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }), // combined depth(r32float)
                    texture_2d(TextureSampleType::Float { filterable: false }), // albedo
                    texture_2d(TextureSampleType::Float { filterable: false }), // normal
                    texture_2d(TextureSampleType::Float { filterable: false }), // material
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        )
    }

    const ENTRY: &'static str = "main";
}
