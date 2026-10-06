//! TOML settings store (ADR-0002 §5): atomic writes, corruption reported.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    application::ports::{SettingsError, SettingsStore},
    domain::mosque::{MosqueId, MosqueSummary},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SelectedEntry {
    slug: String,
    name: String,
    #[serde(default)]
    place: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    selected_mosque: Option<SelectedEntry>,
}

/// TOML-backed store at a caller-supplied path (tests use tempdirs).
#[derive(Debug, Clone)]
pub struct TomlSettings {
    path: PathBuf,
}

impl TomlSettings {
    pub fn new(path: impl Into<PathBuf>) -> TomlSettings {
        TomlSettings { path: path.into() }
    }

    /// XDG default: `config_dir()/mawaqit-tui/config.toml`. `None` only on
    /// platforms without a config dir — the caller decides the UX (M4).
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("mawaqit-tui").join("config.toml"))
    }
}

impl SettingsStore for TomlSettings {
    fn load(&self) -> Result<Option<MosqueSummary>, SettingsError> {
        let raw = match fs::read(&self.path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(SettingsError::Io(err.to_string())),
        };
        let file: SettingsFile =
            toml::from_slice(&raw).map_err(|_| SettingsError::Corrupt)?;
        let Some(entry) = file.selected_mosque else {
            return Ok(None);
        };
        // A hostile slug in the file is corruption, not an empty selection.
        let id = MosqueId::parse(&entry.slug).map_err(|_| SettingsError::Corrupt)?;
        Ok(Some(MosqueSummary { id, name: entry.name, place: entry.place }))
    }

    fn save(&self, selection: &MosqueSummary) -> Result<(), SettingsError> {
        let file = SettingsFile {
            selected_mosque: Some(SelectedEntry {
                slug: selection.id.as_str().to_owned(),
                name: selection.name.clone(),
                place: selection.place.clone(),
            }),
        };
        let raw = toml::to_string(&file)
            .map_err(|err| SettingsError::Io(format!("serialize: {err}")))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| SettingsError::Io(err.to_string()))?;
        }
        // Atomic: write a sibling temp file, then rename over the target.
        let tmp = self.path.with_extension("toml.tmp");
        fs::write(&tmp, raw).map_err(|err| SettingsError::Io(err.to_string()))?;
        fs::rename(&tmp, &self.path).map_err(|err| SettingsError::Io(err.to_string()))?;
        Ok(())
    }
}
