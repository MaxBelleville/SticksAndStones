use super::phase;
use bevy::camera::Viewport;
use bevy::render::render_resource::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, StoreOp};

use bevy::{
    camera::MainPassResolutionOverride,
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_phase::ViewSortedRenderPhases,
        render_resource::RenderPassDescriptor,
        renderer::RenderContext,
        view::{ExtractedView, ViewTarget},
    },
};
use super::utils;

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct RasterGBufferLabel;

#[derive(Default)]
pub struct RasterGBufferNode;
impl ViewNode for RasterGBufferNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        Option<&'static MainPassResolutionOverride>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, target, resolution_override): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // First, we need to get our phases resource
        let Some(raster_phases) = world.get_resource::<ViewSortedRenderPhases<phase::RasterGBuffer3D>>()
        else {
            return Ok(());
        };

        // Get the view entity from the graph
        let view_entity = graph.view_entity();

        // Get the phase for the current view running our node
        let Some(raster_phase) = raster_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        // get GBuffer targets (render-world resource)
        let gbuffer = match world.get_resource::<utils::GBufferTargets>() {
            Some(g) => g,
            None => return Ok(()),
        };

        let albedo_view = match &gbuffer.albedo {
            Some(t) => &t.default_view,
            None => return Ok(()),
        };
        let normal_view = match &gbuffer.normal {
            Some(t) => &t.default_view,
            None => return Ok(()),
        };
        let depth_view = match &gbuffer.depth {
            Some(t) => &t.default_view,
            None => return Ok(()),
        };

         let color_attachments = &[
            Some(RenderPassColorAttachment {
                view: albedo_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(LinearRgba::BLACK.into()), // albedo clear
                    store: StoreOp::Store,
                },
                depth_slice: None,
            }),
            Some(RenderPassColorAttachment {
                view: normal_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(LinearRgba::BLACK.into()), // neutral normal
                    store: StoreOp::Store,
                },
                depth_slice: None,
            }),
        ];
        let depth_stencil_attachment = Some(RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        });


        // Render pass setup
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("raster_pass"),
            // For the purpose of the example, we will write directly to the view target. A real
            // stencil pass would write to a custom texture and that texture would be used in later
            // passes to render custom effects using it.
            color_attachments,
            // We don't bind any depth buffer for this pass
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        // Render the phase
        // This will execute each draw functions of each phase items queued in this phase
        if let Err(err) = raster_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the stencil phase {err:?}");
        }

        Ok(())
    }
}
