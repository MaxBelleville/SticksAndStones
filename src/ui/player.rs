use std::fmt::Write;

use bevy::diagnostic::Diagnostics;
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use super::ButtonColors;
use super::DisplayQuality;
use super::MenuButtonAction;
use super::MenuState;
use super::SelectedOption;
use super::TEXT_COLOR;
use super::Volume;
use super::shared;

// Tag component used to tag entities added on the settings menu screen
#[derive(Component)]
struct OnPlayerUIScreen;

#[derive(Component)]
struct PlayerPosTex;

pub struct PlayerUIPlugin;
impl Plugin for PlayerUIPlugin {
    fn build(&self, app: &mut App) {
        app .add_systems(OnEnter(shared::GameState::InGame), player_ui_setup)
            .add_systems(
                OnExit(shared::GameState::InGame),
                shared::despawn::<OnPlayerUIScreen>,
            ).add_systems(Update, update_player_pos.run_if(in_state(shared::GameState::InGame)));
    }
}

fn player_ui_setup(
    mut commands: Commands,
    
) {
      commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            ..default()
        },
        OnPlayerUIScreen,
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            children![
                // Display the game name
                (
                    Text::new("Sticks & Stones"),
                    TextColor(TEXT_COLOR),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    PlayerPosTex
                ),
            ]
        )]
      ));
}

fn update_player_pos(
    diagnostics: Res<DiagnosticsStore>,
    cam_query: Query<&Transform, With<Camera3d>>,
    mut text_query: Query<&mut Text, With<PlayerPosTex>>,
) {
    if let Ok(cam_transform) = cam_query.single() {
        let fps = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps_diag| fps_diag.smoothed())
            .unwrap_or(0.0);

        let pos = cam_transform.translation;
        if let Ok(mut text) = text_query.single_mut() {
            text.0 = format!("Camera: ({:.2}, {:.2}, {:.2} FPS: {:.0})", pos.x, pos.y, pos.z, fps);
        }
    }
}