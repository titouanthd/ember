//! `uid_registry.ron`: stable path <-> UID mapping.

use crate::{AssetError, Uid};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
struct RegistryFile {
    assets: BTreeMap<String, String>,
}

/// Paths are asset-root relative with `/` separators.
#[derive(Debug, Default, Clone)]
pub struct UidRegistry {
    by_path: BTreeMap<String, Uid>,
    by_uid: HashMap<Uid, String>,
}

impl UidRegistry {
    /// A missing file is an empty registry.
    pub fn load(file: &Path) -> Result<Self, AssetError> {
        let text = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(AssetError::Io(format!("{}: {e}", file.display()))),
        };
        let parsed: RegistryFile = ron::from_str(&text)
            .map_err(|e| AssetError::Parse(format!("{}: {e}", file.display())))?;
        let mut reg = Self::default();
        for (path, uid) in parsed.assets {
            let uid = Uid::parse(&uid)?;
            reg.by_uid.insert(uid.clone(), path.clone());
            reg.by_path.insert(path, uid);
        }
        Ok(reg)
    }

    pub fn save(&self, file: &Path) -> Result<(), AssetError> {
        let data = RegistryFile {
            assets: self
                .by_path
                .iter()
                .map(|(p, u)| (p.clone(), u.as_str().to_string()))
                .collect(),
        };
        let text = ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::default())
            .expect("registry serializes");
        std::fs::write(file, text).map_err(|e| AssetError::Io(format!("{}: {e}", file.display())))
    }

    pub fn uid_for_path(&self, path: &str) -> Option<&Uid> {
        self.by_path.get(path)
    }

    pub fn path_for_uid(&self, uid: &Uid) -> Option<&str> {
        self.by_uid.get(uid).map(String::as_str)
    }

    /// Returns the existing UID, or mints one from `content_hash` and
    /// the file stem. A UID is never changed once assigned.
    pub fn assign(&mut self, path: &str, content_hash: u64) -> Uid {
        if let Some(uid) = self.by_path.get(path) {
            return uid.clone();
        }
        let stem = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("asset")
            .replace(char::is_whitespace, "_");
        let mut h = content_hash;
        let uid = loop {
            let candidate = Uid::parse(&format!("uid://{stem}_{:05x}", h & 0xf_ffff))
                .expect("generated uid is valid");
            if !self.by_uid.contains_key(&candidate) {
                break candidate;
            }
            h = h.wrapping_add(1);
        };
        self.by_uid.insert(uid.clone(), path.to_string());
        self.by_path.insert(path.to_string(), uid.clone());
        uid
    }

    /// Moves the entry to `new`, keeping its UID.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), AssetError> {
        let uid = self
            .by_path
            .remove(old)
            .ok_or_else(|| AssetError::NotFound(old.to_string()))?;
        self.by_uid.insert(uid.clone(), new.to_string());
        self.by_path.insert(new.to_string(), uid);
        Ok(())
    }
}
