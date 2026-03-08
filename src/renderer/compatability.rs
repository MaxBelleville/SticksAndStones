use bevy::camera::CameraOutputMode;
use bevy::prelude::*;

use bevy::{core_pipeline::tonemapping::Tonemapping, render::render_resource::BlendState};
use super::structs;
pub struct CompatabilityPlugin;

impl Plugin for CompatabilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(make_hdr_compatible)
            .add_observer(convert_standard_material);
    }
}

fn make_hdr_compatible(
    add: On<Add, Camera>,
    mut cameras: Query<&mut Camera>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let mut camera = cameras.get_mut(entity).unwrap();
    if camera.order == isize::from(super::CameraOrder::World) {
        // Use the world model camera to determine tonemapping.
        return;
    }
    // Needed because of https://github.com/bevyengine/bevy/issues/18902
    commands.entity(entity).insert(Tonemapping::None);

    // Needed because of https://github.com/bevyengine/bevy/issues/18901
    // and https://github.com/bevyengine/bevy/issues/18903
    camera.clear_color = ClearColorConfig::Custom(Color::NONE);
    camera.output_mode = CameraOutputMode::Write {
        blend_state: Some(BlendState::ALPHA_BLENDING),
        clear_color: ClearColorConfig::None,
    };
}

fn convert_standard_material(
    add: On<Add, MeshMaterial3d<StandardMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    mut raster_material_assets: ResMut<Assets<structs::RasterMaterial>>,
    mut query: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let mat_handle = query.get_mut(entity).unwrap();
     if let Some(std_mat) = materials.get(&mat_handle.0) {
        let raster_mat = structs::RasterMaterial {
            base_color_texture: std_mat.base_color_texture.clone(),
            normal_texture: std_mat.normal_map_texture.clone(),
            metallic_roughness_texture: std_mat.metallic_roughness_texture.clone(),
            material_uniform: structs::RasterMaterialUniform {
                base_color: LinearRgba::from(std_mat.base_color).to_vec4(),
                metallic: std_mat.metallic,
                roughness: std_mat.perceptual_roughness
            }
        };
        let handle = raster_material_assets.add(raster_mat);

        commands.entity(entity)
            .insert(MeshMaterial3d(handle))
            .remove::<MeshMaterial3d<StandardMaterial>>();
    }
    
}