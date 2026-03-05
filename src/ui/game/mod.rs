use std::f32::consts::{FRAC_PI_4, PI};

use bevy::{
    app::PluginGroupBuilder,
    prelude::*,
};

use crate::ui;

mod chat;
mod console;
mod hud;
mod inventory;
mod item;
mod map;
mod player_list;
mod quick_select;

pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(GamePlugin)
        .add(chat::ChatPlugin)
        .add(console::ConsolePlugin)
        .add(hud::HudPlugin)
        .add(inventory::InventoryPlugin)
        .add(item::ItemPlugin)
        .add(map::MapPlugin)
        .add(player_list::PlayerListPlugin)
        .add(quick_select::QuickSelectPlugin)
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ui::ManagerState::InGame),
            spawn_world,
        );
    }
}
//TODO: CREATE ENVIROMENT AMBASSORDOR OR GENERAL ENVIROMENT MODULE FOR SUN/LIGHTING ETC.

//TODO REMOVE, USED JUST FOR DEBUGING SPECIALIZED MESHES.
fn spawn_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let sphere = meshes.add(Sphere::new(0.5));
    let wall = meshes.add(Cuboid::new(0.2, 4.0, 3.0));
    
    let blue_material = materials.add(Color::srgb_u8(0, 0, 255));
    let red_material = materials.add(Color::srgb_u8(255, 0, 0));
    let white_material = materials.add(Color::srgb_u8(255, 255, 255));

    // Top side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(white_material.clone()),
    ));
    // Under side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(0.0, -0.01, 0.0).with_rotation(Quat::from_rotation_x(PI)),
    ));
    // Blue sphere
    commands.spawn((
        Mesh3d(sphere.clone()),
        MeshMaterial3d(blue_material.clone()),
        Transform::from_xyz(3.0, 1.5, 0.0),
    ));
    // Tall wall
    commands.spawn((
        Mesh3d(wall.clone()),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(-3.0, 2.0, 0.0),
    ));
    // Cube behind wall
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(blue_material.clone()),
        Transform::from_xyz(-4.2, 0.5, 0.0),
    ));
    // Hidden cube under floor
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(red_material.clone()),
        Transform {
            translation: Vec3::new(3.0, -2.0, 5.0),
            rotation: Quat::from_euler(EulerRot::YXZEx, FRAC_PI_4, FRAC_PI_4, 0.0),
            ..default()
        },
    ));
}
