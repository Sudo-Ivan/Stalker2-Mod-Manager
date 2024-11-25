mod settings;
mod mod_manager;
mod mod_info;
mod nexus_api;
mod install_dialog;
mod settings_dialog;

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, HeaderBar, Button, Box, ScrolledWindow, 
         Orientation};
use crate::settings::Settings;
use crate::mod_manager::ModManager;
use crate::install_dialog::{show_install_dialog, show_install_dialog_with_nxm};
use crate::settings_dialog::show_settings_dialog;
use crate::nexus_api::NxmLink;
use std::rc::Rc;
use std::cell::RefCell;
use crate::mod_info::ModInfo;
use std::path::PathBuf;

const APP_ID: &str = "org.stalker2.modmanager";

fn main() -> glib::ExitCode {
    // Initialize the tokio runtime
    let rt = tokio::runtime::Runtime::new().expect("Unable to create Tokio runtime");
    
    // Set the runtime as the default for this thread
    let _guard = rt.enter();

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(build_ui);
    
    // Handle NXM links
    app.connect_open(|app, files, _| {
        for file in files {
            let uri = file.uri().to_string();
            if uri.starts_with("nxm://") {
                handle_nxm_link(app, &uri);
            }
        }
    });

    app.run()
}

fn handle_nxm_link(app: &Application, uri: &str) {
    if let Ok(nxm) = NxmLink::parse(uri) {
        // Get the main window and list box
        if let Some(window) = app.active_window() {
            // Find the list box in the widget hierarchy
            if let Some(main_box) = window.child() {
                if let Some(scrolled) = main_box.first_child() {
                    if let Some(list_box) = scrolled.first_child().and_downcast::<gtk::ListBox>() {
                        show_install_dialog_with_nxm(&window, &list_box, nxm);
                    }
                }
            }
        }
    }
}

fn build_ui(app: &Application) {
    let settings = Settings::load();
    let mod_manager = Rc::new(RefCell::new(ModManager::new(settings.clone()).unwrap()));
    
    let window = ApplicationWindow::builder()
        .application(app)
        .title("S.T.A.L.K.E.R. 2 Mod Manager")
        .default_width(1024)
        .default_height(768)
        .build();

    // Create header bar with buttons
    let header = HeaderBar::new();
    let install_button = Button::with_label("Install Mod");
    let settings_button = Button::from_icon_name("emblem-system-symbolic");
    header.pack_start(&install_button);
    header.pack_end(&settings_button);
    window.set_titlebar(Some(&header));

    // Main container
    let main_box = Box::new(Orientation::Vertical, 0);
    
    // Create scrolled window for mod list
    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    // Create list box for mods
    let list_box = gtk::ListBox::new();
    list_box.set_selection_mode(gtk::SelectionMode::None);
    
    // Load existing mods
    if let Ok(mods) = mod_manager.borrow().load_mod_list() {
        for mod_info in mods {
            list_box.append(&mod_info.to_list_box_row());
        }
    }

    scrolled.set_child(Some(&list_box));
    main_box.append(&scrolled);

    window.set_child(Some(&main_box));

    // Connect signals with mod_manager cloned for each closure
    install_button.connect_clicked(glib::clone!(@strong mod_manager, @weak window, @weak list_box => move |_| {
        show_install_dialog(&window, &list_box);
    }));

    settings_button.connect_clicked(glib::clone!(@strong mod_manager, @weak window => move |_| {
        show_settings_dialog(&window);
    }));

    // Apply dark theme
    let gtk_settings = gtk::Settings::default().unwrap();
    gtk_settings.set_gtk_application_prefer_dark_theme(settings.dark_theme);

    // Save mods when the window is closed
    window.connect_close_request(move |window| {
        let mut mods = Vec::new();
        let mut row = list_box.first_child();
        
        while let Some(widget) = row {
            if let Some(list_box_row) = widget.downcast_ref::<gtk::ListBoxRow>() {
                if let Some(box_) = list_box_row.child().and_downcast::<gtk::Box>() {
                    let mut children = Vec::new();
                    let mut child = box_.first_child();
                    while let Some(widget) = child {
                        children.push(widget.clone());
                        child = widget.next_sibling();
                    }

                    // Get name, version, author from labels
                    let name = children.get(0)
                        .and_then(|w| w.downcast_ref::<gtk::Label>())
                        .map(|l| l.text().to_string())
                        .unwrap_or_default();
                    
                    let version = children.get(1)
                        .and_then(|w| w.downcast_ref::<gtk::Label>())
                        .map(|l| l.text().to_string())
                        .unwrap_or_default();
                    
                    let author = children.get(2)
                        .and_then(|w| w.downcast_ref::<gtk::Label>())
                        .map(|l| l.text().to_string())
                        .unwrap_or_default();
                    
                    let enabled = box_.last_child()
                        .and_downcast::<gtk::Switch>()
                        .map(|s| s.is_active())
                        .unwrap_or_default();

                    // Try to find the mod path based on the name
                    let installed_path = if enabled {
                        Some(mod_manager.borrow().mods_path().join(format!("{}.pak", name)))
                    } else {
                        Some(mod_manager.borrow().settings().game_path.clone()
                            .unwrap_or_else(|| PathBuf::new())
                            .join("Stalker2")
                            .join("ModManager")
                            .join("unloaded_mods")
                            .join(format!("{}.pak", name)))
                    };

                    let mod_info = ModInfo {
                        name,
                        version,
                        author,
                        description: String::new(),
                        nexus_mod_id: None,
                        installed_path,
                        enabled,
                    };
                    mods.push(mod_info);
                }
            }
            row = widget.next_sibling();
        }

        let _ = mod_manager.borrow_mut().save_mod_list(&mods);
        window.destroy();
        glib::Propagation::Stop
    });

    window.present();
}
