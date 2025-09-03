use super::{res, shared};
use bevy::log::info;
use bevy::{
    core_pipeline::blit::{BlitPipeline, BlitPipelineKey},
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::{
            BindGroupEntries, BindGroupEntry, BindingResource, CachedRenderPipelineId, LoadOp,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            SpecializedRenderPipelines, StoreOp, TextureFormat, TextureView,
        },
        renderer::RenderContext,
        texture::GpuImage,
        view::ViewTarget,
    },
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct BlitRenderLabel;

#[derive(Default)]
pub struct BlitNode;

impl ViewNode for BlitNode {
    type ViewQuery = (&'static ViewTarget,);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (_view_target,): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(targets) = world.get_resource::<res::GBufferTargets>() else {
            return Ok(());
        };
        let Some(final_out) = &targets.final_out else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let runtime = world.resource::<BlitRuntime>();

        // If we haven’t prepared it yet, or it’s not compiled this frame, bail out gracefully.
        let Some(id) = runtime.id else {
            return Ok(());
        };
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(id) else {
            info!("blit pipeline not ready yet");
            return Ok(());
        };

        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let worlddata = world.resource::<shared::res::WorldData>();
        let Some(gpu_image) = gpu_images.get(&worlddata.render) else {
            return Ok(());
        };
        let target_view: &TextureView = &gpu_image.texture_view;

        // bind group
        let blit_pipeline = world.resource::<BlitPipeline>();
        let bind_group = render_context.render_device().create_bind_group(
            "BlitBindGroup",
            &blit_pipeline.texture_bind_group,
            &BindGroupEntries::sequential((&final_out.default_view, &blit_pipeline.sampler)),
        );

        // render pass
        let mut pass = render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("BlitPass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        pass.set_pipeline(render_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
        //info!("blit drew fullscreen triangle");
        Ok(())
    }
}
#[derive(Resource, Default, Clone, Copy)]
pub struct BlitRuntime {
    id: Option<CachedRenderPipelineId>,
}

pub fn prepare_blit_pipeline(
    blit_pipeline: Res<BlitPipeline>,
    mut pipes: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    cache: Res<PipelineCache>,
    mut runtime: ResMut<BlitRuntime>,
) {
    // Match your UI target format
    let key = BlitPipelineKey {
        texture_format: TextureFormat::Bgra8UnormSrgb,
        blend_state: None,
        samples: 1,
    };
    let id = pipes.specialize(&cache, &*blit_pipeline, key);
    runtime.id = Some(id);
}
