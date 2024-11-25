use gtk::prelude::*;
use gtk::{Dialog, Box, Label, Entry, ProgressBar, ResponseType, Orientation, Button, Window};
use gtk::glib::{self, clone};
use std::path::Path;
use crate::mod_info::ModInfo;
use crate::mod_manager::ModManager;
use crate::settings::Settings;
use crate::nexus_api::NxmLink;

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
                        
                        match install_mod(&mod_manager, *mod_id, &progress_bar, None).await {
                            Ok(mod_info) => {
                                list_box.append(&mod_info.to_list_box_row());
                                success_count += 1;
                            },
                            Err(e) => {
                                errors.push(format!("Mod {}: {}", mod_id, e));
                            }
                        }
                        progress_bar.set_fraction(base_progress + (1.0 / total_mods));
                    }

                    if errors.is_empty() {
                        dialog.close();
                    } else {
                        let error_text = format!(
                            "Installed {}/{} mods successfully.\nErrors:\n{}",
                            success_count,
                            ids.len(),
                            errors.join("\n")
                        );
                        status_label.set_text(&error_text);
                        progress_bar.set_visible(false);
                        install_button.set_sensitive(true);
                    }
                }));
            },
            Ok(_) => {
                status_label.set_text("Please enter at least one mod ID");
            },
            Err(_) => {
                status_label.set_text("Please enter valid mod IDs separated by commas");
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
    
    // Get the latest main file (filter for category_name "MAIN")
    let file = mod_files.iter()
        .filter(|f| f.category_id == Some(1)) // 1 is MAIN category
        .max_by_key(|f| f.version.clone())
        .ok_or_else(|| anyhow::anyhow!("No main files available for this mod"))?;
    
    // Download mod
    progress_bar.set_fraction(0.6);
    let mod_data = client.download_mod(mod_id, file.id(), nxm_info).await?;
    
    // Install mod
    progress_bar.set_fraction(0.8);
    let mod_path = mod_manager.mods_path().join(&file.file_name);
    std::fs::write(&mod_path, mod_data)?;
    
    progress_bar.set_fraction(1.0);
    
    Ok(ModInfo {
        name: mod_info.name,
        version: file.version.clone().unwrap_or_else(|| "1.0".to_string()),
        author: mod_info.user.name,
        description: mod_info.description,
        nexus_mod_id: Some(mod_id),
        installed_path: Some(mod_path),
        enabled: true,
    })
}

fn show_file_chooser_dialog(parent: &impl IsA<Window>, list_box: &gtk::ListBox) {
    let dialog = gtk::FileChooserDialog::new(
        Some("Select PAK File"),
        Some(parent),
        gtk::FileChooserAction::Open,
        &[
            ("Cancel", gtk::ResponseType::Cancel),
            ("Open", gtk::ResponseType::Accept),
        ],
    );

    // Add file filter for .pak files
    let filter = gtk::FileFilter::new();
    filter.add_pattern("*.pak");
    filter.set_name(Some("PAK files"));
    dialog.add_filter(&filter);

    dialog.connect_response(clone!(@weak list_box => move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    let settings = Settings::load();
                    let mod_manager = ModManager::new(settings).unwrap();
                    
                    match install_local_mod(&mod_manager, &path) {
                        Ok(mod_info) => {
                            list_box.append(&mod_info.to_list_box_row());
                        },
                        Err(e) => {
                            show_error_dialog(dialog, &format!("Failed to install mod: {}", e));
                        }
                    }
                }
            }
        }
        dialog.close();
    }));

    dialog.present();
}

fn install_local_mod(mod_manager: &ModManager, source_path: &Path) -> anyhow::Result<ModInfo> {
    // Create mods directory if it doesn't exist
    std::fs::create_dir_all(mod_manager.mods_path())?;

    // Get the filename from the source path
    let file_name = source_path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

    // Construct destination path
    let dest_path = mod_manager.mods_path().join(file_name);

    // Copy the file
    std::fs::copy(source_path, &dest_path)?;

    // Create ModInfo from local file
    Ok(ModInfo {
        name: file_name.to_string_lossy().into_owned(),
        version: "Local".to_string(),
        author: "Local".to_string(),
        description: "Locally installed mod".to_string(),
        nexus_mod_id: None,
        installed_path: Some(dest_path),
        enabled: true,
    })
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