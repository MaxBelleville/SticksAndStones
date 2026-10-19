use std::fmt::Display;
use std::sync::Arc;
use std::{any::TypeId, hash::Hash};

use bevy_log::info;
use jsonschema::Validator;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use sns_macros::MakeDefinedAsset;
use std::fs;
use std::path::{Path, PathBuf};

pub mod baseplates;
pub mod environment;
pub mod metadata;
pub mod registry;
mod validators;
pub mod worldshared;

pub trait DefinedAsset: Sized {
    type ID: Ord + Eq + Hash + From<u32> + Into<u32> + Copy;
    const TYPE: &'static str;
    const NAMESPACE: &'static str;

    fn load_definition(
        registries: &mut registry::DefinedRegistrySystem,
        value: serde_json::Value,
    ) -> Result<(), String>;
    fn current_ref() -> AssetRef;
}

#[derive(Default, MakeDefinedAsset)]
#[asset(namespace = "core", type = "enums")]
pub struct EnumDefines {
    values: Vec<String>,
}

#[derive(Default, MakeDefinedAsset)]
#[asset(namespace = "core", type = "tags")]
pub struct TagDefines {
    values: Vec<String>,
}

pub trait StaticAsset: Sized + 'static {
    type ID: Ord + Eq + Hash + From<u32> + Into<u32> + Copy;
    const TYPE: &'static str;
    const NAMESPACE: &'static str;

    fn load_asset(
        registries: &mut registry::RegistrySystem,
        defs: &registry::DefinedRegistrySystem,
        value: serde_json::Value,
    ) -> Result<(), String>;
    fn dependencies() -> Vec<TypeId>;
    fn meta() -> metadata::Meta;
    fn current_ref() -> AssetRef;
    fn visit_asset_refs<F: FnMut(TypeId, u32)>(&self, visitor: F);
}

/// A reference to a defined enum with a specific bitflag value
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnumRef {
    /// Which defined enum this refers to (e.g., "rule_flag:region")
    pub id: EnumDefinesID, // Index into registry, OR...
    /// The bitflag value for this instance
    pub bits: u32,
}

impl EnumRef {
    pub fn new(id: EnumDefinesID, bits: u32) -> Self {
        Self { id, bits }
    }

    /// Check if a specific variant is set (by index)
    pub fn has_index(&self, index: u8) -> bool {
        self.bits & (1u32 << index) != 0
    }

    /// Get the raw bitflag value
    pub fn as_u32(&self) -> u32 {
        self.bits
    }

    /// Iterate all set bit indices
    pub fn iter_set_bits(&self) -> impl Iterator<Item = u8> {
        (0..32).filter(move |i| self.has_index(*i))
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, Default)]
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
impl Display for AssetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.name)
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

pub trait IterableRefMut {
    type Item;
    fn iter_mut(self) -> std::vec::IntoIter<(&'static str, Self::Item)>;
    fn iter_mut_val(self) -> std::vec::IntoIter<Self::Item>;
}

#[derive(Default)]
pub struct Libraries {
    libraries: Vec<Library>,
    registry: registry::RegistrySystem,
    defs: registry::DefinedRegistrySystem,
}
impl Libraries {
    pub fn registry(&self) -> &registry::RegistrySystem {
        &self.registry
    }
    pub fn libraries(&self) -> &Vec<Library> {
        &self.libraries
    }
    pub fn defs(&self) -> &registry::DefinedRegistrySystem {
        &self.defs
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
    pub fn manifest(&self) -> &LibraryManifest {
        &self.manifest
    }
    pub fn root(&self) -> &Path {
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
    defined_assets: Vec<DefinedAssetContainer>,

    #[serde(default)]
    settings: Option<serde_json::Value>,
}
#[derive(Default, Deserialize)]
pub struct AssetContainer {
    asset_type: AssetRef,
    schema: String,

    #[serde(default)]
    files: Vec<String>,

    #[serde(default)]
    folders: Vec<String>,
}

#[derive(Default, Deserialize)]
pub struct DefinedAssetContainer {
    asset_type: AssetRef,

    #[serde(default)]
    enum_files: Vec<String>,

    #[serde(default)]
    tag_files: Vec<String>,
}

pub fn load_libraies(root: &Path) -> Libraries {
    let schema_path = root.join("lib.schema.json");
    info!("{}/lib.schema.json", root.to_str().unwrap_or(""));
    let schema_validator = validators::build_validator(&schema_path);
    let mut libraries = Vec::new();

    load_library_recursive(&root, &schema_validator, &mut libraries).unwrap();

    let mut registry = registry::RegistrySystem::default();
    let mut defs = registry::DefinedRegistrySystem::default();
    load_def(&root, &libraries, &mut defs);
    load_assets(&libraries, &mut registry, &defs);
    Libraries {
        libraries: libraries,
        registry: registry,
        defs: defs,
    }
}
///depth-first order libraries (child first)
pub fn load_library_recursive(
    path: &Path,
    schema_validator: &Validator,
    libraries: &mut Vec<Library>,
) -> Result<(), String> {
    let lib_path = &path.join("lib.json");

    let Ok(text) = fs::read_to_string(lib_path) else {
        return Err(format!(
            "Could not read file: {}",
            lib_path.to_string_lossy()
        ));
    };
    let json: Value = serde_json::from_str(&text).map_err(|_| "Could not parse json")?;

    if !schema_validator.is_valid(&json) {
        return Err("Invalid json schema".into());
    }

    let manifest: LibraryManifest =
        serde_json::from_value(json).map_err(|_| "Invalid manifest structure")?;

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

pub fn load_def(root: &Path, libraries: &Vec<Library>, def: &mut registry::DefinedRegistrySystem) {
    info!("{}/enum.schema.json", root.to_str().unwrap_or(""));
    let enum_schema = &root.join("enum.schema.json");
    let enum_schema_validator = validators::build_validator(enum_schema);

    for library in libraries {
        let root = &library.root;
        for container in &library.manifest.defined_assets {
            for enums in &container.enum_files {
                let file = root.join(enums);

                let Ok(text) = fs::read_to_string(&file) else {
                    continue;
                };

                let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };

                if let Err(e) =
                    def.load_by_type(&container.asset_type, &enum_schema_validator, json)
                {
                    eprintln!("Failed to load {:?}: {}", file, e);
                }
            }
        }
    }
}

pub fn load_assets(
    libraries: &Vec<Library>,
    registry: &mut registry::RegistrySystem,
    defs: &registry::DefinedRegistrySystem,
) {
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
                //print json value

                if let Err(e) = registry.load_by_type(
                    &container.asset_type,
                    defs,
                    &asset_schema_validator,
                    json,
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
