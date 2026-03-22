use std::any::{TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use sns_macros::register_assets;
use jsonschema::Validator;
use serde_json::Value;
use crate::baseplates;

use super::{enviroment,metadata, worldshared, DefinedAsset, AssetRef};


register_assets!(
    enviroment::Curve,
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
    fn intern(&mut self, asset_ref: AssetRef) -> ID {
        self.glossary.intern(asset_ref)
    }

    fn insert(&mut self, asset: Asset, asset_ref: AssetRef) -> ID {
        let id = self.glossary.intern(asset_ref.clone());

        if self.assets.contains(id) {
            panic!("Duplicate asset definition: {:?}", asset_ref);
        }

        self.assets.insert(id, asset);
        id
    }

    pub fn asset(&self, id: ID) -> Option<&Asset> {
        self.assets.get(id)
    }


    pub fn asset_mut(&mut self, id: ID) -> Option<&mut Asset> {
        self.assets.get_mut(id)
    }
    pub fn lookup_id(&self, asset_ref: &AssetRef) -> Option<&ID> {
        self.glossary.get(asset_ref)
    }
    pub fn lookup_asset(&self, asset_ref: &AssetRef) -> Option<&Asset> {
        if let Some(id) = self.glossary.get(asset_ref) {
            return self.assets.get(*id);
        }
        None
    }
    pub fn lookup_asset_mut(&mut self, asset_ref: &AssetRef) -> Option<&mut Asset> {
        if let Some(id) = self.glossary.get(asset_ref) {
            return self.assets.get_mut(*id);
        }
        None
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }
    pub fn assets(&self) -> &Vec<Option<Asset>> {
        self.assets.assets()
    }
    


    pub fn resolve(&self, id: ID) -> &AssetRef {
        self.glossary.resolve(id)
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
    pub fn assets(&self) -> &Vec<Option<Asset>> {
        &self.assets
    }
    pub fn len(&self) -> usize {
        self.assets.len()
    }
    
    pub fn contains(&self, id: ID) -> bool {
        self.assets
            .get(id.into() as usize)
            .map_or(false, |a| a.is_some())
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
    pub fn get_glossary(&self) -> &HashMap<AssetRef, ID> {
        &self.map
    }
    pub fn get_reverse(&self) -> &Vec<AssetRef> {
        &self.reverse
    }
    pub fn get_len(&self) -> usize {
        self.reverse.len()
    }

    pub fn resolve(&self, id: ID) -> &AssetRef {
        &self.reverse[id.into() as usize]
    }
}