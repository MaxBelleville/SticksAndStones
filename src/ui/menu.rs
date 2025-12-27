use bevy::prelude::*;
use bevy_lunex::prelude::*;


use super::shared;
use super::{UICanvas,MenuState,PauseBackground,SettingsBackground};

#[derive(Component)]
struct MainMenu;



pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MenuState>()
            .add_systems(OnEnter(MenuState::Main), (shared::despawn::<SettingsBackground>,main_menu_setup))
            .add_systems(OnExit(MenuState::Main), (shared::despawn::<MainMenu>,shared::despawn::<PauseBackground>));
    }
}


fn main_menu_setup(
    mut commands: Commands,
    game_state: Res<State<shared::GameState>>,
    asset_server: Res<AssetServer>,
    container_query: Query<Entity, With<UICanvas>>,
) {

    let buttons = if game_state.get() == &shared::GameState::Paused {
        Vec::from([
            "Continue",
            "Load Game",
            "Save Game",
            "Settings",
            "Quit Game",
        ])
    }
    else {
        Vec::from([
            "New Game",
            "Load Game",
            "Settings",
            "Credits",
            "Quit Game"
        ])
    };

   if let Ok(container) = container_query.single() {

        commands.entity(container).with_children(|ui| {

                if game_state.get() == &shared::GameState::Paused {
                    ui.spawn((
                        PauseBackground,
                        // You can name your entites for easier debug
                        Name::new("Background"),
                        UiLayout::solid()
                            .size((640.0, 360.0))
                            .scaling(Scaling::Fill)
                            .pack(),
                        Sprite::from_image(asset_server.load("images/menu/pause_bg.png")),
                        
                    ));
                }

                ui.spawn((
                    MainMenu,
                    UiLayout::solid().size((881.0, 1600.0)).align_x(-0.90).pack(),
                )).with_children(|ui| {
            

                //Logo 
                ui.spawn((
                    Name::new("Logo"),
                    UiLayout::solid().size((260.0, 33.0)).align_y(-0.85).pack(),
                    Sprite::from_image(asset_server.load("images/menu/title.png")),
                ));

                // Spawn buttons
                let gap = 32.0;
                let size = 52.0;
                let mut offset = 192.0;
                for button in buttons {
                    // Spawn the button
                    let mut button_entity = ui.spawn((
                        Name::new(button),
                        UiLayout::window()
                            .pos((0.0,offset))
                            .size((300.0,size))
                            .pack(),
                        Visibility::Visible,
                        OnHoverSetCursor::new(bevy::window::SystemCursorIcon::Pointer),
                    ));
                    button_entity.with_children(|ui| {
                        // Spawn the image
                        ui.spawn((
                            // You can define layouts for multiple states
                            UiLayout::new(vec![
                                (UiBase::id(), UiLayout::window().full()),
                                (UiHover::id(), UiLayout::window().x(Rl(10.0)).full()),
                            ]),
                            // Like this you can enable a state
                            UiHover::new().forward_speed(8.0).backward_speed(10.0),
                        // You can specify colors for multiple states
                            Sprite {
                                image: asset_server.load("images/menu/button.png"),
                                ..default()
                            },
                            // Make sure it does not cover the bounding zone of parent
                            Pickable::IGNORE,
                        ))
                        .with_children(|ui| {
                            // Spawn the text
                            ui.spawn((
                                // For text always use window layout to position it
                                UiLayout::window()
                                    .pos((Rh(75.0), Rl(48.0)))
                                    .anchor(Anchor::CENTER_LEFT)
                                    .pack(),
                                     
                                UiColor::new(vec![
                                    (UiBase::id(), Color::BLACK),
                                    (UiHover::id(), Color::srgba(15.0/255.0,42.0/255.0,63.0/255.0,1.0)),
                                ]),
                           
                                // You can control the size of the text
                                UiTextSize::from(Rh(120.0)),
                                // You can attach text like this
                                Text2d::new(button.to_ascii_uppercase()),
                                TextFont {
                                    font: asset_server.load("fonts/monogram.ttf"),
                                    font_size: 72.0,
                                    ..default()
                                },
                                // Make sure it does not cover the bounding zone of parent
                                Pickable::IGNORE,
                            ));
                            // Spawn the text
                            ui.spawn((
                                // For text always use window layout to position it
                                UiLayout::window()
                                    .pos((Rh(75.0), Rl(42.0)))
                                    .anchor(Anchor::CENTER_LEFT)
                                    .pack(),
                                UiColor::new(vec![
                                    (UiBase::id(), Color::srgba(246.0/255.0, 214.0/255.0, 189.0/255.0,1.0)),
                                    (UiHover::id(), Color::WHITE),
                                ]),
                            UiHover::new().forward_speed(8.0).backward_speed(10.0),
                                // You can control the size of the text
                                UiTextSize::from(Rh(120.0)),
                                // You can attach text like this
                                Text2d::new(button.to_ascii_uppercase()),
                                TextFont {
                                    font: asset_server.load("fonts/monogram.ttf"),
                                    font_size: 72.0,
                                    ..default()
                                },
                                // Make sure it does not cover the bounding zone of parent
                                Pickable::IGNORE,
                            ));
                        });
                    }).observe(hover_set::<Pointer<Over>, true>).observe(hover_set::<Pointer<Out>, false>);

                    match button {
                            "New Game" => {
                                button_entity.observe(|_: On<Pointer<Click>>, mut next: ResMut<NextState<shared::GameState>>, 
                                    mut menu_next: ResMut<NextState<MenuState>>| {
                                    // Change the state to settings
                                    next.set(shared::GameState::StartGame);
                                    menu_next.set(MenuState::Disabled);
                                });
                            },
                            "Settings" => {
                                button_entity.observe(|_: On<Pointer<Click>>, mut next: ResMut<NextState<MenuState>>| {
                                    // Change the state to settings
                                    next.set(MenuState::Settings);
                                });
                            },
                            "Quit Game" => {
                                button_entity.observe(|_: On<Pointer<Click>>, mut exit: MessageWriter<AppExit>| {
                                    // Close the app
                                    exit.write(AppExit::Success);
                                });
                            },
                            _ => {
                                button_entity.observe(|c_trigger: On<Pointer<Click>>, c_button: Query<NameOrEntity, With<UiLayout>>| {
                                    info!("Clicked: {}", c_button.get(c_trigger.event_target()).unwrap());
                                });
                            }
                            }


                    offset += gap + size;
                }
            });
                
        });
    }
}
