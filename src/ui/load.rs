use bevy::prelude::*;
use bevy_lunex::prelude::*;

use super::UICanvas;
use super::shared;

// Tag component used to tag entities added on the splash screen
#[derive(Component)]
struct LoadScreen;

pub struct LoadPlugin;

// This plugin will display a splash screen with Bevy logo for 1 second before switching to the menu
impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app
            // When entering the state, spawn everything needed for this screen
            .add_systems(OnEnter(shared::GameState::StartGame), load_setup)
            // While in this state, run the `countdown` system
            .add_systems(
                Update,
                progress.run_if(in_state(shared::GameState::StartGame)),
            )
            // When exiting the state, despawn everything that was spawned for this screen
            .add_systems(
                OnExit(shared::GameState::StartGame),
                shared::despawn::<LoadScreen>,
            );
    }
}

fn load_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    container_query: Query<Entity, With<UICanvas>>,
) {
    if let Ok(container) = container_query.single() {
        commands.entity(container).with_children(|ui| {
            ui.spawn((
                LoadScreen,
                // You can name the entity

                // Specify the position and size of the button
                UiLayout::window()
                    .anchor(Anchor::CENTER) // Put the origin at the center
                    .pos(Rl((50.0, 50.0))) // Set the position to 50%
                    .size((200.0, 200.0)) // Set the size to [200.0, 200.0]
                    .pack(),
                // Attach sprite to the node
                Sprite::from_image(asset_server.load("images/bevy.png")),
                // When hovered, it will request the cursor icon to be changed
                OnHoverSetCursor::new(SystemCursorIcon::Pointer),
                // Interactivity is done through observers, you can query anything here
            ));
        });
    }
}

fn progress() {}
