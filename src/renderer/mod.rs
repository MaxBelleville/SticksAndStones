use bevy::app::PluginGroupBuilder;

use bevy::camera::visibility::RenderLayers;

use bevy::render::RenderStartup;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, PipelineCache,
    ShaderStages, TextureDescriptor, TextureDimension,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::view::{ExtractedView, ViewUniform, ViewUniformOffset, ViewUniforms};
use bevy::{
    pbr::MeshPipeline,
    prelude::*,
    render::{
        Render, RenderApp, RenderDebugFlags, RenderSystems,
        extract_resource::ExtractResourcePlugin,
        render_phase::SortedRenderPhasePlugin,
        render_resource::UniformBuffer,
        render_resource::{Extent3d, TextureFormat, TextureUsages},
        renderer::RenderDevice,
        texture::TextureCache,
    },
};
use bitflags::bitflags;

mod blit;
mod compatability;
mod compute;
mod lighting;
mod post;
mod raster;
mod raymarch;
mod structs;
mod enviroment;

use crate::window::WindowSettings;
pub struct RendererPlugins;
pub use structs::RasterMaterial;
pub use enviroment::{EnvironmentUniform,EnviromentInterpolated};

impl PluginGroup for RendererPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(RendererPlugin)
            .add(raster::RasterPlugin)
            .add(raymarch::RaymarchPlugin)
            .add(lighting::LightingPlugin)
            .add(post::PostPlugin)
            .add(blit::BlitPlugin)
            .add(compatability::CompatabilityPlugin)
        //add for in game ui plugins here)
    }
}

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        //Init resources and any thing that is shared between folders.
        app.add_plugins((
            ExtractComponentPlugin::<WorldModelCamera>::default(),
            ExtractResourcePlugin::<WindowSettings>::default(),
            ExtractResourcePlugin::<enviroment::EnvironmentUniform>::default(),
            SortedRenderPhasePlugin::<raster::RasterGBuffer3D, MeshPipeline>::new(
                RenderDebugFlags::default(),
            ),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<structs::SceneTargets>()
            .add_systems(RenderStartup, prepare_shared_layout)
            .add_systems(
                Render,
                (
                    prepare_scene_targets,
                    prepare_shared_group.after(prepare_scene_targets),
                )
                    .in_set(RenderSystems::PrepareBindGroups),
            );
    }
}

fn prepare_scene_targets(
    settings: Res<WindowSettings>,
    mut tex_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut targets: ResMut<structs::SceneTargets>,
) {
    let size = Extent3d {
        width: settings.dim.z.round() as u32,
        height: settings.dim.w.round() as u32,
        depth_or_array_layers: 1,
    };

    let ldr_desc = create_tex_storage_desc("LDRTexture", TextureFormat::Rgba8Unorm, size);

    let hdr_desc = create_tex_storage_desc("HdrTexture", TextureFormat::Rgba16Float, size);

    let albedo_desc = create_tex_storage_desc("AlbedoTexture", TextureFormat::Rgba8Unorm, size);

    let normal_desc = create_tex_storage_desc("NormalTexture", TextureFormat::Rgba16Float, size);
    let material_desc = create_tex_storage_desc("MaterialTexture", TextureFormat::Rgba8Unorm, size);

    let raster_depth_desc = create_tex_desc(
        "RasterDepthTexture",
        TextureFormat::Depth32Float,
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        size,
    );

    let combined_depth_desc =
        create_tex_storage_desc("CombinedDepthTexture", TextureFormat::R32Float, size);

    // Grab textures from cache (reused or freshly allocated)
    targets.set_hdr(tex_cache.get(&render_device, hdr_desc));
    targets.set_ldr(tex_cache.get(&render_device, ldr_desc));
    //Handle gbuffer
    targets.set_albedo(tex_cache.get(&render_device, albedo_desc));
    targets.set_normal(tex_cache.get(&render_device, normal_desc));
    targets.set_material(tex_cache.get(&render_device, material_desc));
    targets.set_raster_depth(tex_cache.get(&render_device, raster_depth_desc));
    targets.set_combined_depth(tex_cache.get(&render_device, combined_depth_desc));
}

pub fn prepare_shared_layout(mut commands: Commands) {
    let bind_layout = BindGroupLayoutDescriptor::new(
        "FrameLayout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<structs::FrameUniform>(false),
                uniform_buffer::<enviroment::EnvironmentUniform>(false),
            ),
        ),
    );
    commands.insert_resource(structs::FrameBindLayout::new(bind_layout));
}

pub fn prepare_shared_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    frame_layout: Res<structs::FrameBindLayout>,
    view_uniforms: Res<ViewUniforms>,
    queue: Res<RenderQueue>,
    settings: Res<WindowSettings>,
    enviroment: Res<enviroment::EnvironmentUniform>,
    views: Query<(Entity, &ViewUniformOffset), (With<ExtractedView>, With<WorldModelCamera>)>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };
    let Ok((_, view_offset)) = views.single() else {
        return;
    };

    let mut frame_buffer = UniformBuffer::from(structs::FrameUniform {
        res: vec2(settings.dim.z, settings.dim.w),
        frame_index: 0, //might nto always be true
    });

    frame_buffer.write_buffer(&render_device, &queue);

    let mut env_buffer = UniformBuffer::from(enviroment.into_inner());
    env_buffer.write_buffer(&render_device, &queue);

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(&frame_layout.get_layout()),
        &BindGroupEntries::sequential((
            view_binding.clone(),
            frame_buffer.binding().unwrap(),
            env_buffer.binding().unwrap(),
        )),
    );

    commands.insert_resource(structs::BindersRes::<structs::Frame>::new(
        bind_group,
        vec![view_offset.offset],
    ));
}

pub fn create_tex_desc<'w>(
    label: &'w str,
    format: TextureFormat,
    usage: TextureUsages,
    size: Extent3d,
) -> TextureDescriptor<'w> {
    TextureDescriptor {
        label: Some(label),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
        mip_level_count: 1,
    }
}

pub fn create_tex_storage_desc<'w>(
    label: &'w str,
    format: TextureFormat,
    size: Extent3d,
) -> TextureDescriptor<'w> {
    TextureDescriptor {
        label: Some(label),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    }
}
pub fn dispatch_size(dim: Vec4) -> UVec3 {
    uvec3(
        (dim.z.round() as u32 + 7) / 8,
        (dim.w.round() as u32 + 7) / 8,
        1,
    )
}

#[derive(Component, Debug, ExtractComponent, Clone)]
#[require(Transform, Visibility)]
pub struct WorldModelCamera;

#[derive(Component, Debug, ExtractComponent, Clone)]
#[require(Transform, Visibility)]
pub struct ViewModelCamera;

#[derive(Component, Debug, ExtractComponent, Clone)]
#[require(Transform, Visibility)]
pub struct UIModelCamera;

pub enum CameraOrder {
    World,
    ViewModel,
    Ui,
}

impl From<CameraOrder> for isize {
    fn from(order: CameraOrder) -> Self {
        order as isize
    }
}

bitflags! {
    pub struct RenderLayer: u32 {
        /// Used implicitly by all entities without a `RenderLayers` component.
        /// Our world model camera and all objects other than the player are on this layer.
        /// The light source belongs to both layers.
        const DEFAULT = 0b00000001;
        /// Used by the view model camera and the player's arm.
        /// The light source belongs to both layers.
        const VIEW_MODEL = 0b00000010;
        /// Since we use multiple cameras, we need to be explicit about
        /// which one is allowed to render particles.
        const PARTICLES = 0b00000100;
        /// 3D gizmos. These need to be rendered only by a 3D camera, otherwise the UI camera will render them in a buggy way.
        /// Specifically, the UI camera is a 2D camera, which by default is placed at a far away Z position,
        /// so it will effectively render a very zoomed out view of the scene in the center of the screen.
        const GIZMO3 = 0b0001000;
        /// UI elements.
        const UI = 0b0010000;
    }
}

impl From<RenderLayer> for RenderLayers {
    fn from(layer: RenderLayer) -> Self {
        // Render layers are just vectors of ints, so we convert each active bit to an int.
        RenderLayers::from_iter(layer.iter().map(|l| (l.bits() >> 1) as usize))
    }
}
