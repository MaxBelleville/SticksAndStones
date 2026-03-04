use bevy::{
    anti_alias::{fxaa::Fxaa, taa::TemporalAntiAliasing},
    camera::visibility::RenderLayers,
    core_pipeline::{
        prepass::{DeferredPrepass, DepthPrepass},
        tonemapping::Tonemapping,
    },
    light::ShadowFilteringMethod,
    post_process::bloom::Bloom,
    prelude::*,
    render::{extract_component::ExtractComponent, view::Hdr},
};
use leafwing_input_manager::prelude::*;

use crate::renderer;
use crate::ui;

/// Key configuration
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum Input {
    // Movement
    #[actionlike(DualAxis)]
    Move,
    #[actionlike(DualAxis)]
    Look,

    Jump,
    Sprint,
    Crouch,
    // Menus
    Pause,
    Inventory,
    QuickSelect,
    Chat,
    Interaction, //Not necessarily a menu but typically is
    Map,
    PlayerList,
    Console,
    Quest,
    //Attack Buttons
    LeftAttack,  //Left Click
    RightAttack, //Right Click

    //Use Items and Weapon Abilities
    UsePrimary,   //Uses primary magic,
    UseSecondary, //Uses secondary magic
    UseConsumable,
    //Hotswap weapons
    SwapForward,
    SwapBackward,
}

impl Input {
    const MOVE_MODS: [Self; 2] = [Input::Sprint, Input::Crouch];

    const MENUS: [Self; 9] = [
        Input::Pause,
        Input::Inventory,
        Input::QuickSelect,
        Input::Chat,
        Input::Interaction,
        Input::PlayerList,
        Input::Map,
        Input::Console,
        Input::Quest,
    ];

    const ATTACK: [Self; 2] = [Input::LeftAttack, Input::RightAttack];
    const USE: [Self; 3] = [Input::UsePrimary, Input::UseSecondary, Input::UseConsumable];
    const SWAP: [Self; 2] = [Input::SwapForward, Input::SwapBackward];
}

impl Input {
    fn default_input_map() -> InputMap<Input> {
        let mut input_map = InputMap::default();
        input_map.insert_dual_axis(
            Input::Move,
            VirtualDPad::wasd()
                .with_circle_deadzone(0.1)
                .inverted_y()
                .reset_processing_pipeline(),
        );

        input_map.insert_dual_axis(
            Input::Look,
            MouseMove::default()
                .replace_processing_pipeline([DualAxisSensitivity::new(0.002, 0.004).into()]),
        );

        input_map.insert(Input::Jump, KeyCode::Space);

        input_map.insert(Input::Sprint, KeyCode::ShiftLeft);
        input_map.insert(Input::Crouch, KeyCode::ControlLeft);

        input_map.insert(Input::LeftAttack, GamepadButton::West);
        input_map.insert(Input::RightAttack, GamepadButton::North);

        input_map.insert(Input::Pause, KeyCode::Escape);

        input_map.insert(Input::SwapForward, KeyCode::KeyZ);
        input_map.insert(Input::SwapBackward, KeyCode::KeyC);

        input_map.insert(Input::QuickSelect, KeyCode::Backquote);
        input_map.insert(Input::Inventory, KeyCode::KeyI);
        input_map.insert(Input::Chat, KeyCode::KeyT);
        input_map.insert(Input::Interaction, KeyCode::KeyF);
        input_map.insert(Input::Map, KeyCode::KeyM);
        input_map.insert(Input::Console, KeyCode::Slash);
        input_map.insert(Input::Quest, KeyCode::KeyJ);
        input_map.insert(Input::PlayerList, KeyCode::Tab);

        input_map.insert(Input::UsePrimary, KeyCode::KeyQ);
        input_map.insert(Input::UseSecondary, KeyCode::KeyE);
        input_map.insert(Input::UseConsumable, KeyCode::KeyR);

        input_map
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Input::default_input_map())
            .insert_resource(PlayerSettings::default())
            .add_systems(OnEnter(ui::ManagerState::InGame), spawn_player)
            .add_systems(
                Update,
                (player_look, player_move).run_if(in_state(ui::GameState::Normal)),
            )
            .add_systems(
                Update,
                change_mode.run_if(in_state(ui::ManagerState::InGame)),
            );
    }
}

#[derive(Resource, Debug)]
pub struct PlayerSettings {
    pub fly_mode: bool,
    pub move_speed: f32,
    pub sprint_speed: f32,
    pub crouch_speed: f32,
    pub jump_height: f32,
    pub jump_speed: f32,
}
impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            fly_mode: true,
            move_speed: 5.0,
            sprint_speed: 10.0,
            crouch_speed: 3.0,
            jump_height: 1.0,
            jump_speed: 5.0,
        }
    }
}

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
struct Player;

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct CameraPivot;

#[derive(Component, Debug, ExtractComponent,Clone)]
#[require(Transform, Visibility)]
pub struct WorldModelCamera;



fn spawn_player(mut commands: Commands) {
    commands
        .spawn((
            Player,
            Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::X, Vec3::Y),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // The "Head" pivot - handle vertical rotation (Pitch) here
            parent
                .spawn((CameraPivot, Transform::IDENTITY, Visibility::default()))
                .with_children(|head| {
                    head.spawn((
                        WorldModelCamera,
                        Camera3d::default(),
                        Projection::from(PerspectiveProjection {
                            fov: 75.0_f32.to_radians(),
                            ..default()
                        }),
                        Camera {
                            order: renderer::CameraOrder::World.into(),
                            clear_color: Color::srgb_u8(15, 9, 20).into(),
                            ..default()
                        },
                        Tonemapping::TonyMcMapface,
                        Bloom::NATURAL,
                        Hdr,
                        RenderLayers::from(
                            renderer::RenderLayer::DEFAULT
                                | renderer::RenderLayer::PARTICLES
                                | renderer::RenderLayer::GIZMO3,
                        ),
                        (
                            Msaa::Off,
                      
                            DeferredPrepass,
                        ),
                    ));

                    // Spawn view model camera.
                    head.spawn((
                        Camera3d::default(),
                        Camera {
                            // Bump the order to render on top of the world model.
                            order: renderer::CameraOrder::ViewModel.into(),
                            ..default()
                        },
                        Hdr,
                        Projection::from(PerspectiveProjection {
                            // We use whatever FOV we set in the animation software, e.g. Blender.
                            // Tip: if you want to set a camera in Blender to the same defaults as Bevy,
                            // see [this issue](https://github.com/kaosat-dev/Blenvy/issues/223)
                            fov: 62.0_f32.to_radians(),
                            ..default()
                        }),
                        Tonemapping::TonyMcMapface,
                        // Only render objects belonging to the view model.
                        RenderLayers::from(renderer::RenderLayer::VIEW_MODEL),
                        (DepthPrepass, Msaa::Off, DeferredPrepass, Fxaa::default()),
                    ));
                });
        });
}

fn player_look(
    mut player_query: Query<&mut Transform, With<Player>>,
    mut head_query: Query<&mut Transform, (With<CameraPivot>, Without<Player>)>,
    action_state: Res<ActionState<Input>>,
) {
    let mut player_transform = player_query.single_mut().expect("No player?");
    let mut head_transform = head_query.single_mut().expect("No camera pivot point");

    // leafwing_input_manager accumulates all mouse movement since the last frame
    let look_delta = action_state.axis_pair(&Input::Look);

    if look_delta != Vec2::ZERO {
        // 1. Handle Horizontal (Yaw) on the Player
        let (mut yaw, _, _) = player_transform.rotation.to_euler(EulerRot::YXZ);
        yaw -= look_delta.x;

        player_transform.rotation = Quat::from_rotation_y(yaw);

        // 2. Handle Vertical (Pitch) on the Head
        let (_, mut pitch, _) = head_transform.rotation.to_euler(EulerRot::YXZ);
        pitch -= look_delta.y;

        // Clamp to prevent flipping over (89 degrees)
        pitch = pitch.clamp(-1.54, 1.54);

        head_transform.rotation = Quat::from_rotation_x(pitch);
    }
}

fn player_move(
    settings: Res<PlayerSettings>,
    time: Res<Time>,
    action_state: Res<ActionState<Input>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    head_query: Query<&Transform, (With<CameraPivot>, Without<Player>)>,
) {
    let mut player_transform = player_query.single_mut().expect("No player?");
    let head_transform = head_query.single().expect("No camera pivot point");

    let move_input = action_state.clamped_axis_pair(&Input::Move);

    if move_input != Vec2::ZERO {
        let speed = if action_state.pressed(&Input::Crouch) {
            settings.crouch_speed
        } else if action_state.pressed(&Input::Sprint) {
            settings.sprint_speed
        } else {
            settings.move_speed
        };

        let delta = time.delta_secs() * speed;

        // Calculate movement directions based on current rotation
        // forward() and right() are relative to the entity's local orientation

        let forward = player_transform.forward();

        let right = player_transform.right();

        // Move the player: move_input.y is W/S, move_input.x is A/D
        let mut move_vec = (forward * move_input.y) + (right * move_input.x);

        if settings.fly_mode {
            let y_forward = head_transform.forward();
            move_vec.y += y_forward.y * move_input.y;
        }
        // Apply movement (ignoring Y to stay on the ground)
        player_transform.translation += move_vec * delta;
    }
}

fn change_mode(
    action_state: Res<ActionState<Input>>,
    mut next_game: ResMut<NextState<ui::GameState>>,
    mut next_menu: ResMut<NextState<ui::MenuState>>,
    // We use the current state to check if we should toggle back to Normal
    game_state: Res<State<ui::GameState>>,
) {
    for input in Input::MENUS.iter() {
        if action_state.just_pressed(input) {
            // Determine the target state based on the input
            let target_state = match input {
                Input::Pause => ui::GameState::Paused,
                Input::Inventory => ui::GameState::Inventory,
                Input::QuickSelect => ui::GameState::QuickSelect,
                Input::Chat => ui::GameState::Chat,
                Input::Interaction => ui::GameState::Interaction,
                Input::PlayerList => ui::GameState::PlayerList,
                Input::Map => ui::GameState::Map,
                Input::Console => ui::GameState::Console,
                Input::Quest => ui::GameState::Quest,
                _ => continue, // Skip any inputs not mapped to states
            };

            // Toggle logic: If we are already in the target state, go back to Normal
            if game_state.get() == &target_state {
                next_game.set(ui::GameState::Normal);
            } else {
                next_game.set(target_state);
            }

            if game_state.get() == &ui::GameState::Paused {
                next_menu.set(ui::MenuState::Disabled);
            }
            else {
                next_menu.set(ui::MenuState::Pause);
            }

            // Break after the first match to prevent multiple state changes in one frame
            break;
        }
    }
}
