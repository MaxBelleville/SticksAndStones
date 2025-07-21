use bevy::{
    prelude::*, window::{CursorGrabMode, PrimaryWindow, WindowMode}, winit::WinitWindows
};

use super::shared;

pub struct WindowManagerPlugin;

impl Plugin for WindowManagerPlugin {
    fn build(&self, app: &mut App) {
         app.insert_resource(shared::res::WindowManagerSettings::default());
         app.add_systems(Startup, setup_window);
    }
}

fn setup_window(
    winit_windows: NonSend<WinitWindows>,
    mut windows: Query<(Entity, &mut Window, Option<&PrimaryWindow>)>,
    mut settings: ResMut<shared::res::WindowManagerSettings>,
) {
    if let Some((entity, mut window, _)) = windows.iter_mut().find(|(_, _, primary)| primary.is_some()) {
        // Set the cursor options for the active window.
        window.cursor_options.grab_mode = CursorGrabMode::Confined;
        window.cursor_options.visible = false;
        window.mode = settings.window_mode;
        
        window.present_mode = settings.present_mode;
        window.cursor_options.grab_mode = CursorGrabMode::Confined;
        window.cursor_options.visible = false;
        if settings.window_mode != WindowMode::Windowed {
            if let Some(monitor)  = winit_windows.get_window(entity).and_then(|winit_window| winit_window.current_monitor()) 
            {
                let (width, height) =  (monitor.size().width / settings.display_factor, monitor.size().height / settings.display_factor);
                settings.dim = Vec4::new(monitor.position().x as f32, monitor.position().y as f32, width as f32, height as f32);
            }
        }
        
        window.resolution.set(settings.dim.z*settings.display_factor as f32, settings.dim.w*settings.display_factor as f32);
        window.position.center(MonitorSelection::Current);
      
    }
    else {
        warn!("No primary window found; Flycam will not be able to set cursor options.");
    }
}
