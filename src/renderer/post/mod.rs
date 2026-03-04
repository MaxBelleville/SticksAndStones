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
pub struct PostPlugin;

use super::compute;
use super::utils;

impl Plugin for PostPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                RenderStartup,
                compute::init_pipeline::<PostPass>.after(super::prepare_shared_layout),
            )
            .add_systems(
                Render,
                (prepare_bind_group.after(super::prepare_shared_group))
                    .in_set(RenderSystems::PrepareBindGroups),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<compute::ComputeNode<PostPass>>>(
                Core3d,
                utils::PostLabel,
            )
            .add_render_graph_edges(Core3d, (utils::LightingLabel, utils::PostLabel));
    }
}
fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<compute::ComputePipelineRes<PostPass>>,
    scene: Res<utils::SceneTargets>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
        let bind_group = render_device.create_bind_group(
            None,
            &pipeline_cache.get_bind_group_layout(pipeline.get_layout()),
            &BindGroupEntries::sequential((scene.hdr_view().unwrap(), scene.ldr_view().unwrap())),
        );
        commands
            .insert_resource(utils::BindersRes::<utils::Post>::new_empty(bind_group));
    
}
#[derive(Default)]
pub struct PostPass;

impl compute::ComputePassSpec for PostPass {
    type Marker = utils::Post;
    type Frame = utils::Frame;

    const SHADER_PATH: &'static str = utils::POST_SHADER_PATH;

    fn create_layout() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "PostBindGroup",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }), // hdr
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                ),
            ),
        )
    }

    const ENTRY: &'static str = "main";
}
