use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar};
use gio::ApplicationFlags;
use glib::ExitCode;
use gtk::{
    Box as GtkBox, Entry, Orientation, ListBox, ScrolledWindow, PolicyType, SelectionMode, Label, Align, Image, EventControllerKey
};
use std::fs;
use std::path::PathBuf;
use xdg::BaseDirectories;

fn main() -> ExitCode {
    let app = Application::builder()
        .application_id("com.example.bettermenu")
        .flags(ApplicationFlags::default())
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Better Menu")
        .default_width(600)
        .default_height(400)
        .build();

    let header_bar = HeaderBar::new();

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.prepend(&header_bar);

    let entry = Entry::builder()
        .placeholder_text("Search applications...")
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .build();

    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::Single);

    scrolled_window.set_child(Some(&list_box));

    main_box.append(&entry);
    main_box.append(&scrolled_window);
    scrolled_window.set_vexpand(true);

    window.set_content(Some(&main_box));

    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(glib::clone!(@weak window => @default-return glib::Propagation::Proceed, move |_, key, _code, _state| {
        if key == gtk::gdk::Key::Escape {
            window.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    }));
    window.add_controller(key_controller);

    load_desktop_entries(&list_box);

    entry.connect_changed(move |entry| {
        let query = entry.text().to_lowercase();
        filter_entries(&list_box, &query);
    });

    window.present();
}

fn load_desktop_entries(list_box: &ListBox) {
    let xdg_dirs = BaseDirectories::new().unwrap();
    let mut desktop_files = Vec::new();

    let data_home_path = xdg_dirs.get_data_home();
    collect_desktop_files(data_home_path.join("applications"), &mut desktop_files);

    for data_dir in xdg_dirs.get_data_dirs() {
        collect_desktop_files(data_dir.join("applications"), &mut desktop_files);
    }

    desktop_files.sort();
    desktop_files.dedup();

    for file_path in desktop_files {
        if let Some((app_name, icon_name)) = parse_desktop_file(&file_path) {
            let item_box = GtkBox::new(Orientation::Horizontal, 5);

            let icon = Image::from_icon_name(&icon_name);
            icon.set_icon_size(gtk::IconSize::Large);
            icon.set_margin_start(5);
            item_box.append(&icon);

            let label = Label::new(Some(&app_name));
            label.set_halign(Align::Start);
            label.set_margin_start(10);
            label.set_margin_end(10);
            label.set_margin_top(5);
            label.set_margin_bottom(5);
            item_box.append(&label);
            
            list_box.append(&item_box);
        }
    }
}

fn collect_desktop_files(dir: PathBuf, desktop_files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "desktop") {
                desktop_files.push(path);
            } else if path.is_dir() {
                collect_desktop_files(path, desktop_files); 
            }
        }
    }
}

fn parse_desktop_file(file_path: &PathBuf) -> Option<(String, String)> {
    let content = fs::read_to_string(file_path).ok()?;
    let mut name: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut no_display = false;
    let mut hidden = false;
    let mut app_type: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("Name=") && name.is_none() {
            name = Some(line.trim_start_matches("Name=").to_string());
        }
        if line.starts_with("Icon=") && icon.is_none() {
            icon = Some(line.trim_start_matches("Icon=").to_string());
        }
        if line.starts_with("NoDisplay=") {
            if line.trim_start_matches("NoDisplay=").to_lowercase() == "true" {
                no_display = true;
            }
        }
        if line.starts_with("Hidden=") {
            if line.trim_start_matches("Hidden=").to_lowercase() == "true" {
                hidden = true;
            }
        }
        if line.starts_with("Type=") && app_type.is_none() {
            app_type = Some(line.trim_start_matches("Type=").to_string());
        }
    }

    if no_display || hidden {
        return None;
    }

    if app_type.as_deref() != Some("Application") {
        return None;
    }
    
    name.zip(icon)
}

fn filter_entries(list_box: &ListBox, query: &str) {
    let mut current_row_widget = list_box.first_child();
    while let Some(row_widget) = current_row_widget {
        if let Some(list_box_row) = row_widget.downcast_ref::<gtk::ListBoxRow>() {
            if let Some(item_box) = list_box_row.child().as_ref().and_then(|child_widget_ref| child_widget_ref.downcast_ref::<GtkBox>()) {
                if let Some(label) = item_box.last_child().as_ref().and_then(|child_widget_ref| child_widget_ref.downcast_ref::<Label>()) {
                    let app_name = label.text().to_lowercase();
                    list_box_row.set_visible(app_name.contains(query));
                }
            }
        }
        current_row_widget = row_widget.next_sibling();
    }
}
