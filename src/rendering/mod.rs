use super::shared;
use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    pbr::MeshPipeline,
    prelude::*,
    render::{
        Render, RenderApp, RenderDebugFlags, RenderSet,
        batching::gpu_preprocessing::batch_and_prepare_sorted_render_phase,
        extract_component::ExtractComponentPlugin,
        extract_resource::ExtractResourcePlugin,
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_phase::{
            AddRenderCommand, DrawFunctions, SortedRenderPhasePlugin, ViewSortedRenderPhases,
            sort_phase_system,
        },
        render_resource::*,
        renderer::RenderDevice,
        texture::TextureCache,
        view::{ExtractedView, ViewDepthTexture},
    },
};
mod lighting;
mod prefill_depth;
mod raster;
mod raymarch;
mod res;
mod utils;

pub struct RenderSystemPlugin;

impl Plugin for RenderSystemPlugin {
    fn build(&self, app: &mut App) {
        //Extract resources into render
        app.add_plugins((
            ExtractComponentPlugin::<shared::res::ComputeCamera>::default(),
            ExtractResourcePlugin::<shared::res::WindowManagerSettings>::default(),
            ExtractResourcePlugin::<shared::res::AtmosphereData>::default(),
            ExtractResourcePlugin::<shared::res::ExtractedGameState>::default(),
            ExtractResourcePlugin::<shared::res::WorldData>::default(),
            ExtractComponentPlugin::<raster::DrawRaster>::default(),
            SortedRenderPhasePlugin::<raster::RasterMainPass, MeshPipeline>::new(
                RenderDebugFlags::default(),
            ),
        ));

        // get the render_app
        let render_app = app.sub_app_mut(RenderApp);

        // init render-only resources (in render world)
        render_app
            .init_resource::<res::RenderBindGroups>()
            .init_resource::<res::GBufferTargets>()
            .init_resource::<SpecializedMeshPipelines<raster::pipe::RasterPipeline>>()
            .init_resource::<DrawFunctions<raster::RasterMainPass>>()
            .add_render_command::<raster::RasterMainPass, raster::DrawMesh3dRaster>()
            .init_resource::<ViewSortedRenderPhases<raster::RasterMainPass>>()
            .add_systems(ExtractSchedule, raster::extract_camera_phases);

        // Prepare systems: create gbuffer textures first, then shared bind groups,
        // then per-pass bind group preparations. Put them in the PrepareBindGroups set,
        // and ensure ordering so dependencies are ready.
        render_app.add_systems(
            Render,
            (
                (
                    // Create/reserve the G-buffer textures (must run first)
                    prepare_gbuffer,
                    // Shared uniforms (camera/atmo/svt) depend on the view and depth texture
                    prepare_shared_bind_group.after(prepare_gbuffer),
                    // Per-pass bind groups depend on both G-buffer targets + shared bindgroup existing
                    raymarch::prepare_bind_group
                        .after(prepare_shared_bind_group)
                        .after(prepare_gbuffer),
                    prefill_depth::prepare_bind_group
                        .after(prepare_shared_bind_group)
                        .after(prepare_gbuffer),
                    lighting::prepare_bind_group
                        .after(prepare_shared_bind_group)
                        .after(prepare_gbuffer),
                )
                    .in_set(RenderSet::PrepareBindGroups)
                    .run_if(in_extracted_game_state),
                (
                    raster::queue_meshes.in_set(RenderSet::QueueMeshes),
                    sort_phase_system::<raster::RasterMainPass>.in_set(RenderSet::PhaseSort),
                    batch_and_prepare_sorted_render_phase::<
                        raster::RasterMainPass,
                        raster::pipe::RasterPipeline,
                    >
                        .in_set(RenderSet::PrepareResources),
                ),
            ),
        );

        // Register per-view render graph nodes.
        // Use ViewNodeRunner so the node runs for every view that matches the ViewQuery inside the node type.

        // Add the nodes as view-nodes
        render_app
            .add_render_graph_node::<ViewNodeRunner<raymarch::RaymarchNode>>(
                Core3d,
                raymarch::RaymarchRenderLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<prefill_depth::PrefillDepthNode>>(
                Core3d,
                prefill_depth::PrefillDepthRenderLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<raster::RasterMainPassNode>>(
                Core3d,
                raster::RasterRenderLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<lighting::LightingNode>>(
                Core3d,
                lighting::LightingRenderLabel,
            );

        // Hook ordering into core_3d graph.
        // The tuple below automatically creates the edges so the nodes execute in the desired order.
        // NOTE: Node3d names may vary by Bevy version; replace with your version's constants if needed.
        render_app.add_render_graph_edges(
            Core3d,
            // run raymarch after prepass finishes and before main pass starts
            (
                Node3d::EndPrepasses,
                raymarch::RaymarchRenderLabel,
                prefill_depth::PrefillDepthRenderLabel,
                raster::RasterRenderLabel,
                Node3d::EndMainPass,
                lighting::LightingRenderLabel,
                Node3d::Tonemapping,
            ),
        );
    }

    fn finish(&self, app: &mut App) {
        // init pipeline resources (after main app build so RenderDevice exists)
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<res::SharedLayouts>()
            .init_resource::<res::GBufferLayout>()
            .init_resource::<raymarch::RaymarchPipeline>()
            .init_resource::<prefill_depth::PrefillDepthPipeline>()
            .init_resource::<raster::pipe::RasterPipeline>()
            .init_resource::<lighting::LightingPipeline>();
    }
}

fn in_extracted_game_state(resource: Res<shared::res::ExtractedGameState>) -> bool {
    resource.0 == shared::GameState::InGame
}

fn prepare_gbuffer(
    settings: Res<shared::res::WindowManagerSettings>,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut targets: ResMut<res::GBufferTargets>,
) {
    let size = Extent3d {
        width: settings.dim.z as u32,
        height: settings.dim.w as u32,
        depth_or_array_layers: 1,
    };

    let albedo_desc = TextureDescriptor {
        label: Some("AlbedoTexture"),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    };
    let normal_desc = TextureDescriptor {
        label: Some("NormalTexture"),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    };

    let ray_depth_desc = TextureDescriptor {
        label: Some("RayDepthTexture"),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R32Float,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    };

    let depth_attach_desc = TextureDescriptor {
        label: Some("DepthAttach"),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    };

    // Grab textures from cache (reused or freshly allocated)
    targets.albedo = Some(texture_cache.get(&render_device, albedo_desc));
    targets.normal = Some(texture_cache.get(&render_device, normal_desc));
    targets.ray_depth = Some(texture_cache.get(&render_device, ray_depth_desc));
    targets.depth_attach = Some(texture_cache.get(&render_device, depth_attach_desc));
}

fn prepare_shared_bind_group(
    shared: Res<res::SharedLayouts>,
    views: Query<(&ExtractedView, &ViewDepthTexture), With<shared::res::ComputeCamera>>,
    settings: Res<shared::res::WindowManagerSettings>,
    world_data: Res<shared::res::WorldData>,
    mut bind_groups: ResMut<res::RenderBindGroups>,
    atmo_data: Res<shared::res::AtmosphereData>,
    render_device: Res<RenderDevice>,
) {
    if let Ok((view, depth_texture)) = views.single() {
        //Prepares buffers will be used to bind data later.

        let cam_pos = view.world_from_view.translation();
        let rot = Mat3::from_quat(view.world_from_view.rotation());

        let camera_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("EnviromentData"),
            contents: bytemuck::bytes_of(&utils::EnviromentData::new(
                atmo_data.into_inner(),
                rot,
                cam_pos,
            )),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let world = world_data.into_inner();
        let push_constants = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("PushContants"),
            contents: bytemuck::bytes_of(&utils::PushConstants::new(world, settings.into_inner())),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let svt_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("SVTArray"),
            contents: bytemuck::cast_slice(&world.gpu_data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        //Loads depth, camera, "push consts" and svt buffer data.
        let bind_group_shared = render_device.create_bind_group(
            "shared_bind",
            &shared.layout,
            &BindGroupEntries::sequential((
                depth_texture.view(),
                camera_buffer.as_entire_binding(),
                svt_buffer.as_entire_binding(),
                push_constants.as_entire_binding(),
            )),
        );
        bind_groups.shared = Some(bind_group_shared);
    } else {
        panic!("View not found");
    }
}
