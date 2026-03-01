use std::f32::consts::{FRAC_PI_4, PI};

use bevy::{
    app::PluginGroupBuilder,
    camera::visibility::RenderLayers,
    color::palettes::tailwind,
    prelude::*,
};

use crate::renderer;
use crate::ui;

pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(GamePlugin)
        //add for in game ui plugins here
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ui::ManagerState::InGame),
            (spawn_world, spawn_lights),
        );
    }
}

fn spawn_lights(mut commands: Commands) {
    // Main light
    commands.spawn((
        PointLight {
            color: Color::from(tailwind::ORANGE_300),
            shadows_enabled: true,
            ..default()
        },
        RenderLayers::from(renderer::RenderLayer::DEFAULT | renderer::RenderLayer::VIEW_MODEL),
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
    // Light behind wall
    commands.spawn((
        PointLight {
            color: Color::WHITE,

            shadows_enabled: true,

            ..default()
        },
        RenderLayers::from(renderer::RenderLayer::DEFAULT | renderer::RenderLayer::VIEW_MODEL),
        Transform::from_xyz(-3.5, 3.0, 0.0),
    ));
    // Light under floor
    commands.spawn((
        PointLight {
            color: Color::from(tailwind::RED_300),

            shadows_enabled: true,
            ..default()
        },
        RenderLayers::from(renderer::RenderLayer::DEFAULT | renderer::RenderLayer::VIEW_MODEL),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
}

fn spawn_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let sphere = meshes.add(Sphere::new(0.5));
    let wall = meshes.add(Cuboid::new(0.2, 4.0, 3.0));

    let blue_material = materials.add(StandardMaterial {
        base_color: tailwind::BLUE_700.into(),
        ..default()
    });
    let red_material = materials.add(StandardMaterial {
        base_color: tailwind::RED_700.into(),
        ..default()
    });
    let white_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });

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
            translation: Vec3::new(3.0, -2.0, 0.0),
            rotation: Quat::from_euler(EulerRot::YXZEx, FRAC_PI_4, FRAC_PI_4, 0.0),
            ..default()
        },
    ));
}

fn change_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut next_menu: ResMut<NextState<ui::MenuState>>,
    mut next_game: ResMut<NextState<ui::GameState>>,
    menu_state: Res<State<ui::MenuState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        //check if paused or not
        if *menu_state.get() == ui::MenuState::Disabled {
            next_menu.set(ui::MenuState::Pause);
            next_game.set(ui::GameState::Paused);
        } else {
            next_menu.set(ui::MenuState::Disabled);
            next_game.set(ui::GameState::Normal);
        }
    }
}
