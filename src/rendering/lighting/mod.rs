use super::{res, shared, utils};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderLabel, ViewNode},
        render_resource::{BindGroupEntries, CachedComputePipelineId, PipelineCache},
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct LightingRenderLabel;

#[derive(Default)]
pub struct LightingNode;

impl ViewNode for LightingNode {
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
        let pipeline = world.resource::<LightingPipeline>();
        let cache = world.resource::<PipelineCache>();
        let Some(current) = &bind.lighting else {
            return Ok(());
        };
        let Some(shared) = &bind.shared else {
            return Ok(());
        };


        utils::run_compute_pass(
            render_context,
            vec![current, shared],
            cache,
            world
                .resource::<shared::res::WindowManagerSettings>()
                .clone(),
            pipeline.lighting_pipeid,
        );
        // info!("lighting drew fullscreen triangle");
        Ok(())
    }
}

pub fn prepare_bind_group(
    render_device: Res<RenderDevice>,
    gbf_layout: Res<res::GBufferLayout>,
    targets: Res<res::GBufferTargets>,
    mut render_bind_groups: ResMut<res::RenderBindGroups>,
) {
    render_bind_groups.lighting = Some(render_device.create_bind_group(
         "GBufferLighting",
        &gbf_layout.read_layout,
        &BindGroupEntries::sequential((
            &targets.albedo.as_ref().unwrap().default_view,
            &targets.normal.as_ref().unwrap().default_view,
            &targets.raster_depth.as_ref().unwrap().default_view,
            &targets.final_out.as_ref().unwrap().default_view,
        )),
    ));
}

#[derive(Resource)]
pub struct LightingPipeline {
    pub lighting_pipeid: CachedComputePipelineId,
}

impl FromWorld for LightingPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.load_asset(shared::LIGHTING_PATH);
        let shared = world.resource::<res::SharedLayouts>();
        let gbf_layout = world.resource::<res::GBufferLayout>();

        let bind_group_layouts = vec![gbf_layout.read_layout.clone(), shared.layout.clone()];

        let pipeline_cache = world.resource::<PipelineCache>();
        let lighting_pipeid = pipeline_cache.queue_compute_pipeline(
            utils::compute_pipeline_descriptor(shader, "main", &bind_group_layouts),
        );

        LightingPipeline { lighting_pipeid }
    }
}
