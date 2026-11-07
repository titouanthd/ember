// crates/ember-core/src/io.rs
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Erreur d'I/O fichier (lecture, écriture, (dé)sérialisation RON).
#[derive(Debug)]
pub enum IoError {
    Io(std::io::Error),
    /// Sérialisation RON (écriture).
    Serialize(ron::Error),
    /// Désérialisation RON (lecture). Inclut la position ligne/colonne.
    Deserialize(ron::error::SpannedError),
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Io(e) => write!(f, "erreur I/O : {e}"),
            IoError::Serialize(e) => write!(f, "erreur de sérialisation RON : {e}"),
            IoError::Deserialize(e) => write!(f, "erreur de désérialisation RON : {e}"),
        }
    }
}

impl std::error::Error for IoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IoError::Io(e) => Some(e),
            IoError::Serialize(e) => Some(e),
            IoError::Deserialize(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for IoError {
    fn from(e: std::io::Error) -> Self {
        IoError::Io(e)
    }
}

impl From<ron::Error> for IoError {
    fn from(e: ron::Error) -> Self {
        IoError::Serialize(e)
    }
}

impl From<ron::error::SpannedError> for IoError {
    fn from(e: ron::error::SpannedError) -> Self {
        IoError::Deserialize(e)
    }
}

pub fn save_to_file<T: Serialize>(path: &Path, data: &T) -> Result<(), IoError> {
    let ron_string = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())?;
    fs::write(path, ron_string)?;
    Ok(())
}

pub fn load_from_file<T: DeserializeOwned>(path: &Path) -> Result<T, IoError> {
    let content = fs::read_to_string(path)?;
    Ok(ron::de::from_str(&content)?)
}