

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_atmosphere::plugin::AtmospherePlugin;
use bevy_rand::global::GlobalEntropy;
use bevy_rand::prelude::ChaCha12Rng;
use bevy_rand::traits::ForkableSeed;
use bevy_rand::traits::SeedSource;

use super::shared;
use super::contree;
use super::flycam;
use super::compute_system;

mod enviroment;
mod features;
mod chunks;
mod events;

pub struct WorldBuilderPlugins;

impl PluginGroup for WorldBuilderPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(WorldBuilderPlugin)
            .add(flycam::FlycamPlugin)
            .add( AtmospherePlugin)
            .add( MaterialPlugin::<compute_system::ComputeRenderMaterial>::default())
            .add(compute_system::ComputeSystemPlugin)
            .add(enviroment::EnviromentPlugin)
            .add(chunks::ChunkManagerPlugin)
            .add(events::handler::EventHandlerPlugin)
    }
}

struct WorldBuilderPlugin;

impl Plugin for WorldBuilderPlugin {
    fn build(&self, app: &mut App) {   
        app.insert_resource(WorldBuilderResource::default()).
        add_systems(Startup, setup);
    }
}

fn setup(mut rng: GlobalEntropy<ChaCha12Rng>, mut state: ResMut<WorldBuilderResource>) {
    let binding = rng.fork_seed();
    let seed: &[u8; 32] = binding.get_seed();
    state.noise_seed = shared::maths::get_noise_seed(seed);
}

#[derive(Resource,Default)]
pub struct WorldBuilderResource {
    pub noise_seed: Vec3,
    pub contree: contree::Contree,
    pub gpu_data: Vec<i32>,
    pub file_data: Vec<u8>,
}