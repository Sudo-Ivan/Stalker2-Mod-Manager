use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use gtk::prelude::*;
use gtk::glib::{self, clone};
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
        box_.set_margin_start(6);
        box_.set_margin_end(6);
        box_.set_margin_top(6);
        box_.set_margin_bottom(6);

        let name_label = gtk::Label::new(Some(&self.name));
        name_label.set_xalign(0.0);
        name_label.set_hexpand(true);

        let version_label = gtk::Label::new(Some(&self.version));
        let author_label = gtk::Label::new(Some(&self.author));
        
        let enable_switch = gtk::Switch::new();
        enable_switch.set_active(self.enabled);

        if let Some(path) = self.installed_path.clone() {
            enable_switch.connect_state_set(move |_switch, state| {
                let settings = Settings::load();
                if let Ok(mod_manager) = ModManager::new(settings) {
                    if state {
                        let _ = mod_manager.enable_mod(&path);
                    } else {
                        let _ = mod_manager.disable_mod(&path);
                    }
                }
                glib::Propagation::Stop
            });
        }

        box_.append(&name_label);
        box_.append(&version_label);
        box_.append(&author_label);
        box_.append(&enable_switch);

        row.set_child(Some(&box_));
        row
    }
} 