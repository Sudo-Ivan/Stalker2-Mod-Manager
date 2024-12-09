use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::nexus_api::NexusClient;
use crate::settings::Settings;
use crate::mod_info::ModInfo;
use std::fs;
use serde_json;
use zip::{ZipWriter, write::FileOptions};
use std::io::{Read, Write};

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
        if !mod_path.starts_with(&self.mods_path) {
            let file_name = mod_path.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?;
            let new_path = self.mods_path.join(file_name);
            if new_path.exists() {
                std::fs::remove_file(&new_path)?;
            }
            std::fs::rename(mod_path, new_path)?;
        }
        Ok(())
    }

    pub fn disable_mod(&self, mod_path: &Path) -> Result<()> {
        if !mod_path.starts_with(&self.unloaded_mods_path) {
            let file_name = mod_path.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?;
            let new_path = self.unloaded_mods_path.join(file_name);
            if new_path.exists() {
                std::fs::remove_file(&new_path)?;
            }
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
        std::fs::create_dir_all(&self.mods_path)?;

        let file_name = source_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
        
        // Remove any duplicate .pak extensions
        let name = file_name.to_string_lossy();
        let clean_name = name.trim_end_matches(".pak").to_string() + ".pak";
        let dest_path = self.mods_path.join(clean_name);

        // Copy the file
        std::fs::copy(source_path, &dest_path)?;
        
        Ok(dest_path)
    }

    pub fn is_mod_enabled(&self, mod_path: &Path) -> bool {
        if let Some(file_name) = mod_path.file_name() {
            self.mods_path.join(file_name).exists()
        } else {
            false
        }
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

        println!("Loading mods from: {:?}", mod_list_path);

        if !mod_list_path.exists() {
            println!("No mod list file found");
            return Ok(Vec::new());
        }

        let json = fs::read_to_string(&mod_list_path)?;
        println!("Loaded JSON: {}", json);
        
        let mods: Vec<ModInfo> = serde_json::from_str(&json)?;
        println!("Parsed {} mods from JSON", mods.len());
        
        let mut verified_mods = Vec::new();
        for mut mod_info in mods {
            if let Some(path) = &mod_info.installed_path {
                let file_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .map(|n| n.trim_end_matches(".pak").to_string() + ".pak");
                
                if let Some(name) = file_name {
                    let enabled_path = self.mods_path.join(&name);
                    let disabled_path = self.unloaded_mods_path.join(&name);
                    
                    println!("Checking mod paths: \nEnabled: {:?}\nDisabled: {:?}", enabled_path, disabled_path);
                    
                    // Check if either path exists
                    if enabled_path.exists() {
                        println!("Found enabled mod: {}", name);
                        mod_info.enabled = true;
                        mod_info.installed_path = Some(enabled_path);
                        verified_mods.push(mod_info);
                    } else if disabled_path.exists() {
                        println!("Found disabled mod: {}", name);
                        mod_info.enabled = false;
                        mod_info.installed_path = Some(disabled_path);
                        verified_mods.push(mod_info);
                    } else {
                        println!("Mod file not found at either location: {}", name);
                    }
                }
            }
        }

        println!("Returning {} verified mods", verified_mods.len());
        Ok(verified_mods)
    }

    pub fn export_mods(&self, zip_path: &Path) -> Result<()> {
        let file = std::fs::File::create(zip_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // First, write the manifest
        let mod_list = self.load_mod_list()?;
        let manifest = serde_json::to_string_pretty(&mod_list)?;
        zip.start_file("mod-manifest.json", options)?;
        zip.write_all(manifest.as_bytes())?;

        // Then write each mod file
        for mod_info in mod_list {
            if let Some(path) = mod_info.installed_path {
                if path.exists() {
                    let file_name = path.file_name()
                        .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?
                        .to_string_lossy();
                    
                    zip.start_file(format!("mods/{}", file_name), options)?;
                    let mut file = std::fs::File::open(&path)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                }
            }
        }

        zip.finish()?;
        Ok(())
    }

    pub fn import_mods(&self, zip_path: &Path) -> Result<()> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // First, read the manifest into a String
        let manifest_contents = {
            let mut manifest_file = archive.by_name("mod-manifest.json")?;
            let mut contents = String::new();
            manifest_file.read_to_string(&mut contents)?;
            contents
        };

        // Now parse the manifest
        let mod_list: Vec<ModInfo> = serde_json::from_str(&manifest_contents)?;

        // Create necessary directories
        std::fs::create_dir_all(&self.mods_path)?;
        std::fs::create_dir_all(&self.unloaded_mods_path)?;

        // Track successfully imported mods
        let mut imported_mods = Vec::new();

        // Extract mod files
        for mut mod_info in mod_list {
            if let Some(path) = mod_info.installed_path.clone() {
                let file_name = path.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?
                    .to_string_lossy();
                
                let zip_path = format!("mods/{}", file_name);
                if let Ok(mut zip_file) = archive.by_name(&zip_path) {
                    let target_path = if mod_info.enabled {
                        self.mods_path.join(&*file_name)
                    } else {
                        self.unloaded_mods_path.join(&*file_name)
                    };

                    let mut target_file = std::fs::File::create(&target_path)?;
                    std::io::copy(&mut zip_file, &mut target_file)?;

                    // Update mod_info with new path
                    mod_info.installed_path = Some(target_path);
                    imported_mods.push(mod_info);
                }
            }
        }

        // Update mod list with imported mods
        let mut current_mods = self.load_mod_list()?;
        current_mods.extend(imported_mods);
        self.save_mod_list(&current_mods)?;

        Ok(())
    }

    pub fn add_to_mod_list(&self, mod_info: ModInfo) -> Result<()> {
        let mut current_mods = self.load_mod_list()?;
        current_mods.push(mod_info);
        self.save_mod_list(&current_mods)?;
        Ok(())
    }
} 