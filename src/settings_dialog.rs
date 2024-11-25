use gtk::prelude::*;
use gtk::{Dialog, Box, Label, Entry, Switch, ResponseType, Orientation, Button, FileChooserDialog, FileChooserAction};
use crate::settings::Settings;

pub fn show_settings_dialog(parent: &impl IsA<gtk::Window>) {
    let dialog = Dialog::builder()
        .title("Settings")
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

    // Game path selection
    let path_box = Box::new(Orientation::Horizontal, 12);
    let path_label = Label::new(Some("Game Path:"));
    let path_button = Button::with_label("Select Game Path");
    let path_display = Label::new(None);
    path_box.append(&path_label);
    path_box.append(&path_button);
    path_box.append(&path_display);
    content.append(&path_box);

    // Nexus API key
    let api_box = Box::new(Orientation::Horizontal, 12);
    let api_label = Label::new(Some("Nexus API Key:"));
    let api_entry = Entry::new();
    api_entry.set_input_purpose(gtk::InputPurpose::Password);
    api_box.append(&api_label);
    api_box.append(&api_entry);
    content.append(&api_box);

    // Dark theme toggle
    let theme_box = Box::new(Orientation::Horizontal, 12);
    let theme_label = Label::new(Some("Dark Theme:"));
    let theme_switch = Switch::new();
    theme_box.append(&theme_label);
    theme_box.append(&theme_switch);
    content.append(&theme_box);

    // Load current settings
    let settings = Settings::load();
    if let Some(path) = settings.game_path.as_ref() {
        path_display.set_text(&path.to_string_lossy());
    }
    if let Some(key) = settings.nexus_api_key.as_ref() {
        api_entry.set_text(key);
    }
    theme_switch.set_active(settings.dark_theme);

    // Setup file chooser dialog
    path_button.connect_clicked(glib::clone!(@weak dialog, @weak path_display => move |_| {
        let file_chooser = FileChooserDialog::new(
            Some("Select Game Path"),
            Some(&dialog),
            FileChooserAction::SelectFolder,
            &[("Cancel", ResponseType::Cancel), ("Select", ResponseType::Accept)]
        );

        file_chooser.connect_response(glib::clone!(@weak path_display => move |file_chooser, response| {
            if response == ResponseType::Accept {
                if let Some(path) = file_chooser.file().and_then(|file| file.path()) {
                    path_display.set_text(&path.to_string_lossy());
                }
            }
            file_chooser.close();
        }));

        file_chooser.show();
    }));

    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Accept);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let mut settings = Settings::load();
            let path_str = path_display.text();
            settings.game_path = Some(std::path::PathBuf::from(path_str.as_str()));
            settings.nexus_api_key = Some(api_entry.text().to_string());
            settings.dark_theme = theme_switch.is_active();
            settings.save().unwrap();

            // Apply dark theme immediately
            let gtk_settings = gtk::Settings::default().unwrap();
            gtk_settings.set_gtk_application_prefer_dark_theme(settings.dark_theme);
        }
        dialog.close();
    });

    dialog.present();
} 