use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use super::shared;
// Since it's possible to have multiple `Actions` components, you need
// to define a marker and derive `InputContext` trait.
#[derive(InputContext)]
pub struct PlayerCam;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerMovementSettings>();
        app.add_input_context::<PlayerCam>();
        app.add_observer(bind_actions); // Bind the actions to the input context
        app.add_observer(apply_movement);
        app.add_observer(apply_rotate);
    }
}

fn bind_actions(
    trigger: Trigger<Bind<PlayerCam>>,
    mut players: Query<&mut Actions<PlayerCam>>,
    settings: Res<PlayerMovementSettings>,
) {
    let mut actions = players.get_mut(trigger.target()).unwrap();
    actions
        .bind::<Move>()
        .to((
            Cardinal::wasd_keys(),
            Cardinal::arrow_keys(),
            Axial::left_stick(),
        ))
        .with_modifiers((DeadZone::default(),));

    actions.bind::<Rotate>().to((
        // You can attach modifiers to individual inputs as well.
        Input::mouse_motion().with_modifiers((Scale::splat(settings.sensitivity), Negate::all())),
        Axial::right_stick()
            .with_modifiers_each((Scale::splat(settings.stick_sensitivity), Negate::x())),
    ));

    actions.bind::<Sprint>().to(KeyCode::ShiftLeft);
}

fn apply_rotate(trigger: Trigger<Fired<Rotate>>, mut players: Query<&mut Transform>, state: Res<State<shared::GameState>>) {
    if state.get() != &shared::GameState::InGame {
        return;
    }
    let mut transform = players.get_mut(trigger.target()).unwrap();
    let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

    yaw += trigger.value.x.to_radians();
    pitch += trigger.value.y.to_radians();
    pitch = pitch.clamp((-89.0f32).to_radians(), (89.0f32).to_radians());
    
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
}

fn apply_movement(
    trigger: Trigger<Fired<Move>>,
    flycam_act: Single<&Actions<PlayerCam>>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
    state: Res<State<shared::GameState>>,
    mut settings: ResMut<PlayerMovementSettings>,
) {
    if state.get() != &shared::GameState::InGame {
        return;
    }

    let actions = flycam_act.into_inner();
    let mut transform = transforms.get_mut(trigger.target()).unwrap();
    // Move to the camera direction.
    let rotation = transform.rotation;

    let mut movement = trigger.value.extend(0.0).xzy();
    movement.z = -movement.z;

    let mut accel_amount = settings.accel;
    if actions.state::<Sprint>().unwrap() == ActionState::Fired {
        accel_amount = settings.sprint_accel;
    }

    // Calculate acceleration
    let accel = if movement.length() != 0.0 {
        (rotation * movement).normalize() * accel_amount
    } else {
        Vec3::ZERO
    };

    settings.velocity += accel * time.delta_secs();

    // Apply friction (opposes current velocity)

    let friction: Vec3 = if settings.velocity.length() != 0.0 {
        settings.velocity.normalize() * -1.0 * settings.friction
    } else {
        Vec3::ZERO
    };

    // Clamp velocity to max_speed
    if settings.velocity.length() > settings.max_speed {
        settings.velocity = settings.velocity.normalize() * settings.max_speed;
    }

    let delta_friction = friction * time.delta_secs();

    settings.velocity =
        if (settings.velocity + delta_friction).signum() != settings.velocity.signum() {
            Vec3::ZERO
        } else {
            settings.velocity + delta_friction
        };
    transform.translation += settings.velocity;
}

// All actions should implement the `InputAction` trait.
// It can be done manually, but we provide a derive for convenience.
// The only necessary parameter is `output`, which defines the output type.
#[derive(Debug, InputAction)]
#[input_action(output = Vec3)]
struct Move;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct Rotate;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct Sprint;

#[derive(Resource)]
pub struct PlayerMovementSettings {
    pub accel: f32,
    pub sprint_accel: f32,
    pub max_speed: f32,
    pub sensitivity: f32,
    pub stick_sensitivity: f32,
    pub friction: f32,
    pub velocity: Vec3,
}

impl Default for PlayerMovementSettings {
    fn default() -> Self {
        Self {
            accel: 1.1,
            sprint_accel: 1.3,
            max_speed: 0.3,
            sensitivity: 0.1,
            stick_sensitivity: 2.0,
            friction: 1.0,
            velocity: Vec3::ZERO,
        }
    }
}
