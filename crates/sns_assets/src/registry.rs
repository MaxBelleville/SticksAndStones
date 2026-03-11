use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;

use sns_macros::register_assets;
use crate::baseplates;

use super::{enviroment, worldshared, DefinedAsset, AssetRef};


pub struct RegistrySystem {
    registries: HashMap<TypeId, Box<dyn AssetRegistryDyn>>,
}
impl RegistrySystem {
    pub fn get<A: DefinedAsset + 'static>(&self) -> Option<&AssetRegistry<A::ID, A>> {
        self.registries
            .get(&TypeId::of::<A>())
            .and_then(|r| r.as_any().downcast_ref())
    }

    pub fn get_mut<A: DefinedAsset + 'static>(&mut self) -> Option<&mut AssetRegistry<A::ID, A>> {
        self.registries
            .get_mut(&TypeId::of::<A>())
            .and_then(|r| r.as_any_mut().downcast_mut())
    }
    pub fn init_registry<A: DefinedAsset + 'static>(&mut self) {
        self.registries
            .insert(TypeId::of::<A>(), A::create_registry());
    }
}

register_assets!(
    enviroment::Curves,
    enviroment::Preset,
    enviroment::PresetSegement,
    worldshared::NoiseField,
    worldshared::BaseRule,
    worldshared::TerrainProfile,
    worldshared::DensityCurve,
    worldshared::ClimateField,
    baseplates::NoiseBasis,
    baseplates::RegionArchetype,
    baseplates::Biome,
    baseplates::SubBiome
);

#[derive(Default, Debug)]
pub struct AssetRegistry<ID, Asset> {
    assets: AssetStorage<ID, Asset>,
    glossary: AssetGlossary<ID>,
}
impl<ID: Eq + Hash + From<u32> + Into<u32> + Copy, Asset> AssetRegistry<ID, Asset> {
    pub fn intern(&mut self, asset_ref: AssetRef) -> ID {
        self.glossary.intern(asset_ref)
    }

    pub fn insert(&mut self, asset: Asset, asset_ref: AssetRef) -> ID {
        let id = self.glossary.intern(asset_ref.clone());

        if self.assets.contains_key(id) {
            panic!("Duplicate asset definition: {:?}", asset_ref);
        }

        self.assets.insert(id, asset);
        id
    }

    pub fn get_asset(&self, id: ID) -> Option<&Asset> {
        self.assets.get(id)
    }
    pub fn get_asset_mut(&mut self, id: ID) -> Option<&mut Asset> {
        self.assets.get_mut(id)
    }
    pub fn lookup(&self, asset_ref: &AssetRef) -> Option<&ID> {
        self.glossary.get(asset_ref)
    }
    pub fn resolve(&self, id: ID) -> &AssetRef {
        self.glossary.resolve(id)
    }
}

pub trait AssetRegistryDyn: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<ID, Asset> AssetRegistryDyn for AssetRegistry<ID, Asset>
where
    ID: 'static,
    Asset: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default, Debug)]
pub struct AssetStorage<ID, Asset> {
    assets: Vec<Option<Asset>>,
    _marker: std::marker::PhantomData<ID>,
}
impl<ID: Into<u32> + From<u32> + Copy, Asset> AssetStorage<ID, Asset> {
    pub fn insert(&mut self, id: ID, asset: Asset) {
        let idx = id.into() as usize;

        if self.assets.len() <= idx {
            self.assets.resize_with(idx + 1, || None);
        }

        if self.assets[idx].is_some() {
            panic!("Duplicate asset id");
        }

        self.assets[idx] = Some(asset);
    }
    pub fn get(&self, id: ID) -> Option<&Asset> {
        self.assets.get(id.into() as usize).and_then(|a| a.as_ref())
    }
    pub fn get_mut(&mut self, id: ID) -> Option<&mut Asset> {
        self.assets
            .get_mut(id.into() as usize)
            .and_then(|a| a.as_mut())
    }
    pub fn contains_key(&self, id: ID) -> bool {
        self.assets.get(id.into() as usize).is_some()
    }
}

#[derive(Default, Debug)]
pub struct AssetGlossary<ID> {
    map: HashMap<AssetRef, ID>,
    reverse: Vec<AssetRef>,
}

impl<ID: From<u32> + Into<u32> + Copy> AssetGlossary<ID> {
    pub fn intern(&mut self, asset: AssetRef) -> ID {
        if let Some(id) = self.map.get(&asset) {
            return *id;
        }

        let id = ID::from(self.reverse.len() as u32);

        self.map.insert(asset.clone(), id);
        self.reverse.push(asset);

        id
    }
    pub fn get(&self, asset: &AssetRef) -> Option<&ID> {
        self.map.get(asset)
    }
    pub fn resolve(&self, id: ID) -> &AssetRef {
        &self.reverse[id.into() as usize]
    }
}