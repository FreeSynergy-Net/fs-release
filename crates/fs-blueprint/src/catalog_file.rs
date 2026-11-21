//! Pattern: Adapter (the catalog of building blocks read from a TOML file: skeleton versions, capabilities, kits, basic types, modules, bundles).

use crate::kit::{BundleInfo, Kind, KitCatalog, ModuleInfo};
use serde::Deserialize;

/// The catalog shipped with the tool (`catalog/fs.toml`).
pub const BUILTIN: &str = include_str!("../catalog/fs.toml");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkeletonRow {
    version: String,
    #[serde(default)]
    pending: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleRow {
    name: String,
    kind: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    words: Vec<String>,
    #[serde(default)]
    presets: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleRow {
    name: String,
    #[serde(default)]
    parts: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    presets: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractRow {
    key: String,
    levels: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    skeleton: Vec<SkeletonRow>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    kits: Vec<String>,
    #[serde(default)]
    basics: Vec<String>,
    #[serde(default)]
    contract: Vec<ContractRow>,
    #[serde(default)]
    module: Vec<ModuleRow>,
    #[serde(default)]
    bundle: Vec<BundleRow>,
}

/// The catalog of a file. Built once; the check only asks.
#[derive(Debug)]
pub struct FileCatalog {
    file: File,
}

impl FileCatalog {
    /// Reads a catalog from TOML text.
    ///
    /// # Errors
    /// The reason as text: the text is no TOML, a key is unknown, or a module has a kind that is none of `standard`, `toolkit`, `component`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
        if let Some(m) = file.module.iter().find(|m| Kind::parse(&m.kind).is_none()) {
            return Err(format!(
                "module {}: the kind {} is none of standard, toolkit, component",
                m.name, m.kind
            ));
        }
        Ok(Self { file })
    }

    /// The catalog shipped with the tool.
    ///
    /// # Panics
    /// Never for the shipped file (a test reads it).
    #[must_use]
    pub fn builtin() -> Self {
        Self::parse(BUILTIN).expect("the shipped catalog is valid")
    }
}

impl KitCatalog for FileCatalog {
    fn skeleton_known(&self, version: &str) -> bool {
        self.file.skeleton.iter().any(|s| s.version == version)
    }
    fn skeleton_pending(&self, version: &str) -> Vec<String> {
        self.file
            .skeleton
            .iter()
            .find(|s| s.version == version)
            .map(|s| s.pending.clone())
            .unwrap_or_default()
    }
    fn capability_known(&self, key: &str) -> bool {
        self.file.capabilities.iter().any(|c| c == key)
    }
    fn contract_known(&self, key: &str, version: &str) -> bool {
        self.file
            .contract
            .iter()
            .any(|c| c.key == key && c.levels.iter().any(|l| l == version))
    }
    fn module(&self, name: &str) -> Option<ModuleInfo> {
        let m = self.file.module.iter().find(|m| m.name == name)?;
        Some(ModuleInfo {
            kind: Kind::parse(&m.kind)?,
            tags: m.tags.clone(),
            words: m.words.clone(),
            presets: m.presets.clone(),
        })
    }
    fn bundle(&self, name: &str) -> Option<BundleInfo> {
        let b = self.file.bundle.iter().find(|b| b.name == name)?;
        Some(BundleInfo {
            parts: b.parts.clone(),
            tags: b.tags.clone(),
            presets: b.presets.clone(),
        })
    }
    fn kit_known(&self, name: &str) -> bool {
        self.file.kits.iter().any(|k| k == name)
    }
    fn basic_known(&self, name: &str) -> bool {
        self.file.basics.iter().any(|b| b == name)
    }
}
