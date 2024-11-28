mod settings;
mod mod_manager;
mod mod_info;
mod nexus_api;
mod install_dialog;
mod settings_dialog;
mod docs_window;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, HeaderBar, Button, Box, ScrolledWindow, 
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
use async_channel::bounded;

const APP_ID: &str = "org.stalker2.mod.manager";

fn main() -> glib::ExitCode {
    // Initialize GTK first
    gtk::init().expect("Failed to initialize GTK");

    // Initialize GTK settings more safely
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_cursor_theme_size(24);
    }

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    let (_sender, receiver) = bounded::<()>(1);

    // Set up receiver
    glib::spawn_future_local(async move {
        while receiver.recv().await.is_ok() {
        }
    });

    app.run()
}

fn handle_nxm_link(app: &Application, uri: &str) {
    if let Ok(nxm) = NxmLink::parse(uri) {
        if let Some(window) = app.active_window() {
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

    // Create main container
    let main_box = Box::new(Orientation::Vertical, 0);
    
    // Create scrolled window and list box first
    let scrolled = ScrolledWindow::new();
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

    // Create header bar with buttons
    let header = HeaderBar::new();
    let install_button = Button::with_label("Install Mod");
    let settings_button = Button::from_icon_name("emblem-system-symbolic");
    header.pack_start(&install_button);
    header.pack_end(&settings_button);
    window.set_titlebar(Some(&header));

    // Now connect button handlers after list_box is created
    install_button.connect_clicked(glib::clone!(@weak window, @weak list_box => move |_| {
        show_install_dialog(&window, &list_box);
    }));

    settings_button.connect_clicked(glib::clone!(@weak window => move |_| {
        show_settings_dialog(&window);
    }));

    window.set_child(Some(&main_box));

    // Clone Rc for closures
    let mod_manager_close = Rc::clone(&mod_manager);
    let list_box_close = list_box.clone();

    // Save mods when the window is closed
    window.connect_close_request(move |window| {
        let mut mods = Vec::new();
        let mut row = list_box_close.first_child();
        let mod_manager = mod_manager_close.borrow();
        
        while let Some(widget) = row {
            if let Some(list_box_row) = widget.downcast_ref::<gtk::ListBoxRow>() {
                if let Some(box_) = list_box_row.child().and_downcast::<gtk::Box>() {
                    // Get all children first
                    let mut children = Vec::new();
                    let mut child = box_.first_child();
                    while let Some(widget) = child {
                        children.push(widget.clone());
                        child = widget.next_sibling();
                    }

                    // Process children
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

                    let installed_path = if enabled {
                        Some(mod_manager.mods_path().join(format!("{}.pak", name)))
                    } else {
                        Some(mod_manager.settings().game_path.clone()
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

        drop(mod_manager);  // Release the borrow before mutating
        let _ = mod_manager_close.borrow_mut().save_mod_list(&mods);
        window.destroy();
        glib::Propagation::Stop
    });

    // Enable drag and drop
    let drop_target = gtk::DropTarget::new(gtk::glib::Type::STRING, gtk::gdk::DragAction::COPY);
    drop_target.set_types(&[gtk::glib::Type::STRING, gtk::gio::File::static_type()]);
    
    let list_box_drop = list_box.clone();

    drop_target.connect_drop(move |_, value, _, _| {
        if let Ok(files) = value.get::<gtk::gio::ListModel>() {
            for i in 0..files.n_items() {
                if let Some(file) = files.item(i) {
                    if let Some(file) = file.downcast_ref::<gtk::gio::File>() {
                        if let Some(path) = file.path() {
                            if path.extension().map_or(false, |ext| ext == "pak") {
                                let name = path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown Mod")
                                    .to_string();
                                
                                let mod_info = ModInfo {
                                    name,
                                    version: String::from("1.0"),
                                    author: String::from("Unknown"),
                                    description: String::new(),
                                    nexus_mod_id: None,
                                    installed_path: Some(path.clone()),
                                    enabled: true,
                                };
                                
                                list_box_drop.append(&mod_info.to_list_box_row());
                            }
                        }
                    }
                }
            }
        }
        true
    });

    window.add_controller(drop_target);
    window.present();

    let (sender, receiver) = bounded::<()>(1);

    unsafe {
        window.set_data("refresh_sender", sender);
    }

    glib::spawn_future_local(async move {
        while receiver.recv().await.is_ok() {
            if let Ok(mods) = mod_manager.borrow().load_mod_list() {
                // Clear existing items
                while let Some(child) = list_box.first_child() {
                    list_box.remove(&child);
                }
                
                // Add updated items
                for mod_info in mods {
                    list_box.append(&mod_info.to_list_box_row());
                }
            }
        }
    });
}
