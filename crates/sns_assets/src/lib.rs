use std::{any::TypeId, hash::Hash};
use std::sync::Arc;

use serde::{Deserialize, Deserializer};

mod enviroment;
mod world_gen;
mod registry;
mod schema;

pub mod prelude {
    pub use crate::enviroment::*;
    pub use crate::world_gen::*;
}
pub trait DefinedAsset: Sized + 'static {
    type ID;
    const TYPE: &'static str;
    fn create_registry() -> Box<dyn registry::AssetRegistryDyn>;
    fn load_asset(
        registries: &mut registry::RegistrySystem,
        value: serde_json::Value,
    );
    fn dependencies() -> Vec<TypeId>;
    fn schema() -> schema::Schema;
    fn visit_asset_refs<F:  FnMut(TypeId, u32)>(&self, visitor: F);
}


#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct AssetRef {
    pub namespace: Arc<str>,
    pub name: Arc<str>,
}
impl From<String> for AssetRef {
    fn from(s: String) -> Self {
        let mut parts = s.split(':');
        Self {
            namespace: parts.next().unwrap_or("").into(),
            name: parts.next().unwrap_or("").into(),
        }
    }
}
impl Into<String> for AssetRef {
    fn into(self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
}
impl<'de> Deserialize<'de> for AssetRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut parts = s.split(':');
        let namespace = parts
            .next()
            .ok_or_else(|| serde::de::Error::custom("missing namespace"))?;
        let name = parts
            .next()
            .ok_or_else(|| serde::de::Error::custom("missing name"))?;
        Ok(AssetRef {
            namespace: namespace.into(),
            name: name.into(),
        })
    }
}
