use bevy::camera::CameraOutputMode;
use bevy::prelude::*;

use bevy::{core_pipeline::tonemapping::Tonemapping, render::render_resource::BlendState};

use crate::shared;

pub struct HdrPlugin;

impl Plugin for HdrPlugin {
    fn build(&self, app: &mut App) {
         app.add_observer(make_hdr_compatible);
    }
}

fn make_hdr_compatible(
    add: On<Add, Camera>,
    mut cameras: Query<&mut Camera>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let mut camera = cameras.get_mut(entity).unwrap();
    if camera.order == shared::CameraOrder::World as isize {
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