use std::f32::consts::FRAC_PI_2;

use bevy::{
    anti_alias::fxaa::Fxaa,
    app::PluginGroupBuilder,
    camera::visibility::RenderLayers,
    core_pipeline::
        prepass::{DeferredPrepass, DepthPrepass}
    ,
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
};

use crate::shared;
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
        app.add_systems(OnEnter(ui::ManagerState::InGame), spawn_view_model)
            .add_systems(Update, (move_player, change_mode));
    }
}

#[derive(Debug, Component)]
struct Player;

#[derive(Debug, Component, Deref, DerefMut)]
struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(
            // These factors are just arbitrary mouse sensitivity values.
            // It's often nicer to have a faster horizontal sensitivity than vertical.
            // We use a component for them so that we can make them user-configurable at runtime
            // for accessibility reasons.
            // It also allows you to inspect them in an editor if you `Reflect` the component.
            Vec2::new(0.003, 0.002),
        )
    }
}

#[derive(Debug, Component)]
struct WorldModelCamera;
//Todo move out of here to player plugin.
fn spawn_view_model(mut commands: Commands) {
    //Setup 3d camera
    commands
        .spawn((
            Player,
            CameraSensitivity::default(),
            Transform::from_xyz(0.0, 1.0, 0.0),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                WorldModelCamera,
                Camera3d::default(),
                Projection::from(PerspectiveProjection {
                    fov: 75.0_f32.to_radians(),
                    ..default()
                }),
                Camera {
                    order: shared::CameraOrder::World.into(),
                    clear_color: Color::srgb_u8(15, 9, 20).into(),
                    ..default()
                },
                RenderLayers::from(
                    shared::RenderLayer::DEFAULT
                        | shared::RenderLayer::PARTICLES
                        | shared::RenderLayer::GIZMO3,
                ),
                (Msaa::Off, DeferredPrepass),
            ));

            // Spawn view model camera.
            parent.spawn((
                Camera3d::default(),
                Camera {
                    // Bump the order to render on top of the world model.
                    order: shared::CameraOrder::ViewModel.into(),
                    ..default()
                },
                Projection::from(PerspectiveProjection {
                    // We use whatever FOV we set in the animation software, e.g. Blender.
                    // Tip: if you want to set a camera in Blender to the same defaults as Bevy,
                    // see [this issue](https://github.com/kaosat-dev/Blenvy/issues/223)
                    fov: 62.0_f32.to_radians(),
                    ..default()
                }),
                // Only render objects belonging to the view model.
                RenderLayers::from(shared::RenderLayer::VIEW_MODEL),
                (DepthPrepass, Msaa::Off, DeferredPrepass, Fxaa::default()),
            ));
        });
}

fn move_player(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();

    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        // Note that we are not multiplying by delta_time here.
        // The reason is that for mouse movement, we already get the full movement that happened since the last frame.
        // This means that if we multiply by delta_time, we will get a smaller rotation than intended by the user.
        // This situation is reversed when reading e.g. analog input from a gamepad however, where the same rules
        // as for keyboard input apply. Such an input should be multiplied by delta_time to get the intended rotation
        // independent of the framerate.
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        // If the pitch was ±¹⁄₂ π, the camera would look straight up or down.
        // When the user wants to move the camera back to the horizon, which way should the camera face?
        // The camera has no way of knowing what direction was "forward" before landing in that extreme position,
        // so the direction picked will for all intents and purposes be arbitrary.
        // Another issue is that for mathematical reasons, the yaw will effectively be flipped when the pitch is at the extremes.
        // To not run into these issues, we clamp the pitch to a safe range.
        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
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
