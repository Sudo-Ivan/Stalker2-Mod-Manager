use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::nexus_api::NexusClient;
use crate::settings::Settings;
use crate::mod_info::ModInfo;
use std::fs;
use serde_json;

pub struct ModManager {
    settings: Settings,
    nexus_client: Option<NexusClient>,
    mods_path: PathBuf,
    unloaded_mods_path: PathBuf,
}

impl ModManager {
    pub fn new(settings: Settings) -> Result<Self> {
        let nexus_client = if let Some(api_key) = &settings.nexus_api_key {
            Some(NexusClient::new(api_key)?)
        } else {
            None
        };

        let game_path = settings.game_path.clone().unwrap_or_else(|| PathBuf::new());
        let mods_path = game_path.join("Stalker2").join("Content").join("Paks").join("~mods");
        let unloaded_mods_path = game_path.join("Stalker2").join("ModManager").join("unloaded_mods");

        // Create both directories if they don't exist
        std::fs::create_dir_all(&mods_path)?;
        std::fs::create_dir_all(&unloaded_mods_path)?;

        Ok(Self {
            settings,
            nexus_client,
            mods_path,
            unloaded_mods_path,
        })
    }

    pub async fn install_mod(&self, mod_id: i32, file_id: i32) -> Result<()> {
        let client = self.nexus_client.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Nexus API key not configured")
        })?;

        // Get mod info and files
        let _mod_info = client.get_mod_info(mod_id).await?;
        let mod_files = client.get_mod_files(mod_id).await?;
        
        let file = mod_files.iter()
            .find(|f| f.id() == file_id)
            .ok_or_else(|| anyhow::anyhow!("File ID not found"))?;
        
        // Download and save the mod
        let mod_data = client.download_mod(mod_id, file_id, None).await?;
        
        // Ensure mods directory exists
        std::fs::create_dir_all(&self.mods_path)?;

        // Write mod file
        let mod_path = self.mods_path.join(&file.file_name);
        std::fs::write(mod_path, mod_data)?;

        Ok(())
    }

    pub fn get_installed_mods(&self) -> Result<Vec<PathBuf>> {
        let mut mods = Vec::new();

        // Get loaded mods
        if self.mods_path.exists() {
            mods.extend(std::fs::read_dir(&self.mods_path)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().map_or(false, |ext| ext == "pak")));
        }

        // Get unloaded mods
        if self.unloaded_mods_path.exists() {
            mods.extend(std::fs::read_dir(&self.unloaded_mods_path)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().map_or(false, |ext| ext == "pak")));
        }

        Ok(mods)
    }

    pub fn enable_mod(&self, mod_path: &Path) -> Result<()> {
        if mod_path.starts_with(&self.unloaded_mods_path) {
            let file_name = mod_path.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?;
            let new_path = self.mods_path.join(file_name);
            std::fs::rename(mod_path, new_path)?;
        }
        Ok(())
    }

    pub fn disable_mod(&self, mod_path: &Path) -> Result<()> {
        if mod_path.starts_with(&self.mods_path) {
            let file_name = mod_path.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?;
            let new_path = self.unloaded_mods_path.join(file_name);
            std::fs::rename(mod_path, new_path)?;
        }
        Ok(())
    }

    pub fn nexus_client(&self) -> Option<&NexusClient> {
        self.nexus_client.as_ref()
    }

    pub fn mods_path(&self) -> &Path {
        &self.mods_path
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn install_local_mod(&self, source_path: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.unloaded_mods_path)?;

        let file_name = source_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
        let dest_path = self.unloaded_mods_path.join(file_name);

        // Copy the file
        std::fs::copy(source_path, &dest_path)?;
        
        Ok(dest_path)
    }

    pub fn is_mod_enabled(&self, mod_path: &Path) -> bool {
        mod_path.starts_with(&self.mods_path)
    }

    pub fn save_mod_list(&self, mods: &[ModInfo]) -> Result<()> {
        let mod_list_path = self.settings.game_path.clone()
            .unwrap_or_else(|| PathBuf::new())
            .join("Stalker2")
            .join("ModManager")
            .join("mod_list.json");

        // Create parent directories if they don't exist
        if let Some(parent) = mod_list_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(mods)?;
        fs::write(mod_list_path, json)?;
        Ok(())
    }

    pub fn load_mod_list(&self) -> Result<Vec<ModInfo>> {
        let mod_list_path = self.settings.game_path.clone()
            .unwrap_or_else(|| PathBuf::new())
            .join("Stalker2")
            .join("ModManager")
            .join("mod_list.json");

        if !mod_list_path.exists() {
            return Ok(Vec::new());
        }

        let json = fs::read_to_string(mod_list_path)?;
        let mods: Vec<ModInfo> = serde_json::from_str(&json)?;
        
        // Verify mods still exist and update their enabled status
        let mut verified_mods = Vec::new();
        for mut mod_info in mods {
            if let Some(path) = &mod_info.installed_path {
                if path.exists() {
                    mod_info.enabled = self.is_mod_enabled(path);
                    verified_mods.push(mod_info);
                }
            }
        }

        Ok(verified_mods)
    }
} 