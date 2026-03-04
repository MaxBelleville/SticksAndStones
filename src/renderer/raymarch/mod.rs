use bevy::{
    core_pipeline::core_3d::graph::Core3d,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_graph::{RenderGraphExt, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, PipelineCache,
            ShaderStages, StorageTextureAccess, TextureFormat,
            binding_types::{texture_depth_2d, texture_storage_2d},
        },
        renderer::RenderDevice,
    },
};

pub struct RaymarchPlugin;

use super::compute;

use super::utils;

impl Plugin for RaymarchPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                RenderStartup,
                compute::init_pipeline::<RaymarchPass>.after(super::prepare_shared_layout),
            )
            .add_systems(
                Render,
                (prepare_bind_group.after(super::prepare_shared_group))
                    .in_set(RenderSystems::PrepareBindGroups),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<compute::ComputeNode<RaymarchPass>>>(
                Core3d,
                utils::RaymarchLabel,
            )
            .add_render_graph_edges(Core3d, (utils::RasterLabel, utils::RaymarchLabel));
    }
}
fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<compute::ComputePipelineRes<RaymarchPass>>,
    scene: Res<utils::SceneTargets>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
        let bind_group = render_device.create_bind_group(
            None,
            &pipeline_cache.get_bind_group_layout(pipeline.get_layout()),
            &BindGroupEntries::sequential((
                scene.raster_depth_view().unwrap(),
                scene.albedo_view().unwrap(),
                scene.normal_view().unwrap(),
                scene.material_view().unwrap(),
                scene.combined_depth_view().unwrap(),
            )),
        );
        commands
            .insert_resource(utils::Binder::<utils::Raymarch>::new_empty(bind_group));
    
}
#[derive(Default)]
pub struct RaymarchPass;

impl compute::ComputePassSpec for RaymarchPass {
    type Marker = utils::Raymarch;
    type Frame = utils::Frame;

    const SHADER_PATH: &'static str = utils::RAYMARCH_SHADER_PATH;

    fn create_layout() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "RaymarchBindGroup",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_depth_2d(),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        )
    }

    const ENTRY: &'static str = "main";

}
