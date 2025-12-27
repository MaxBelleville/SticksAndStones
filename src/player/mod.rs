use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use super::shared;


#[derive(Component)]
pub struct Player;
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default())
        // The InputMap and ActionState components will be added to any entity with the Player component
        .add_systems(Startup, spawn_player)
        // Read the ActionState in your systems using queries!
        .add_systems(Update, jump);
    }
}

// This is the list of "things in the game I want to be able to do based on input"
#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
enum PlayerAction {
    Run,
    Jump,
}

fn spawn_player(mut commands: Commands) {
    // Describes how to convert from player inputs into those actions
    let input_map = InputMap::new([(PlayerAction::Jump, KeyCode::Space)]);
    commands.spawn(input_map).insert(Player);
}

// Query for the `ActionState` component in your game logic systems!
fn jump(query: Query<&ActionState<PlayerAction>, With<Player>>) {
    let action_state = query.single();
    if let Ok(action_state) = action_state {
        // Each action has a button-like state of its own that you can check
        if action_state.just_pressed(&PlayerAction::Jump) {
            println!("I'm jumping!");
        }
    }
}