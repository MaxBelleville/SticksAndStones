use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_atmosphere::plugin::AtmospherePlugin;
use bevy_rand::global::GlobalEntropy;
use bevy_rand::prelude::ChaCha12Rng;
use bevy_rand::traits::ForkableSeed;
use bevy_rand::traits::SeedSource;

use crate::shared::res::SVTPushConsts;

use super::compute_system;
use super::flycam;
use super::shared;

mod chunks;
mod enviroment;
mod events;
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
            .add(events::handler::EventHandlerPlugin)
            .add(compute_system::ComputeSystemPlugin)
    }
}

struct WorldBuilderPlugin;

impl Plugin for WorldBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SVTPushConsts::default())
            .insert_resource(shared::res::WorldBuilderResource::default())
            .add_systems(
                OnEnter(shared::GameState::InGame),
                setup.run_if(not(resource_exists::<shared::res::IsGamePrepared>)),
            );
    }
}

fn setup(
    mut rng: GlobalEntropy<ChaCha12Rng>,
    mut state: ResMut<shared::res::WorldBuilderResource>,
    mut push: ResMut<SVTPushConsts>,
) {
    let binding = rng.fork_seed();
    let seed: &[u8; 32] = binding.get_seed();
    state.noise_seed = shared::maths::get_noise_seed(seed);
    for z in 0..100 {
        for y in 0..100 {
            for x in 0..100 {
                state.contree.insert(ivec3(x, y, z), Some(1), None);
            }
        }
    }
  //  state.contree.print_root();
    (state.file_data, state.gpu_data) = state.contree.compress_data();
    println!("root_size: {}", state.contree.get_root_size());
    
    println!("root_min: {:?}", state.contree.get_root_min());
    println!("offset: {:?}", state.contree.get_offset());
    push.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
    push.root_size = state.contree.get_root_size() as f32;
}
