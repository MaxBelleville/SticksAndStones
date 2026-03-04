use bevy::{
    core_pipeline::{
        blit::{BlitPipeline, BlitPipelineKey},
        core_3d::graph::Core3d,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems, render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt as _, ViewNode, ViewNodeRunner,
        }, render_resource::{
            BindGroupEntries, CachedRenderPipelineId, LoadOp, Operations, PipelineCache,
            RenderPassColorAttachment, RenderPassDescriptor, SpecializedRenderPipelines, StoreOp,
            TextureFormat,
        }, renderer::{RenderContext, RenderDevice}, view::ViewTarget
    },
};

use super::utils;

pub struct BlitPlugin;

impl Plugin for BlitPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
         render_app.add_systems(
            Render,
            (
                prepare_blit_runtime.in_set(RenderSystems::Prepare),
                (prepare_bind_group.after(super::prepare_scene_targets))
                    .in_set(RenderSystems::PrepareBindGroups),
            ),
        );

        render_app
            .add_render_graph_node::<ViewNodeRunner<BlitNode>>(Core3d, utils::BlitLabel)
            .add_render_graph_edges(Core3d, (utils::PostLabel, utils::BlitLabel));
    }
}

#[derive(Default)]
pub struct BlitNode;
impl ViewNode for BlitNode {
    type ViewQuery = &'static ViewTarget;

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        view_target: QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> std::result::Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let runtime = world.resource::<BlitRuntime>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(runtime.pipeline) else {
            return Ok(());
        };

        let bind_group = world.resource::<utils::BindersRes<utils::Blit>>();
        
        let mut pass = render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("blit_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: view_target.main_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group.get_first_group(), bind_group.get_first_offset());
        pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
pub struct BlitRuntime {
    pipeline: CachedRenderPipelineId,
}

pub fn prepare_blit_runtime(
    mut commands: Commands,
    blit_pipeline: Res<BlitPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    cache: Res<PipelineCache>,
) {
    // Match your UI target format

    let key = BlitPipelineKey {
        texture_format: TextureFormat::Rgba16Float,
        blend_state: None,
        samples: 1,
    };
    let pipeline = pipelines.specialize(&cache, &*blit_pipeline, key);

    commands.insert_resource(BlitRuntime { pipeline });
}

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<BlitPipeline>,
    scene: Res<utils::SceneTargets>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((scene.ldr_view().unwrap(), &pipeline.sampler)),
    );
    commands.insert_resource(utils::BindersRes::<utils::Blit>::new_empty(bind_group));
}
