use std::ops::Range;

use bevy::{
    core_pipeline::{
        oit::OrderIndependentTransparencySettings,
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    },
    ecs::query::QueryItem,
    math::FloatOrd,
    pbr::{DrawMesh, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup},
    platform::collections::HashSet,
    prelude::*,
    render::{
        camera::{Camera, ExtractedCamera}, mesh::RenderMesh, render_asset::RenderAssets, render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode}, render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
            PhaseItemExtraIndex, SetItemPipeline, SortedPhaseItem, ViewSortedRenderPhases,
        }, render_resource::{
            BindGroupEntries, CachedRenderPipelineId, LoadOp, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, SamplerDescriptor, SpecializedMeshPipelines, StoreOp
        }, renderer::{RenderContext, RenderDevice}, sync_world::MainEntity, view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewDepthTexture, ViewTarget}, Extract
    },
};

use super::{res, shared};

pub mod pipe;

#[derive(Default)]
pub struct RasterMainPassNode;
impl ViewNode for RasterMainPassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
          &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view,_target, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // First, we need to get our phases resource
        let Some(raster_phases) = world.get_resource::<ViewSortedRenderPhases<RasterMainPass>>()
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
        let gbuffer = match world.get_resource::<res::GBufferTargets>() {
            Some(g) => g,
            None => return Ok(()),
        };

        let render_bind_groups = match world.get_resource::<res::RenderBindGroups>() {
            Some(rbg) => rbg,
            None => return Ok(()),
        };

        let raster_group = match &render_bind_groups.raster {
            Some(r) => r,
            None => return Ok(()),
        };

        // must have the textures present
        let albedo_view = match &gbuffer.albedo {
            Some(t) => &t.default_view,
            None => return Ok(()),
        };
        let normal_view = match &gbuffer.normal {
            Some(t) => &t.default_view,
            None => return Ok(()),
        };
        let combined_depth_view = match &gbuffer.combined_depth {
            Some(t) => &t.default_view,
            None => return Ok(()),
        }; // depth_color as R32 (or separate)

        // Build render pass descriptor with your MRTs and depth attachment
        let color_attachments = &[
            Some(RenderPassColorAttachment {
                view: albedo_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // albedo clear
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: normal_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // neutral normal
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: combined_depth_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // depth_color clear (not used visually)
                    store: StoreOp::Store,
                },
            }),
        ];
 
        // Render pass setup
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("raster pass"),
            // For the purpose of the example, we will write directly to the view target. A real
            // stencil pass would write to a custom texture and that texture would be used in later
            // passes to render custom effects using it.
            color_attachments,
            // We don't bind any depth buffer for this pass
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }
        render_pass.set_bind_group(2, raster_group, &[]);
        // Render the phase
        // This will execute each draw functions of each phase items queued in this phase
        if let Err(err) = raster_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the stencil phase {err:?}");
        }
        // info!("raster drew fullscreen triangle");
        Ok(())
    }
}

// Render label used to order our render graph node that will render our phase
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct RasterRenderLabel;

pub type DrawMesh3dRaster = (
    SetItemPipeline,
    // This will set the view bindings in group 0
    SetMeshViewBindGroup<0>,
    // This will set the mesh bindings in group 1
    SetMeshBindGroup<1>,
    // This will draw the mesh
    DrawMesh,
);

pub fn prepare_bind_group(
    render_device: Res<RenderDevice>,
    gbf_layout: Res<res::GBufferLayout>,
    gbuffer: Res<res::GBufferTargets>,
    views: Query<&ViewTarget, With<shared::res::ComputeCamera>>,
    mut render_bind_groups: ResMut<res::RenderBindGroups>,
) {
    if let Ok(target) = views.single() {

        render_bind_groups.raster = Some(render_device.create_bind_group(
            "GBufferRaster",
            &gbf_layout.raster_layout,
            &BindGroupEntries::sequential((
                target.main_texture_view(),
                &render_device.create_sampler(&SamplerDescriptor::default()),
                &gbuffer.ray_depth.as_ref().unwrap().default_view,
            )),
        ));
    } else {
        panic!("View not found in raster");
    }
}

// When defining a phase, we need to extract it from the main world and add it to a resource
// that will be used by the render world. We need to give that resource all views that will use
// that phase
pub fn extract_camera_phases(
    mut raster_phases: ResMut<ViewSortedRenderPhases<RasterMainPass>>,
    cameras: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();
    for (main_entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }
        // This is the main camera, so we use the first subview index (0)
        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);

        raster_phases.insert_or_clear(retained_view_entity);
        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views.
    raster_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

// This is a very important step when writing a custom phase.
//
// This system determines which meshes will be added to the phase.
pub fn queue_meshes(
    draw_function: Res<DrawFunctions<RasterMainPass>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<pipe::RasterPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    draw_pipeline: Res<pipe::RasterPipeline>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    mut render_phases: ResMut<ViewSortedRenderPhases<RasterMainPass>>,
    views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &Msaa,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
            Has<OrderIndependentTransparencySettings>,
        ),
    )>,
) {
    for (
        view,
        visible_entities,
        msaa,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass, oit),
    ) in &views
    {
        if let Some(phase) = render_phases.get_mut(&view.retained_view_entity) {
            let draw_custom = draw_function.read().id::<DrawMesh3dRaster>();

            let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
                | MeshPipelineKey::from_hdr(view.hdr);

            if normal_prepass {
                view_key |= MeshPipelineKey::NORMAL_PREPASS;
            }

            if depth_prepass {
                view_key |= MeshPipelineKey::DEPTH_PREPASS;
            }

            if motion_vector_prepass {
                view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
            }

            if deferred_prepass {
                view_key |= MeshPipelineKey::DEFERRED_PREPASS;
            }

            if oit {
                view_key |= MeshPipelineKey::OIT_ENABLED;
            }

            for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
                let Some(mesh_instance) =
                    render_mesh_instances.render_mesh_queue_data(*visible_entity)
                else {
                    continue;
                };
                let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                    continue;
                };

                // Start from the known-good view key
                let mut mesh_key = view_key;
                mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

                let pipeline_id = match pipelines.specialize(
                    &pipeline_cache,
                    &draw_pipeline,
                    mesh_key,
                    &mesh.layout,
                ) {
                    Ok(id) => id,
                    Err(err) => {
                        error!("specialization error: {}", err);
                        continue;
                    }
                };

                let distance = view
                    .rangefinder3d()
                    .distance_translation(&mesh_instance.translation);

                phase.add(RasterMainPass {
                    sort_key: FloatOrd(distance),
                    entity: (*render_entity, *visible_entity),
                    pipeline: pipeline_id,
                    draw_function: draw_custom,
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::None,
                    indexed: mesh.indexed(),
                });
            }
        }
    }
}
pub struct RasterMainPass {
    pub sort_key: FloatOrd,
    pub entity: (Entity, MainEntity),
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    /// Whether the mesh in question is indexed (uses an index buffer in
    /// addition to its vertex buffer).
    pub indexed: bool,
}

// For more information about writing a phase item, please look at the custom_phase_item example
impl PhaseItem for RasterMainPass {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for RasterMainPass {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // bevy normally uses radsort instead of the std slice::sort_by_key
        // radsort is a stable radix sort that performed better than `slice::sort_by_key` or `slice::sort_unstable_by_key`.
        // Since it is not re-exported by bevy, we just use the std sort for the purpose of the example
        items.sort_by_key(SortedPhaseItem::sort_key);
    }

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for RasterMainPass {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}
