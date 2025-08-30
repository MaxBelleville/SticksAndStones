use super::{res, shared, utils};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderLabel, ViewNode},
        render_resource::{
            BindGroupEntries, BindGroupLayoutEntries, CachedRenderPipelineId, LoadOp, Operations,
            PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages,
            StoreOp, TextureSampleType, binding_types::texture_2d,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PrefillDepthRenderLabel;

#[derive(Default)]
pub struct PrefillDepthNode;

impl ViewNode for PrefillDepthNode {
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

        let targets = world.resource::<res::GBufferTargets>();
        let pipeline = world.resource::<PrefillDepthPipeline>();
        let cache = world.resource::<PipelineCache>();
        let Some(current) = &bind.prefill_depth else {
            return Ok(());
        };
        let depth_view = &targets.depth_attach.as_ref().unwrap().default_view;

        let mut pass = render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("prefill_depth_pass"),
                color_attachments: &[/* none or load existing color if needed */],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }), // maybe clear or load
                    stencil_ops: None,
                }),

                timestamp_writes: None,
                occlusion_query_set: None,
            });

        if let Some(render_pipeline) = cache.get_render_pipeline(pipeline.prefill_pipeid) {
            pass.set_pipeline(render_pipeline); // <-- now type matches
        }
        pass.set_bind_group(0, current, &[]);
        pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

pub fn prepare_bind_group(
    render_device: Res<RenderDevice>,
    targets: Res<res::GBufferTargets>,
    mut render_bind_groups: ResMut<res::RenderBindGroups>,
) {
    let layout = render_device.create_bind_group_layout(
        "PrefillDepthLayout",
        &BindGroupLayoutEntries::single(
            ShaderStages::FRAGMENT,
            texture_2d(TextureSampleType::Float { filterable: false }),
        ),
    );

    let bind_group = render_device.create_bind_group(
        "PrefillDepthBindGroup",
        &layout,
        &BindGroupEntries::single(&targets.ray_depth.as_ref().unwrap().default_view),
    );

    render_bind_groups.prefill_depth = Some(bind_group);
}

#[derive(Resource)]
pub struct PrefillDepthPipeline {
    pub prefill_pipeid: CachedRenderPipelineId,
}

impl FromWorld for PrefillDepthPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.load_asset(shared::PREFILL_PATH);
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "PrefillDepthLayout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                texture_2d(TextureSampleType::Float { filterable: false }),
            ),
        );

        let pipeline_cache = world.resource::<PipelineCache>();
        let prefill_pipeid =
            pipeline_cache.queue_render_pipeline(utils::render_pipeline_descriptor_depth(
                shader,
                "fs_main",
                "vs_main",
                &vec![bind_group_layout],
            ));

        PrefillDepthPipeline { prefill_pipeid }
    }
}
