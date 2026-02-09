use bevy::{app::PluginGroupBuilder, prelude::*};
pub struct UIGamePlugins;


impl PluginGroup for UIGamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(UiGamePlugin)
    }
}