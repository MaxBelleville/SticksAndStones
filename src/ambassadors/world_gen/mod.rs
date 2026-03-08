
use bevy::{app::PluginGroupBuilder, prelude::*};

mod brickhub;
mod baseplate;

pub struct WorldGenPlugins;
impl PluginGroup for WorldGenPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
        .add(WorldGenPlugin)
        .add(brickhub::BrickhubPlugin)
        .add(baseplate::BaseplatePlugin)
    }
}

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
    }
}