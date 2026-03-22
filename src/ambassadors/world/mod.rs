
use bevy::{app::PluginGroupBuilder, prelude::*};

mod brickhub;
mod baseplate;
use sns_assets::{AssetRef, Libraries, Library, registry::RegistrySystem};
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
impl WorldData {
    pub fn get_registry(&self) -> &RegistrySystem {
        &self.data_libs.registry()
    }
    pub fn get_libraies(&self) -> &Vec<Library> {
        &self.data_libs.libraries()
    }
    pub fn get_seed(&self) -> u32 {
        self.seed
    }
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
    let id = data_libs.registry().preset_segement.lookup_asset(&AssetRef::from("enviroment:night_to_dawn".to_string())).unwrap().curve_id();
    let curve = data_libs.registry().curve.asset(*id).unwrap();

    info!("curve {:?}", curve.values());
    commands.insert_resource(WorldData { data_libs, seed });
    
     info!("Assets loaded");
}