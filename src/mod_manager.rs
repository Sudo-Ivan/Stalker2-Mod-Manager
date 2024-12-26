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
        // Load the mod list from JSON
        let mod_list = self.load_mod_list()?;
        
        // Get paths from mod list
        let mut mods: Vec<PathBuf> = mod_list.iter()
            .filter_map(|mod_info| mod_info.installed_path.clone())
            .collect();

        // Also scan directories for any untracked mods
        let existing_paths: Vec<PathBuf> = mods.clone();

        // Check ~mods directory for untracked mods
        if self.mods_path.exists() {
            mods.extend(
                std::fs::read_dir(&self.mods_path)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension().map_or(false, |ext| ext == "pak") 
                        && !existing_paths.contains(path)
                    })
            );
        }

        // Check unloaded mods directory for untracked mods
        if self.unloaded_mods_path.exists() {
            mods.extend(
                std::fs::read_dir(&self.unloaded_mods_path)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension().map_or(false, |ext| ext == "pak")
                        && !existing_paths.contains(path)
                    })
            );
        }

        Ok(mods)
    }

    pub fn enable_mod(&self, mod_path: &Path) -> Result<()> {
        eprintln!("Enabling mod: {:?}", mod_path);
        
        let file_name = mod_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?;
        
        // Get absolute paths
        let game_path = self.settings.game_path.clone()
            .ok_or_else(|| anyhow::anyhow!("Game path not set"))?;
        
        // Check if the file is in the unloaded mods directory
        let unloaded_path = self.unloaded_mods_path.join(file_name);
        let enabled_path = self.mods_path.join(file_name);
        
        eprintln!("Checking unloaded path: {:?}", unloaded_path);
        eprintln!("Checking enabled path: {:?}", enabled_path);
        
        // If it's already in the enabled directory, nothing to do
        if enabled_path.exists() {
            eprintln!("Mod is already enabled");
            return Ok(());
        }
        
        // If it's in the unloaded directory, move it
        if unloaded_path.exists() {
            std::fs::create_dir_all(&self.mods_path)?;
            std::fs::rename(&unloaded_path, &enabled_path)?;
            eprintln!("Moved mod from unloaded to enabled directory");
            return Ok(());
        }
        
        Err(anyhow::anyhow!("Mod file not found in expected locations"))
    }

    pub fn disable_mod(&self, mod_path: &Path) -> Result<()> {
        eprintln!("Disabling mod: {:?}", mod_path);
        
        let file_name = mod_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid mod file name"))?;
        
        // Check if the file is in the enabled mods directory
        let unloaded_path = self.unloaded_mods_path.join(file_name);
        let enabled_path = self.mods_path.join(file_name);
        
        eprintln!("Checking unloaded path: {:?}", unloaded_path);
        eprintln!("Checking enabled path: {:?}", enabled_path);
        
        // If it's already in the unloaded directory, nothing to do
        if unloaded_path.exists() {
            eprintln!("Mod is already disabled");
            return Ok(());
        }
        
        // If it's in the enabled directory, move it
        if enabled_path.exists() {
            std::fs::create_dir_all(&self.unloaded_mods_path)?;
            std::fs::rename(&enabled_path, &unloaded_path)?;
            eprintln!("Moved mod from enabled to unloaded directory");
            return Ok(());
        }
        
        Err(anyhow::anyhow!("Mod file not found in expected locations"))
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
        let mut mods: Vec<ModInfo> = Vec::new();
        
        // First load saved mod list
        let mod_list_path = self.settings.game_path.clone()
            .unwrap_or_else(|| PathBuf::new())
            .join("Stalker2")
            .join("ModManager")
            .join("mod_list.json");

        if mod_list_path.exists() {
            let json = fs::read_to_string(&mod_list_path)?;
            mods = serde_json::from_str(&json)?;
        }

        // Then scan for untracked mods in both directories
        let existing_paths: Vec<PathBuf> = mods.iter()
            .filter_map(|m| m.installed_path.clone())
            .collect();

        // Check ~mods directory
        if self.mods_path.exists() {
            for entry in std::fs::read_dir(&self.mods_path)? {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "pak") 
                        && !existing_paths.contains(&path) {
                        // Add untracked mod
                        mods.push(ModInfo {
                            name: path.file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            version: "Unknown".to_string(),
                            author: "Unknown".to_string(),
                            description: String::new(),
                            nexus_mod_id: None,
                            installed_path: Some(path),
                            enabled: true,
                        });
                    }
                }
            }
        }

        // Save updated list if new mods were found
        self.save_mod_list(&mods)?;
        
        Ok(mods)
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

    pub fn unloaded_mods_path(&self) -> &Path {
        &self.unloaded_mods_path
    }
} 