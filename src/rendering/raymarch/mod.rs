use super::{shared,res, utils};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderLabel, ViewNode},
        render_resource::{
            CachedComputePipelineId, PipelineCache,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct RaymarchRenderLabel;

#[derive(Default)]
pub struct RaymarchNode;

impl ViewNode for RaymarchNode {
    type ViewQuery = (&'static ViewTarget,);

    fn run<'w>(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        _view_query: bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> std::result::Result<(), render_graph::NodeRunError> {
        let Some(bind) = world.get_resource::<res::RenderBindGroups>() else {
            return Ok(());
        };
        let pipeline = world.resource::<RaymarchPipeline>();
        let cache = world.resource::<PipelineCache>();
        let Some(current) = &bind.raymarch else {
            return Ok(());
        };
        let Some(shared) = &bind.shared else {
            return Ok(());
        };

        utils::run_compute_pass(
            render_context,
            vec![current,shared],
            cache,
            world
                .resource::<shared::res::WindowManagerSettings>()
                .clone(),
            pipeline.ray_pipeid,
        );

        Ok(())
    }
}

pub fn prepare_bind_group(  
    render_device: Res<RenderDevice>,
    gbf_layout: Res<res::GBufferLayout>,
    targets: Res<res::GBufferTargets>,
    mut render_bind_groups: ResMut<res::RenderBindGroups>,
) {
    render_bind_groups.raymarch = Some(utils::create_bind_group("GBufferRaycast",&render_device,&gbf_layout.write_layout,targets.into_inner()));
}

#[derive(Resource)]
pub struct RaymarchPipeline {
    pub ray_pipeid: CachedComputePipelineId,
}

impl FromWorld for RaymarchPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.load_asset(shared::RAYMARCH_PATH);
        let shared = world.resource::<res::SharedLayouts>();
        let gbf_layout = world.resource::<res::GBufferLayout>();

        let bind_group_layouts = vec![gbf_layout.write_layout.clone(), shared.layout.clone()];

        let pipeline_cache = world.resource::<PipelineCache>();
        let ray_pipeid = pipeline_cache.queue_compute_pipeline(
            utils::compute_pipeline_descriptor(shader, "main", &bind_group_layouts),
        );

        RaymarchPipeline {
            ray_pipeid,
        }
    }
}
