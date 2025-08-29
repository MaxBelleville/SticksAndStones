
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy_lunex::prelude::*;

use super::shared;
use super::UICanvas;
// Tag component used to tag entities added on the settings menu screen
#[derive(Component)]
struct PlayerUI;

#[derive(Component)]
struct PlayerPosTex;

pub struct PlayerUIPlugin;
impl Plugin for PlayerUIPlugin {
    fn build(&self, app: &mut App) {
        app .add_systems(OnEnter(shared::GameState::InGame), player_ui_setup)
            .add_systems(
                OnExit(shared::GameState::InGame),
                shared::despawn::<PlayerPosTex>,
            ).add_systems(Update, update_player_pos.run_if(in_state(shared::GameState::InGame)));
    }
}

fn player_ui_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    container_query: Query<Entity, With<UICanvas>>,
) {
    if let Ok(container) = container_query.single() {
        commands.entity(container).with_children(|ui| {
            ui.spawn((
                // Position the text using the window layout's position and anchor
                UiLayout::window().anchor(Anchor::TopLeft).pack(),
                // This controls the height of the text, so 60% of the parent's node height
                UiTextSize::from(Rh(5.0)),
                // You can attach text like this
                Text2d::new("Button"),
                PlayerPosTex,
                // Font size now works as "text resolution"
                TextFont {
                    font: asset_server.load("fonts/m6x11.ttf"),
                    font_size: 64.0,
                    ..Default::default()
                },
            ));
        });
    }
}

fn update_player_pos(
    diagnostics: Res<DiagnosticsStore>,
    cam_query: Query<&Transform, With<Camera3d>>,
    mut text_query: Query<&mut Text2d, With<PlayerPosTex>>,
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