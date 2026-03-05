
use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    math::FloatOrd,
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMaterialInstances, RenderMeshInstances
    },
    platform::collections::HashSet,
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderStartup, RenderSystems, batching::
            gpu_preprocessing::
                batch_and_prepare_sorted_render_phase, mesh::RenderMesh, render_asset::RenderAssets, render_graph::{
            RenderGraphExt, ViewNodeRunner
        }, render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, ViewSortedRenderPhases, sort_phase_system
        }, render_resource::{
            AsBindGroup, PipelineCache, SpecializedMeshPipelines
        }, renderer::RenderDevice, view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity}
    },
};
mod pipe;
mod phase;
mod node;

use super::utils;
use super::WorldModelCamera;

pub use phase::RasterGBuffer3D;

//Note technically this might be possible to be done just using the specialize function in the material.
//But it's easier to control and modify this (ie manually creating a specialized mesh pipeline), 
//For example the phases would be connected to the normal bevy renderer which would make it harder to connect to raymarch.
//So although it's a bit of a duplication of systems, it's helpful.

pub struct RasterPlugin;

impl Plugin for RasterPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedMeshPipelines<pipe::GBufferMeshPipeline>>()
            .init_resource::<DrawFunctions<phase::RasterGBuffer3D>>()
            .add_render_command::<phase::RasterGBuffer3D, phase::DrawGBufferMeshCommands>()
            .init_resource::<ViewSortedRenderPhases<phase::RasterGBuffer3D>>()
            .add_systems(RenderStartup, init_mesh_pipeline.after(super::prepare_scene_targets))
            .add_systems(ExtractSchedule, extract_camera_phases)
            .add_systems(
                Render,
                (
                    queue_meshes.in_set(RenderSystems::QueueMeshes),
                    sort_phase_system::<phase::RasterGBuffer3D>.in_set(RenderSystems::PhaseSort),
                    batch_and_prepare_sorted_render_phase::<phase::RasterGBuffer3D, pipe::GBufferMeshPipeline>
                        .in_set(RenderSystems::PrepareResources),
                ),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<node::RasterGBufferNode>>(Core3d, utils::RasterLabel)
            // Tell the node to run after the main pass
            .add_render_graph_edges(Core3d, (Node3d::MainOpaquePass, utils::RasterLabel));

    }
    

}

// When defining a phase, we need to extract it from the main world and add it to a resource
// that will be used by the render world. We need to give that resource all views that will use
// that phase
fn extract_camera_phases(
    mut raster_phases: ResMut<ViewSortedRenderPhases<phase::RasterGBuffer3D>>,
     cameras: Extract<Query<(Entity, &Camera), (With<Camera3d>, With<WorldModelCamera>)>>,
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


fn init_mesh_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
) {
    // Load the shader
    let shader_handle: Handle<Shader> = asset_server.load(utils::RASTER_SHADER_PATH);

    let material_layout =utils::RasterMaterial::bind_group_layout_descriptor(&render_device);

    commands.insert_resource(pipe::GBufferMeshPipeline::new(
        mesh_pipeline.clone(),
        shader_handle,
        material_layout
    ));
}

fn queue_meshes(
    draw_functions: Res<DrawFunctions<phase::RasterGBuffer3D>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<pipe::GBufferMeshPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    draw_pipeline: Res<pipe::GBufferMeshPipeline>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_instances: Res<RenderMaterialInstances>,
    mut render_phases: ResMut<ViewSortedRenderPhases<phase::RasterGBuffer3D>>,
    mut views: Query<
    (&ExtractedView, &RenderVisibleEntities, &Msaa),
    With<WorldModelCamera>
    >
) {
    for (view, visible_entities, msaa) in &mut views {
        let Some(phase) = render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let draw_custom = draw_functions.read().id::<phase::DrawGBufferMeshCommands>();

        // Create the key based on the view.
        // In this case we only care about MSAA and HDR
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        let rangefinder = view.rangefinder3d();
        // Since our phase can work on any 3d mesh we can reuse the default mesh 3d filter
        for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
            // We only want meshes with the marker component to be queued to our phase.

            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
         
            // Specialize the key for the current mesh entity
            // For this example we only specialize based on the mesh topology
            // but you could have more complex keys and that's where you'd need to create those keys
            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
            
            //mesh_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
            let Some(material_instance) = material_instances.instances.get(visible_entity) else {
                continue;
            };
            let result = material_instance.asset_id.try_typed();
            if result.is_err() {
                continue;
            }
            let mat_asset_id = result.unwrap();


            let key = pipe::GBufferPipelineKey {
                mesh_key,
                material_id: mat_asset_id,
            };

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &draw_pipeline,
                key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };
            let distance = rangefinder.distance(&mesh_instance.center);
            // At this point we have all the data we need to create a phase item and add it to our
            // phase
            phase.add(phase::RasterGBuffer3D {
                // Sort the data based on the distance to the view
                sort_key: FloatOrd(distance),
                entity: (*render_entity, *visible_entity),
                pipeline: pipeline_id,
                draw_function: draw_custom,
                // Sorted phase items aren't batched
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: mesh.indexed(),
            });
        }
    }
}
