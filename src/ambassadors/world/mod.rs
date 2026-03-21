
use bevy::{app::PluginGroupBuilder, prelude::*};

mod brickhub;
mod baseplate;
use sns_assets::{AssetRef, Libraries};
use super::load::LoadState;
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
        app.add_systems(OnEnter(LoadState::LoadAssets), init_world_assets);
    }
}

fn init_world_assets(mut commands: Commands) {
    let root = std::env::current_dir().unwrap();
    let data_libs = sns_assets::load_libraies(&root.join("assets/data"));
    let seed = 123;
    info!("curve {:?}", data_libs.registry().curve.lookup_asset(&AssetRef::from("enviroment:fog".to_string())).unwrap().values());
    commands.insert_resource(WorldData { data_libs, seed });
    
     info!("Assets loaded");
}