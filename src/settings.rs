use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: Option<PathBuf>,
    pub nexus_api_key: Option<String>,
}

impl Settings {
    pub fn load() -> Self {
        if let Some(config_dir) = directories::ProjectDirs::from("", "", "Stalker2ModManager") {
            if let Ok(contents) = std::fs::read_to_string(
                config_dir.config_dir().join("settings.json")
            ) {
                if let Ok(settings) = serde_json::from_str(&contents) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(config_dir) = directories::ProjectDirs::from("", "", "Stalker2ModManager") {
            std::fs::create_dir_all(config_dir.config_dir())?;
            let config_path = config_dir.config_dir().join("settings.json");
            let contents = serde_json::to_string_pretty(self)?;
            std::fs::write(config_path, contents)?;
        }
        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game_path: None,
            nexus_api_key: None,
        }
    }
} 