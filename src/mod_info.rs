use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use gtk::prelude::*;
use gtk::glib;
use crate::settings::Settings;
use crate::mod_manager::ModManager;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub nexus_mod_id: Option<i32>,
    pub installed_path: Option<PathBuf>,
    pub enabled: bool,
}

impl ModInfo {
    pub fn to_list_box_row(&self) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        
        // Set box margins and make it expand
        box_.set_margin_start(12);
        box_.set_margin_end(12);
        box_.set_margin_top(8);
        box_.set_margin_bottom(8);
        box_.set_hexpand(true);
        box_.set_vexpand(true);

        // Name label with ellipsization
        let name_label = gtk::Label::new(Some(&self.name));
        name_label.set_xalign(0.0);
        name_label.set_hexpand(true);
        name_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name_label.set_width_chars(30);

        // Version label
        let version_label = gtk::Label::new(Some(&self.version));
        version_label.set_width_chars(10);
        version_label.set_xalign(0.5);

        // Author label
        let author_label = gtk::Label::new(Some(&self.author));
        author_label.set_width_chars(15);
        author_label.set_xalign(0.5);
        
        // Enable switch
        let enable_switch = gtk::Switch::new();
        enable_switch.set_active(self.enabled);
        enable_switch.set_valign(gtk::Align::Center);

        // Add widgets to box
        box_.append(&name_label);
        box_.append(&version_label);
        box_.append(&author_label);
        box_.append(&enable_switch);

        // Keep existing switch functionality
        if let Some(path) = self.installed_path.clone() {
            let path = path.clone();
            enable_switch.connect_state_set(move |switch, state| {
                eprintln!("Toggling mod state: {:?} -> {}", path, state);
                
                let settings = Settings::load();
                if let Ok(mod_manager) = ModManager::new(settings.clone()) {
                    // Convert to absolute path if needed
                    let absolute_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        settings.game_path.clone()
                            .unwrap_or_else(|| PathBuf::new())
                            .join(&path)
                    };
                    
                    let result = if state {
                        mod_manager.enable_mod(&absolute_path)
                    } else {
                        mod_manager.disable_mod(&absolute_path)
                    };

                    if let Err(e) = result {
                        eprintln!("Failed to toggle mod state: {} (path: {:?})", e, absolute_path);
                        switch.set_active(!state);
                    }
                }
                glib::Propagation::Stop
            });
        }

        row.set_child(Some(&box_));
        row.set_selectable(true);
        row.set_activatable(true);
        row.set_can_focus(true);
        
        row
    }
} 