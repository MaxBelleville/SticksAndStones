use bevy::{
    prelude::*, window::{CursorGrabMode, CursorOptions, PrimaryWindow}
};

use super::shared;

pub struct WindowManagerPlugin;

impl Plugin for WindowManagerPlugin {
    fn build(&self, app: &mut App) {
         app.insert_resource(shared::res::WindowManagerSettings::default());
         app.add_systems(Startup, setup_window);
         app.add_systems(OnEnter(shared::GameState::InGame), disable_cursor);
         app.add_systems(OnExit(shared::GameState::InGame), enable_cursor);
    }
}

fn disable_cursor(mut cursoropts: Query<(&mut CursorOptions, Option<&PrimaryWindow>)>) {
    if let Some((mut cursor, _)) = cursoropts.iter_mut().find(|(_, primary)| primary.is_some()) {
        cursor.grab_mode = CursorGrabMode::Confined;
        cursor.visible = false;
    }
}

fn enable_cursor(mut cursoropts: Query<(&mut CursorOptions, Option<&PrimaryWindow>)>) {
    if let Some((mut cursor, _)) = cursoropts.iter_mut().find(|(_, primary)| primary.is_some()) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn setup_window(
    mut windows: Query<(Entity, &mut Window, Option<&PrimaryWindow>)>,
    settings: Res<shared::res::WindowManagerSettings>,
) {
    if let Some((_entity, mut window, _)) = windows.iter_mut().find(|(_, _, primary)| primary.is_some()) {
        // Set the cursor options for the active window.
        window.mode = settings.window_mode;
        window.present_mode = settings.present_mode;

        
        window.resolution.set(settings.dim.z*settings.display_factor, settings.dim.w*settings.display_factor);
        window.position.center(MonitorSelection::Current);
      
    }
    else {
        warn!("No primary window found; Flycam will not be able to set cursor options.");
    }
}
