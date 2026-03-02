use bevy::{prelude::*, render::RenderApp};

pub struct RaymarchPlugin;

impl Plugin for RaymarchPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
    }
}
