use std::fs;
use std::path::Path;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn save_to_file<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    let ron_string = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
        .map_err(|e| format!("Erreur de sérialisation RON : {}", e))?;
    fs::write(path, ron_string)
        .map_err(|e| format!("Erreur d'écriture du fichier : {}", e))?;
    Ok(())
}

pub fn load_from_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Erreur de lecture du fichier : {}", e))?;
    let data = ron::de::from_str(&content)
        .map_err(|e| format!("Erreur de désérialisation RON : {}", e))?;
    Ok(data)
}