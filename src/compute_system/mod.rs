use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::*}, window::WindowMode,
};
use bytemuck::{Pod, Zeroable};

use super::shared;
mod comp_utils;
mod node;
mod render_pipe;
mod renderer;


#[derive(Component)]
pub struct ComputePlaneComponent;

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct EnviromentData {
     cam_rotx: Vec3,
     sunx: f32,
     cam_roty: Vec3,
     suny: f32,
     cam_rotz: Vec3,
     sunz: f32,
     cam_pos: Vec3,
     time: f32,
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Default, Zeroable)]
struct PushConstants {
    res: Vec2,
    fov_y_deg: f32, // 4 bytes (to align the Vec3)
    root_size: f32,
    root_min: Vec3,
    mode: f32, // 4 bytes (to bring total to 32)
}

impl PushConstants {
    fn new(res: &shared::res::SVTPushConsts, settings: &shared::res::WindowManagerSettings) -> Self {
        let mode = if settings.window_mode != WindowMode::Windowed {
            0.0
        }
        else {
            1.0
        };
        Self {
            res: vec2(settings.dim.z, settings.dim.w),
            root_size: res.root_size,
            root_min: res.root_min,
            fov_y_deg: 70.0,
            mode: mode
        }
    }
}


#[derive(Resource)]
struct ComputeBindGroups {
    output_frames: [BindGroup; 2],
    shared: BindGroup,
}

#[derive(Resource, Clone, ExtractResource)]
struct ComputeRenderImages {
    render_texture: Handle<Image>,
    render_buffer: Handle<Image>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ComputeRenderMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub render_texture: Handle<Image>,
}

impl Material for ComputeRenderMaterial {
    fn fragment_shader() -> ShaderRef {
        shared::FRAG_SHADER_PATH.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

}


pub struct ComputeSystemPlugin;

impl Plugin for ComputeSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(renderer::ComputeRendererPlugin)
        .add_systems(Startup, setup_renderer)
        .add_systems(Update, switch_buffer);
    }
}
//Setup the images and depth texture to render to and connect to the mat quad.
fn setup_renderer(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    settings: Res<shared::res::WindowManagerSettings>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: settings.dim.z as u32,
            height: settings.dim.w as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Add material to the mesh
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
  //  image.texture_descriptor.sample_count =1;
    let image_tex = images.add(image.clone());
    let image_buff = images.add(image);

    commands.insert_resource(ComputeRenderImages {
        render_texture: image_tex,
        render_buffer: image_buff,
    });
}


// Switch texture to display every frame to show the one that was written to most recently.
fn switch_buffer(
    images: Res<ComputeRenderImages>,
    mut materials: ResMut<Assets<ComputeRenderMaterial>>,
    mesh_query: Query<&MeshMaterial3d<ComputeRenderMaterial>, With<ComputePlaneComponent>>,
) {
    if let Ok(mesh) = mesh_query.single() {
        if let Some(material) = materials.get_mut(&mesh.0) {
            if material.render_texture == images.render_texture {
                material.render_texture = images.render_buffer.clone();
            } else {
                material.render_texture = images.render_texture.clone();
            }
        }
    }
}
