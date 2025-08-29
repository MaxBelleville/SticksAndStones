
use bevy::prelude::*;
use bevy_lunex::prelude::*;

use super::{MenuState,UICanvas};
use super::shared;

// Tag component used to tag entities added on the splash screen
#[derive(Component)]
struct OnSplashScreen;

// Newtype to use a `Timer` for this screen as a resource
#[derive(Resource, Deref, DerefMut)]
struct SplashTimer(Timer);

pub struct SplashPlugin;
// This plugin will display a splash screen with Bevy logo for 1 second before switching to the menu
impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        // As this plugin is managing the splash screen, it will focus on the state `GameState::Splash`
        app
            // When entering the state, spawn everything needed for this screen
            .add_systems(OnEnter(shared::GameState::Splash), splash_setup)
            // While in this state, run the `countdown` system
            .add_systems(
                Update,
                countdown.run_if(in_state(shared::GameState::Splash)),
            )
            // When exiting the state, despawn everything that was spawned for this screen
            .add_systems(
                OnExit(shared::GameState::Splash),
                shared::despawn::<OnSplashScreen>,
            );
    }
}

fn splash_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    container_query: Query<Entity, With<UICanvas>>,
) {
   if let Ok(container) = container_query.single() {

        commands.entity(container).with_children(|ui| {
            ui.spawn((
                // You can name the entity
                OnSplashScreen,
                // Specify the position and size of the button
                UiLayout::window()
                    .anchor(Anchor::Center) // Put the origin at the center
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
 
    // Insert the timer as a resource
   }

    //Probably panic on else
    commands.insert_resource(SplashTimer(Timer::from_seconds(1.0, TimerMode::Once)));

}

// Tick the timer, and change state when finished
fn countdown(
    mut game_state: ResMut<NextState<shared::GameState>>,
    mut menu_state: ResMut<NextState<MenuState>>,
    time: Res<Time>,
    mut timer: ResMut<SplashTimer>,
) {
    if timer.tick(time.delta()).finished() {
        game_state.set(shared::GameState::InMenu);
        menu_state.set(MenuState::Main);
    }
}
