use gtk::prelude::*;
use gtk::{Dialog, Box, Label, Entry, ProgressBar, ResponseType, Orientation, Button, Window, FileChooserDialog, FileChooserAction, FileFilter};
use gtk::glib::{self, clone};
use std::path::Path;
use crate::mod_info::ModInfo;
use crate::mod_manager::ModManager;
use crate::settings::Settings;
use crate::nexus_api::NxmLink;
use std::fs;
use tempfile::tempdir;

pub fn show_install_dialog(parent: &impl IsA<gtk::Window>, list_box: &gtk::ListBox) {
    let dialog = Dialog::builder()
        .title("Install Mod")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);

    // Nexus Mod ID input
    let id_box = Box::new(Orientation::Horizontal, 12);
    let id_label = Label::new(Some("Nexus Mod IDs:"));
    let id_entry = Entry::new();
    id_entry.set_placeholder_text(Some("Enter mod IDs separated by commas (e.g., 1,2,3)"));
    id_box.append(&id_label);
    id_box.append(&id_entry);
    content.append(&id_box);

    // Progress bar
    let progress_bar = ProgressBar::new();
    progress_bar.set_visible(false);
    content.append(&progress_bar);

    // Status label
    let status_label = Label::new(None);
    status_label.set_wrap(true);
    status_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    status_label.set_selectable(true);
    content.append(&status_label);

    dialog.add_button("Cancel", ResponseType::Cancel);
    let install_button = dialog.add_button("Install", ResponseType::Accept)
        .downcast::<Button>()
        .expect("Couldn't downcast to Button");

    // Add buttons box for multiple installation options
    let buttons_box = Box::new(Orientation::Horizontal, 12);
    buttons_box.set_halign(gtk::Align::End);
    
    let local_button = Button::with_label("Install Local Mod");
    let nexus_button = Button::with_label("Install from Nexus");
    
    buttons_box.append(&local_button);
    buttons_box.append(&nexus_button);
    content.append(&buttons_box);

    // Connect local install button
    local_button.connect_clicked(clone!(@weak dialog, @weak list_box => move |_| {
        show_file_chooser_dialog(&dialog, &list_box);
    }));

    // Connect Nexus install button (previous install functionality)
    nexus_button.connect_clicked(clone!(@weak dialog, @weak id_entry, @weak progress_bar, @weak status_label, @weak list_box => move |_| {
        let mod_ids: Result<Vec<i32>, _> = id_entry.text()
            .split(',')
            .map(|s| s.trim().parse::<i32>())
            .collect();

        match mod_ids {
            Ok(ids) if !ids.is_empty() => {
                progress_bar.set_visible(true);
                progress_bar.set_fraction(0.0);
                status_label.set_text("Fetching mod information...");
                install_button.set_sensitive(false);

                // Create a new Tokio runtime for async operations
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                let ctx = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak dialog, @weak progress_bar, @weak status_label, @weak list_box, @weak install_button => async move {
                    let settings = Settings::load();
                    let mod_manager = ModManager::new(settings).unwrap();
                    let total_mods = ids.len() as f64;
                    let mut success_count = 0;
                    let mut errors = Vec::new();

                    for (index, mod_id) in ids.iter().enumerate() {
                        let base_progress = (index as f64) / total_mods;
                        status_label.set_text(&format!("Installing mod {} of {}", index + 1, ids.len()));
                        
                        // Use the runtime to execute async operations
                        match rt.block_on(install_mod(&mod_manager, *mod_id, &progress_bar, None)) {
                            Ok(mod_info) => {
                                list_box.append(&mod_info.to_list_box_row());
                                success_count += 1;
                            },
                            Err(e) => {
                                errors.push(format!("Mod {}: {}", mod_id, e));
                            }
                        }
                    }

                    // Update UI with results
                    let status_text = if errors.is_empty() {
                        format!("Successfully installed {} mods", success_count)
                    } else {
                        format!("Installed {} mods with {} errors:\n{}", 
                            success_count, 
                            errors.len(),
                            errors.join("\n"))
                    };
                    
                    status_label.set_text(&status_text);
                    install_button.set_sensitive(true);
                    
                    if errors.is_empty() {
                        dialog.close();
                    }
                }));
            },
            _ => {
                status_label.set_text("Please enter valid mod IDs");
            }
        }
    }));

    dialog.connect_response(|dialog, response| {
        if response == ResponseType::Cancel {
            dialog.close();
        }
    });

    dialog.present();
}

pub fn show_install_dialog_with_nxm(parent: &impl IsA<gtk::Window>, list_box: &gtk::ListBox, nxm: NxmLink) {
    let dialog = Dialog::builder()
        .title("Install Mod")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);

    // Nexus Mod ID input
    let id_box = Box::new(Orientation::Horizontal, 12);
    let id_label = Label::new(Some("Nexus Mod IDs:"));
    let id_entry = Entry::new();
    id_entry.set_text(&nxm.mod_id.to_string());
    id_entry.set_sensitive(false); // Lock the input since we have specific mod info
    id_box.append(&id_label);
    id_box.append(&id_entry);
    content.append(&id_box);

    // Progress bar
    let progress_bar = ProgressBar::new();
    progress_bar.set_visible(false);
    content.append(&progress_bar);

    // Status label
    let status_label = Label::new(None);
    status_label.set_wrap(true);
    status_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    status_label.set_selectable(true);
    content.append(&status_label);

    dialog.add_button("Cancel", ResponseType::Cancel);
    let install_button = dialog.add_button("Install", ResponseType::Accept)
        .downcast::<Button>()
        .expect("Couldn't downcast to Button");

    // Clone the NXM info before moving into the closure
    let nxm_key = nxm.key.clone();
    let nxm_mod_id = nxm.mod_id;
    let nxm_expires = nxm.expires;

    // Connect install button click
    install_button.connect_clicked(clone!(@weak dialog, @weak progress_bar, @weak status_label, @weak list_box, @weak install_button, @strong nxm_key => move |_| {
        progress_bar.set_visible(true);
        progress_bar.set_fraction(0.0);
        status_label.set_text("Installing mod...");
        install_button.set_sensitive(false);

        let ctx = glib::MainContext::default();
        let nxm_key = nxm_key.clone(); // Clone again for the inner closure
        
        ctx.spawn_local(clone!(@weak dialog, @weak progress_bar, @weak status_label, @weak list_box, @weak install_button, @strong nxm_key => async move {
            let settings = Settings::load();
            let mod_manager = ModManager::new(settings).unwrap();
            
            match install_mod(&mod_manager, nxm_mod_id, &progress_bar, Some((nxm_key, nxm_expires))).await {
                Ok(mod_info) => {
                    list_box.append(&mod_info.to_list_box_row());
                    dialog.close();
                },
                Err(e) => {
                    status_label.set_text(&format!("Error installing mod: {}", e));
                    progress_bar.set_visible(false);
                    install_button.set_sensitive(true);
                }
            }
        }));
    }));

    dialog.connect_response(|dialog, response| {
        if response == ResponseType::Cancel {
            dialog.close();
        }
    });

    dialog.present();
}

async fn install_mod(mod_manager: &ModManager, mod_id: i32, progress_bar: &ProgressBar, nxm_info: Option<(String, i64)>) -> anyhow::Result<ModInfo> {
    let client = mod_manager.nexus_client().ok_or_else(|| anyhow::anyhow!("No Nexus client available"))?;
    
    // Get mod info
    progress_bar.set_fraction(0.2);
    let mod_info = client.get_mod_info(mod_id).await?;
    
    // Get mod files
    progress_bar.set_fraction(0.4);
    let mod_files = client.get_mod_files(mod_id).await?;
    
    // Get the latest main file
    let file = mod_files.iter()
        .filter(|f| f.category_id == Some(1))
        .max_by_key(|f| f.version.clone())
        .ok_or_else(|| anyhow::anyhow!("No main files available for this mod"))?;
    
    // Download mod
    progress_bar.set_fraction(0.6);
    let mod_data = client.download_mod(mod_id, file.id(), nxm_info).await?;
    
    progress_bar.set_fraction(0.8);
    
    let final_path = if file.file_name.to_lowercase().ends_with(".zip") {
        // Create temp dir for extraction
        let temp_dir = tempfile::tempdir()?;
        let temp_zip = temp_dir.path().join(&file.file_name);
        std::fs::write(&temp_zip, &mod_data)?;
        
        // Extract pak files
        let mut pak_path = None;
        if let Ok(file) = std::fs::File::open(&temp_zip) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                for i in 0..archive.len() {
                    if let Ok(mut zip_file) = archive.by_index(i) {
                        let outpath = match zip_file.enclosed_name() {
                            Some(path) => path.to_owned(),
                            None => continue,
                        };
                        
                        if outpath.extension().map_or(false, |ext| ext == "pak") {
                            let pak_path_temp = mod_manager.mods_path().join(outpath.file_name().unwrap());
                            let mut outfile = std::fs::File::create(&pak_path_temp)?;
                            std::io::copy(&mut zip_file, &mut outfile)?;
                            pak_path = Some(pak_path_temp);
                            break; // Install first pak file found
                        }
                    }
                }
            }
        }
        
        pak_path.ok_or_else(|| anyhow::anyhow!("No .pak file found in zip archive"))?
    } else {
        // Direct pak file
        let mod_path = mod_manager.mods_path().join(&file.file_name);
        std::fs::write(&mod_path, mod_data)?;
        mod_path
    };
    
    progress_bar.set_fraction(1.0);
    
    Ok(ModInfo {
        name: mod_info.name,
        version: file.version.clone().unwrap_or_else(|| "1.0".to_string()),
        author: mod_info.user.name,
        description: mod_info.description,
        nexus_mod_id: Some(mod_id),
        installed_path: Some(final_path),
        enabled: true,
    })
}

pub fn show_file_chooser_dialog(parent: &impl IsA<Window>, list_box: &gtk::ListBox) {
    let file_chooser = FileChooserDialog::new(
        Some("Select Mod File"),
        Some(parent),
        FileChooserAction::Open,
        &[("Cancel", ResponseType::Cancel), ("Open", ResponseType::Accept)]
    );

    // Add filters for both .pak and .zip files
    let pak_filter = FileFilter::new();
    pak_filter.add_pattern("*.pak");
    pak_filter.set_name(Some("PAK files"));
    file_chooser.add_filter(&pak_filter);

    let zip_filter = FileFilter::new();
    zip_filter.add_pattern("*.zip");
    zip_filter.set_name(Some("ZIP files"));
    file_chooser.add_filter(&zip_filter);

    file_chooser.connect_response(clone!(@weak list_box => move |file_chooser, response| {
        if response == ResponseType::Accept {
            if let Some(file) = file_chooser.file() {
                if let Some(path) = file.path() {
                    let settings = Settings::load();
                    if let Ok(mod_manager) = ModManager::new(settings) {
                        match path.extension().and_then(|ext| ext.to_str()) {
                            Some("pak") => {
                                handle_pak_file(&mod_manager, &path, &list_box);
                            },
                            Some("zip") => {
                                handle_zip_file(&mod_manager, &path, &list_box);
                            },
                            _ => eprintln!("Unsupported file type"),
                        }
                    }
                }
            }
        }
        file_chooser.close();
    }));

    file_chooser.show();
}

fn handle_pak_file(mod_manager: &ModManager, path: &Path, list_box: &gtk::ListBox) {
    if let Ok(dest_path) = mod_manager.install_local_mod(path) {
        let name = dest_path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mod_info = ModInfo {
            name,
            version: String::from("1.0"),
            author: String::from("Unknown"),
            description: String::new(),
            nexus_mod_id: None,
            installed_path: Some(dest_path),
            enabled: true,
        };

        list_box.append(&mod_info.to_list_box_row());
        let _ = mod_manager.add_to_mod_list(mod_info);
    }
}

fn handle_zip_file(mod_manager: &ModManager, path: &Path, list_box: &gtk::ListBox) {
    // Create a temporary directory for extraction
    if let Ok(temp_dir) = tempdir() {
        if let Ok(file) = fs::File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                // Extract all .pak files
                for i in 0..archive.len() {
                    if let Ok(mut file) = archive.by_index(i) {
                        let outpath = match file.enclosed_name() {
                            Some(path) => path.to_owned(),
                            None => continue,
                        };

                        if outpath.extension().map_or(false, |ext| ext == "pak") {
                            let temp_path = temp_dir.path().join(&outpath);
                            
                            // Create parent directories if needed
                            if let Some(parent) = temp_path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }

                            // Extract the .pak file
                            if let Ok(mut outfile) = fs::File::create(&temp_path) {
                                if let Ok(_) = std::io::copy(&mut file, &mut outfile) {
                                    // Install the extracted .pak file
                                    handle_pak_file(mod_manager, &temp_path, list_box);
                                }
                            }
                        }
                    }
                }
            }
        }
        // Temp dir is automatically cleaned up when it goes out of scope
    }
}

fn show_error_dialog(parent: &impl IsA<Window>, message: &str) {
    let dialog = gtk::MessageDialog::new(
        Some(parent),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        message,
    );
    
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.present();
} 