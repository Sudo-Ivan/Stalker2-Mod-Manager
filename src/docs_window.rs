use gtk::prelude::*;
use gtk::{Window, ScrolledWindow, Box, Label, Picture, Orientation};
use std::fs;

pub fn show_docs_window(parent: &impl IsA<Window>) {
    let window = Window::builder()
        .title("Documentation")
        .transient_for(parent)
        .modal(true)
        .default_width(800)
        .default_height(600)
        .build();

    window.connect_close_request(move |window| {
        window.destroy();
        glib::Propagation::Stop
    });

    let scrolled = ScrolledWindow::new();
    let content_box = Box::new(Orientation::Vertical, 12);
    content_box.set_margin_start(24);
    content_box.set_margin_end(24);
    content_box.set_margin_top(24);
    content_box.set_margin_bottom(24);

    // Load and parse markdown content
    let content = fs::read_to_string("docs/homepage.md")
        .unwrap_or_else(|_| String::from("# Documentation not found\n\nPlease ensure the docs folder exists."));
    
    let parser = pulldown_cmark::Parser::new(&content);
    let mut current_list: Option<Box> = None;
    let mut list_counter = 0;
    let mut current_text = String::new();
    let mut in_heading = false;

    for event in parser {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading(level, _, _)) => {
                in_heading = true;
                let label = Label::new(None);
                match level {
                    pulldown_cmark::HeadingLevel::H1 => label.add_css_class("title-1"),
                    pulldown_cmark::HeadingLevel::H2 => label.add_css_class("title-2"),
                    pulldown_cmark::HeadingLevel::H3 => label.add_css_class("title-3"),
                    _ => {}
                }
                label.set_wrap(true);
                label.set_wrap_mode(gtk::pango::WrapMode::Word);
                label.set_xalign(0.0);
                label.set_margin_top(12);
                content_box.append(&label);
                current_text.clear();
            }
            pulldown_cmark::Event::End(pulldown_cmark::Tag::Heading(_, _, _)) => {
                in_heading = false;
                if let Some(last_widget) = content_box.last_child() {
                    if let Some(label) = last_widget.downcast_ref::<Label>() {
                        label.set_text(&current_text);
                    }
                }
                current_text.clear();
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::List(first_item_number)) => {
                let list_box = Box::new(Orientation::Vertical, 6);
                list_box.set_margin_start(24);
                list_box.set_margin_top(8);
                if let Some(n) = first_item_number {
                    list_counter = n;
                } else {
                    list_counter = 0;
                }
                current_list = Some(list_box);
                current_text.clear();
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Item) => {
                if let Some(list_box) = &current_list {
                    let item_box = Box::new(Orientation::Horizontal, 6);
                    let bullet = if list_counter > 0 {
                        let bullet = format!("{}.", list_counter);
                        list_counter += 1;
                        bullet
                    } else {
                        "•".to_string()
                    };
                    
                    let bullet_label = Label::new(Some(&bullet));
                    bullet_label.set_xalign(0.0);
                    item_box.append(&bullet_label);
                    
                    let text_label = Label::new(None);
                    text_label.set_wrap(true);
                    text_label.set_wrap_mode(gtk::pango::WrapMode::Word);
                    text_label.set_xalign(0.0);
                    text_label.set_hexpand(true);
                    item_box.append(&text_label);
                    
                    list_box.append(&item_box);
                    current_text.clear();
                }
            }
            pulldown_cmark::Event::Text(text) => {
                current_text.push_str(&text);
                if in_heading {
                    if let Some(last_widget) = content_box.last_child() {
                        if let Some(label) = last_widget.downcast_ref::<Label>() {
                            label.set_text(&current_text);
                        }
                    }
                } else if let Some(list_box) = &current_list {
                    if let Some(item_box) = list_box.last_child() {
                        if let Some(text_label) = item_box.last_child() {
                            if let Some(label) = text_label.downcast_ref::<Label>() {
                                label.set_text(&current_text);
                            }
                        }
                    }
                } else {
                    let label = Label::new(Some(&text));
                    label.set_wrap(true);
                    label.set_wrap_mode(gtk::pango::WrapMode::Word);
                    label.set_xalign(0.0);
                    content_box.append(&label);
                }
            }
            pulldown_cmark::Event::End(pulldown_cmark::Tag::Item) => {
                current_text.clear();
            }
            pulldown_cmark::Event::End(pulldown_cmark::Tag::List(_)) => {
                if let Some(list_box) = current_list.take() {
                    content_box.append(&list_box);
                }
                list_counter = 0;
                current_text.clear();
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image(_, path, _)) => {
                let picture = Picture::for_filename(path.as_ref());
                picture.set_can_shrink(true);
                picture.set_margin_top(12);
                picture.set_margin_bottom(12);
                content_box.append(&picture);
            }
            _ => {}
        }
    }

    scrolled.set_child(Some(&content_box));
    window.set_child(Some(&scrolled));

    let css_provider = gtk::CssProvider::new();
    let css = "
        .title-1 { 
            font-size: 24px; 
            font-weight: bold; 
            margin-bottom: 16px;
            color: @theme_fg_color;
        }
        .title-2 { 
            font-size: 20px; 
            font-weight: bold; 
            margin-bottom: 12px;
            color: @theme_fg_color;
        }
        .title-3 { 
            font-size: 16px; 
            font-weight: bold; 
            margin-bottom: 8px;
            color: @theme_fg_color;
        }
        label { 
            margin-bottom: 8px;
            color: @theme_fg_color;
        }
        slider {
            min-width: 1px;
            min-height: 1px;
        }
        
        * {
            -gtk-icon-theme: 'Adwaita';
            cursor-size: 24;
        }
    ";
    css_provider.load_from_data(css);

    window.style_context().add_provider(
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    window.present();
} 