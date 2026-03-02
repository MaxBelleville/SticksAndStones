use bevy::app::PluginGroupBuilder;

use bevy::camera::visibility::RenderLayers;

use bevy::{
    pbr::MeshPipeline,
    prelude::*,
    render::{
        Render, RenderApp, RenderDebugFlags, RenderSystems,
        extract_component::ExtractComponentPlugin,
        extract_resource::ExtractResourcePlugin,
        render_phase::SortedRenderPhasePlugin,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
        texture::TextureCache,
    },
};
use bitflags::bitflags;

mod compatability;
mod lighting;
mod post;
mod raster;
mod raymarch;
pub mod utils;
use crate::window::WindowSettings;

pub struct RendererPlugins;

impl PluginGroup for RendererPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(RendererPlugin)
            .add(raster::RasterPlugin)
            .add(raymarch::RaymarchPlugin)
            .add(lighting::LightingPlugin)
            .add(post::PostPlugin)
            .add(compatability::CompatabilityPlugin)
        //add for in game ui plugins here)
    }
}

use raster::phase;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        //Init resources and any thing that is shared between folders.
        app.add_plugins((
            ExtractResourcePlugin::<WindowSettings>::default(),
            SortedRenderPhasePlugin::<phase::RasterGBuffer3D, MeshPipeline>::new(
                RenderDebugFlags::default(),
            ),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<utils::GBufferTargets>()
            .add_systems(
                Render,
                (prepare_gbuffer).in_set(RenderSystems::PrepareBindGroups),
            );
    }
}

fn prepare_gbuffer(
    settings: Res<WindowSettings>,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut targets: ResMut<utils::GBufferTargets>,
) {
    let size = Extent3d {
        width: settings.dim.z.round() as u32,
        height: settings.dim.w.round() as u32,
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

    let depth_desc = TextureDescriptor {
        label: Some("DepthTexture"),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    };

    let output_desc = TextureDescriptor {
        label: Some("RenderOutTexture"),
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

    // Grab textures from cache (reused or freshly allocated)
    targets.albedo = Some(texture_cache.get(&render_device, albedo_desc));
    targets.normal = Some(texture_cache.get(&render_device, normal_desc));
    targets.depth = Some(texture_cache.get(&render_device, depth_desc));
    targets.material = Some(texture_cache.get(&render_device, output_desc));
}

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
