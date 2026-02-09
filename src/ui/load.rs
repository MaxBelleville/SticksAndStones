use crate::ui;
use bevy::prelude::*;
use iyes_progress::prelude::*;
pub struct LoadPlugin;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::ManagerState::LoadingGame), start_game_setup)
            .add_systems(
                Update,
                (
                    mouse_clicks.track_progress_and_stop::<ui::ManagerState>(),
                    update_progress_entity,
                    update_progress_bar,
                )
                    .run_if(in_state(ui::ManagerState::LoadingGame)),
            );
    }
}

fn mouse_clicks(input: Res<ButtonInput<MouseButton>>, mut n_clicks: Local<u32>) -> Progress {
    const MAX_CLICKS: u32 = 3;

    if input.just_pressed(MouseButton::Left) {
        *n_clicks += 1;
    }

    Progress {
        done: *n_clicks,
        total: MAX_CLICKS,
    }
}

#[derive(Component)]
struct OnLoading;

#[derive(Component)]
struct StartGameProgress;

#[derive(Component)]
struct ProgressBarInner;

#[derive(Component)]
struct ProgressBarText;

fn start_game_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let main_bg = asset_server.load("images/menu/main_bg.png");
    commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::ManagerState::LoadingGame,
            OnLoading,
            JustifyContent::Center,
            AlignItems::Center,
        ))
        .with_children(|parent| {
            parent.spawn((
                StartGameProgress,
                ProgressEntity::<ui::ManagerState>::default()
                    // give it some initial values
                    .with_progress(0, 1),
            ));

            parent.spawn((
                BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                BorderColor::all(Color::srgb(0.0, 0.0, 0.0)),
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(30.0),
                    height: percent(5.0),
                    border: UiRect::all(px(1.0)),
                    ..Default::default()
                },
                children![(
                    BackgroundColor(Color::srgb(0.5, 0.75, 0.5)),
                    BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
                    Node {
                        height: Val::Percent(100.0),
                        width: Val::Percent(0.0),
                        padding: UiRect::left(Val::Px(16.0)),
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..Default::default()
                    },
                    ProgressBarInner,
                    children![(
                        Text::new("0/0".to_owned()),
                        TextColor(Color::BLACK),
                        ProgressBarText,
                    )]
                )],
            ));
        });
}

fn update_progress_entity(
    mut q: Query<&mut ProgressEntity<ui::ManagerState>, With<StartGameProgress>>,
) {
    let Ok(mut p) = q.single_mut() else {
        return;
    };
    p.visible.done = 1;
}

fn update_progress_bar(
    // Here is where the progress is stored
    pt: Res<ProgressTracker<ui::ManagerState>>,
    // our UI elements
    mut q_bar_inner: Query<&mut Node, With<ProgressBarInner>>,
    mut q_bar_text: Query<&mut Text, With<ProgressBarText>>,
) {
    // get the progress info (visible progress)
    let progress = pt.get_global_progress();
    let ratio: f32 = progress.into();

    for mut text in q_bar_text.iter_mut() {
        *text = Text::new(format!("{}/{}", progress.done, progress.total));
    }
    for mut node in q_bar_inner.iter_mut() {
        node.width = Val::Percent(ratio * 100.0);
    }
}
