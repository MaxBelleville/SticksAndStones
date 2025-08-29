use bevy::{app::PluginGroupBuilder, prelude::*};

use super::shared;
use super::contree::chunk;

mod chunks;

mod features;

pub struct WorldBuilderPlugins;

impl PluginGroup for WorldBuilderPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(WorldBuilderPlugin)
            .add(chunks::ChunkManagerPlugin)
    }
}

struct WorldBuilderPlugin;

impl Plugin for WorldBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
                OnExit(shared::GameState::InMenu),
                setup,
            );
    }
}

fn setup(
    mut state: ResMut<shared::res::WorldData>
) {
    (state.file_data, state.gpu_data) = state.contree.compress_data();

    state.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
    state.root_size = state.contree.get_root_size() as f32;

}
