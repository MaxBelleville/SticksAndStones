use std::{any::TypeId, hash::Hash};
use std::sync::Arc;

use serde::{Deserialize, Deserializer};
use std::fs;
use std::path::{Path, PathBuf};
use jsonschema::Validator;
use serde_json::Value;

mod enviroment;
mod worldshared;
mod registry;
mod baseplates;
mod metadata;
mod validators;

pub mod prelude {
    pub use crate::enviroment::*;
    pub use crate::worldshared::*;
    pub use crate::baseplates::*;
    pub use crate::registry::*;
    pub use crate::metadata::*;
}

pub trait DefinedAsset: Sized + 'static {
    type ID: Eq + Hash + From<u32> + Into<u32> + Copy;
    const TYPE: &'static str;
    fn load_asset(
        registries: &mut registry::RegistrySystem,
        value: serde_json::Value,
    )->Result<(),String>;
    fn dependencies() -> Vec<TypeId>;
    fn meta() -> metadata::Meta;
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
  

pub trait IterableRef {
    type Item;
    fn iter(self) -> std::vec::IntoIter<(&'static str, Self::Item)>;
    fn iter_val(self) -> std::vec::IntoIter<Self::Item>;
}

pub trait IterableMut {
    type Item;
    fn iter_mut(self) -> std::vec::IntoIter<(&'static str, Self::Item)>;
    fn iter_mut_val(self) -> std::vec::IntoIter<Self::Item>;
}

#[derive(Default)]
pub struct Libraries {
    libraries: Vec<Library>,
    registry: registry::RegistrySystem,
}
impl Libraries {
    pub fn get_registry(&self) -> &registry::RegistrySystem {
        &self.registry
    }
    pub fn get_libraries(&self) -> &Vec<Library> {
        &self.libraries
    }
}

pub struct Library {
    root: PathBuf,
    manifest: LibraryManifest,
}
impl Library {
    pub fn new(root: &Path, manifest: LibraryManifest) -> Self {
        Self {
            root: root.to_path_buf(),
            manifest,
        }
    }
    pub fn get_manifest(&self) -> &LibraryManifest {
        &self.manifest
    }
    pub fn get_root(&self) -> &Path {
        &self.root
    }
}

#[derive(Deserialize)]
pub struct LibraryManifest {
    namespace: Option<String>,

    version: Option<f32>,

    #[serde(default)]
    libraries: Vec<String>,

    #[serde(default)]
    assets: Vec<AssetContainer>,

    #[serde(default)]
   settings: Option<serde_json::Value>,
}
#[derive(Default,Deserialize)]
pub struct AssetContainer {
    asset_type: String,
    schema: String,

    #[serde(default)]
    files: Vec<String>,

    #[serde(default)]
    folders: Vec<String>,
}


pub fn load_libraies(root: &Path)->Libraries {
    let schema_path = root.join("lib.schema.json");
    let schema_validator = validators::build_validator(&schema_path);
    let mut libraries = Vec::new();
    load_library_recursive(&root, &schema_validator, &mut libraries).unwrap();

    let mut registry = registry::RegistrySystem::default();
    load_assets(&libraries, &mut registry);
    Libraries { libraries: libraries, registry: registry }
}
///depth-first order libraries (child first)
pub fn load_library_recursive(
    path: &Path,
    schema_validator: &Validator,
    libraries: &mut Vec<Library>,
) -> Result<(), String> {
    let lib_path = path.join("lib.json");

    let Ok(text) = fs::read_to_string(lib_path) else {
        return Err("Could not read file".into());
    };
    let json: Value = serde_json::from_str(&text)
        .map_err(|_| "Could not parse json")?;

    if !schema_validator.is_valid(&json) {
        return Err("Invalid json schema".into());
    }

    let manifest: LibraryManifest = serde_json::from_value(json)
        .map_err(|_| "Invalid manifest structure")?;
    
    let mut library = Library::new(path, manifest);
    
    let namespace = path
    .file_name()
    .and_then(|n| n.to_str())
    .ok_or("Invalid library path")?;

    if library.manifest.namespace.is_none() {
        library.manifest.namespace = Some(namespace.to_string());
    }

    for lib in &library.manifest.libraries {

        let sub_path = path.join(lib);

        load_library_recursive(&sub_path, schema_validator, libraries)?;
    }

    libraries.push(library);

    Ok(())
}

pub fn load_assets(libraries: &Vec<Library>, registry: &mut registry::RegistrySystem) {
    for library in libraries {
        let root = &library.root;
        for container in &library.manifest.assets {  
            let schema_file = root.join(&container.schema);

            let mut asset_files = Vec::new();
            for files in &container.files {
                let file_path = root.join(files);
                asset_files.push(file_path);
            }

            for folder in &container.folders {
                let folder_path = root.join(folder);
                asset_files.append(&mut collect_folder_assets(folder_path));
            }

            let asset_schema_validator = validators::build_validator(&schema_file);

            for file in asset_files {
                 let Ok(text) = fs::read_to_string(&file) else {
                    continue;
                };

                let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };

                if let Err(e) = registry.load_by_type(
                    &container.asset_type,
                    &asset_schema_validator,
                    json
                ) {
                    eprintln!("Failed to load {:?}: {}", file, e);
                }
                            
            }

        }
    }
}


pub fn collect_folder_assets(folder: PathBuf) -> Vec<PathBuf> {
    walkdir::WalkDir::new(folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("json".as_ref()))
        .map(|e| e.into_path())
        .collect()
}
