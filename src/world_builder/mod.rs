use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_atmosphere::plugin::AtmospherePlugin;


use super::compute_system;
use super::flycam;
use super::shared;
use super::contree::chunk;

mod chunks;
mod enviroment;
mod features;

pub struct WorldBuilderPlugins;

impl PluginGroup for WorldBuilderPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(WorldBuilderPlugin)
            .add(flycam::FlycamPlugin)
            .add(AtmospherePlugin)
            .add(MaterialPlugin::<compute_system::ComputeRenderMaterial>::default())
            .add(enviroment::EnviromentPlugin)
            .add(chunks::ChunkManagerPlugin)
            .add(compute_system::ComputeSystemPlugin)
    }
}

struct WorldBuilderPlugin;

impl Plugin for WorldBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(shared::res::SVTPushConsts::default())
            .insert_resource(shared::res::WorldBuilderResource::default())
            .add_systems(
                OnExit(shared::GameState::InMenu),
                setup.run_if(not(resource_exists::<shared::res::IsGamePrepared>)),
            );
    }
}

fn setup(
    mut state: ResMut<shared::res::WorldBuilderResource>,
    mut push: ResMut<shared::res::SVTPushConsts>,
) {
    (state.file_data, state.gpu_data) = state.contree.compress_data();

    push.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
    push.root_size = state.contree.get_root_size() as f32;

}
