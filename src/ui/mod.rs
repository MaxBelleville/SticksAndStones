
use bevy::{app::PluginGroupBuilder, prelude::*};
mod pause;
mod settings;
mod menu;
mod splash;

use super::shared;
//Setup plugin group and menu resources
pub struct UIPlugin;

impl PluginGroup for UIPlugin {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(SplashPlugin)
        .add(MenuPlugin)
        .add(SettingsPlugin)
        .add(PausePlugin)
    }
}
