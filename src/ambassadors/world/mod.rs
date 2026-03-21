
use bevy::{app::PluginGroupBuilder, prelude::*};

mod brickhub;
mod baseplate;
use sns_assets::Libraries;

pub struct WorldGenPlugins;
impl PluginGroup for WorldGenPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
        .add(WorldGenPlugin)
        .add(brickhub::BrickhubPlugin)
        .add(baseplate::BaseplatePlugin)
    }
}
#[derive(Resource)]
pub struct WorldData {
    data_libs: Libraries,
    seed: u32,
}

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
    }
}