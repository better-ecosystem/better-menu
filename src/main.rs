pub mod config;
pub mod logger;

use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use evalexpr::eval;
use gio::ApplicationFlags;
use glib::ExitCode;
use gtk::{
    Align, Box as GtkBox, Entry, EventControllerKey, Image, Label, ListBox, Orientation,
    PolicyType, ScrolledWindow, SelectionMode,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use xdg::BaseDirectories;

use crate::config::{load_css, setup_layer_shell};
use lazy_static::lazy_static;
use crate::logger::{Logger, LogLevel};

lazy_static! {
    static ref LOG: Logger = Logger::new(LogLevel::Debug);
}

fn scroll_to_selected(list_box: &ListBox, scrolled_window: &ScrolledWindow) {
    if let Some(selected_row) = list_box.selected_row() {
        let adjustment = scrolled_window.vadjustment();
        
        if let (Some(row_bounds), Some(scrolled_bounds)) = (
            selected_row.compute_bounds(list_box),
            scrolled_window.compute_bounds(scrolled_window)
        ) {
            let row_top = row_bounds.y() as f64;
            let row_bottom = (row_bounds.y() + row_bounds.height()) as f64;
            let visible_top = adjustment.value();
            let visible_bottom = visible_top + scrolled_bounds.height() as f64;
            
            if row_top < visible_top {
                adjustment.set_value(row_top);
            } else if row_bottom > visible_bottom {
                adjustment.set_value(row_bottom - scrolled_bounds.height() as f64);
            }
        }
    }
}

fn evaluate_math_expression(expression: &str) -> Option<String> {
    let expr = if expression.contains('/') {
        expression.replace('/', " / ").split_whitespace().map(|token| {
            if token.parse::<i64>().is_ok() && !token.contains('.') {
                format!("{}.0", token)
            } else {
                token.to_string()
            }
        }).collect::<Vec<String>>().join(" ").replace(" / ", "/")
    } else {
        expression.to_string()
    };
    
    match eval(&expr) {
        Ok(result) => Some(result.to_string()),
        Err(_) => None,
    }
}

fn create_icon_image(icon_name: &str) -> Option<Image> {
    if let Some(display) = gtk::gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        if icon_theme.has_icon(icon_name) {
            let icon = Image::from_icon_name(icon_name);
            icon.set_icon_size(gtk::IconSize::Large);
            icon.set_margin_start(5);
            return Some(icon);
        }
    }
    None
}

fn create_math_result_item(expression: &str, result: &str) -> GtkBox {
    let item_box = GtkBox::new(Orientation::Horizontal, 5);

    if let Some(icon) = create_icon_image("accessories-calculator") {
        item_box.append(&icon);
    }

    let label = Label::new(Some(&format!("{} = {}", expression, result)));
    label.set_halign(Align::Start);
    label.set_margin_start(10);
    label.set_margin_end(10);
    label.set_margin_top(5);
    label.set_margin_bottom(5);
    item_box.append(&label);

    item_box
}

fn copy_to_clipboard(text: &str) {
    if let Some(display) = gtk::gdk::Display::default() {
        let clipboard = display.clipboard();
        clipboard.set_text(text);
    }
}
#[tokio::main]
async fn main() -> ExitCode {
    let app = Application::builder()
        .application_id("com.better-ecosystem.launcher")
        .flags(ApplicationFlags::default())
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let exec_commands: Rc<RefCell<HashMap<String, String>>> = Rc::new(RefCell::new(HashMap::new()));
    let math_results: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Better Launcher")
        .default_width(600)
        .default_height(400)
        .build();

    setup_layer_shell(&window);
    load_css();

    window.set_resizable(false);
    window.set_decorated(false);
    window.set_modal(true);
    window.set_deletable(false);
    window.set_widget_name("launcher");

    window.set_default_size(600, 400);


    let main_box = GtkBox::new(Orientation::Vertical, 0);

    let entry = Entry::builder()
        .placeholder_text("Search applications...")
        .margin_top(15)
        .margin_bottom(15)
        .margin_start(15)
        .margin_end(15)
        .build();

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .build();

    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::Single);

    scrolled_window.set_child(Some(&list_box));

    main_box.append(&entry);
    main_box.append(&scrolled_window);
    scrolled_window.set_vexpand(true);

    window.set_content(Some(&main_box));

    entry.connect_activate(glib::clone!(@weak window, @weak list_box, @strong exec_commands, @strong math_results => move |entry| {
        if let Some(selected_row) = list_box.selected_row() {
            if let Some(item_box) = selected_row.child().as_ref().and_then(|child| child.downcast_ref::<GtkBox>()) {
                if let Some(label) = item_box.last_child().as_ref().and_then(|child| child.downcast_ref::<Label>()) {
                    let label_text = label.text().to_string();

                    if label_text.contains(" = ") {
                        let result = label_text.split(" = ").last().unwrap_or("").to_string();
                        copy_to_clipboard(&result);
                        window.close();
                    } else {
                        if let Some(exec_command) = exec_commands.borrow().get(&label_text) {
                            launch_application(exec_command);
                            window.close();
                        }
                    }
                }
            }
        } else {
            let query = entry.text().to_string();
            if let Some(result) = evaluate_math_expression(&query) {
                copy_to_clipboard(&result);
                window.close();
            } else if let Some(first_row) = list_box.row_at_index(0) {
                list_box.select_row(Some(&first_row));
                if let Some(item_box) = first_row.child().as_ref().and_then(|child| child.downcast_ref::<GtkBox>()) {
                    if let Some(label) = item_box.last_child().as_ref().and_then(|child| child.downcast_ref::<Label>()) {
                        let label_text = label.text().to_string();

                        if label_text.contains(" = ") {
                            let result = label_text.split(" = ").last().unwrap_or("").to_string();
                            copy_to_clipboard(&result);
                            window.close();
                        } else {
                            if let Some(exec_command) = exec_commands.borrow().get(&label_text) {
                                launch_application(exec_command);
                                window.close();
                            }
                        }
                    }
                }
            }
        }
    }));

    load_desktop_entries(&list_box, &exec_commands, &window);

    let list_box_clone = list_box.clone();
    let scrolled_window_clone = scrolled_window.clone();
    let math_results_clone = math_results.clone();
    entry.connect_changed(move |entry| {
        let query = entry.text().to_lowercase();

        while let Some(row) = list_box_clone.row_at_index(0) {
            if let Some(item_box) = row.child().as_ref().and_then(|child| child.downcast_ref::<GtkBox>()) {
                if let Some(label) = item_box.last_child().as_ref().and_then(|child| child.downcast_ref::<Label>()) {
                    let label_text = label.text().to_string();
                    if label_text.contains(" = ") {
                        list_box_clone.remove(&row);
                        continue;
                    }
                }
            }
            break;
        }

        if query.is_empty() {
            list_box_clone.unset_filter_func();
            if let Some(first_row) = list_box_clone.row_at_index(0) {
                list_box_clone.select_row(Some(&first_row));
                scroll_to_selected(&list_box_clone, &scrolled_window_clone);
            }
        } else {
            if let Some(result) = evaluate_math_expression(&query) {
                let math_item = create_math_result_item(&query, &result);
                list_box_clone.prepend(&math_item);
                math_results_clone.borrow_mut().clear();
                math_results_clone.borrow_mut().push(result);
            }

            let query_clone = query.clone();
            let query_for_idle = query.clone();
            list_box_clone.set_filter_func(move |row| {
                if let Some(item_box) = row.child().as_ref().and_then(|child| child.downcast_ref::<GtkBox>()) {
                    if let Some(label) = item_box.last_child().as_ref().and_then(|child| child.downcast_ref::<Label>()) {
                        let label_text = label.text().to_string();
                        if label_text.contains(" = ") {
                            return true;
                        }
                        let app_name = label_text.to_lowercase();
                        return app_name.contains(&query_clone);
                    }
                }
                false
            });

            glib::idle_add_local_once(glib::clone!(@weak list_box_clone, @weak scrolled_window_clone => move || {
                let mut index = 0;
                loop {
                    if let Some(row) = list_box_clone.row_at_index(index) {
                        if let Some(item_box) = row.child().as_ref().and_then(|child| child.downcast_ref::<GtkBox>()) {
                            if let Some(label) = item_box.last_child().as_ref().and_then(|child| child.downcast_ref::<Label>()) {
                                let label_text = label.text().to_string();
                                if label_text.contains(" = ") || label_text.to_lowercase().contains(&query_for_idle) {
                                    list_box_clone.select_row(Some(&row));
                                    scroll_to_selected(&list_box_clone, &scrolled_window_clone);
                                    break;
                                }
                            }
                        }
                        index += 1;
                    } else {
                        break;
                    }
                }
            }));
        }
    });

    let escape_controller = EventControllerKey::new();
    escape_controller.connect_key_pressed(glib::clone!(@weak window => @default-return glib::Propagation::Proceed, move |_, key, _code, _state| {
        if key == gtk::gdk::Key::Escape {
            window.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    }));
    window.add_controller(escape_controller);

    let entry_nav_controller = EventControllerKey::new();
    entry_nav_controller.connect_key_pressed(glib::clone!(@weak list_box, @weak scrolled_window => @default-return glib::Propagation::Proceed, move |_, key, _code, _state| {
        match key {
            gtk::gdk::Key::Down => {
                if let Some(selected_row) = list_box.selected_row() {
                    let index = selected_row.index();
                    if let Some(next_row) = list_box.row_at_index(index + 1) {
                        list_box.select_row(Some(&next_row));
                        scroll_to_selected(&list_box, &scrolled_window);
                    }
                } else {
                    if let Some(first_row) = list_box.row_at_index(0) {
                        list_box.select_row(Some(&first_row));
                        scroll_to_selected(&list_box, &scrolled_window);
                    }
                }
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Up => {
                if let Some(selected_row) = list_box.selected_row() {
                    let index = selected_row.index();
                    if index > 0 {
                        if let Some(prev_row) = list_box.row_at_index(index - 1) {
                            list_box.select_row(Some(&prev_row));
                            scroll_to_selected(&list_box, &scrolled_window);
                        }
                    }
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed
        }
    }));
    entry.add_controller(entry_nav_controller);

    window.present();
    window.set_focus_visible(true);

    entry.grab_focus();
}

fn load_desktop_entries(list_box: &ListBox, exec_commands: &Rc<RefCell<HashMap<String, String>>>, window: &ApplicationWindow) {
    let xdg_dirs = BaseDirectories::new();
    let mut desktop_files = Vec::new();

    let data_home_path = xdg_dirs.get_data_home();
    collect_desktop_files(data_home_path.unwrap().join("applications"), &mut desktop_files);

    for data_dir in xdg_dirs.get_data_dirs() {
        collect_desktop_files(data_dir.join("applications"), &mut desktop_files);
    }

    desktop_files.sort();
    desktop_files.dedup();

    for file_path in desktop_files {
        if let Some((app_name, icon_name, exec_command)) = parse_desktop_file(&file_path) {
            let item_box = GtkBox::new(Orientation::Horizontal, 5);

            if let Some(icon) = create_icon_image(&icon_name) {
                item_box.append(&icon);
            }

            let label = Label::new(Some(&app_name));
            label.set_halign(Align::Start);
            label.set_margin_start(10);
            label.set_margin_end(10);
            label.set_margin_top(5);
            label.set_margin_bottom(5);
            item_box.append(&label);

            exec_commands.borrow_mut().insert(app_name.clone(), exec_command);

            list_box.append(&item_box);
        }
    }

    list_box.connect_row_activated(glib::clone!(@weak window, @strong exec_commands => move |_, row| {
        if let Some(item_box) = row.child().as_ref().and_then(|child| child.downcast_ref::<GtkBox>()) {
            if let Some(label) = item_box.last_child().as_ref().and_then(|child| child.downcast_ref::<Label>()) {
                let label_text = label.text().to_string();
                
                if label_text.contains(" = ") {
                    let result = label_text.split(" = ").last().unwrap_or("").to_string();
                    copy_to_clipboard(&result);
                    window.close();
                } else {
                    if let Some(exec_command) = exec_commands.borrow().get(&label_text) {
                        launch_application(exec_command);
                        window.close();
                    }
                }
            }
        }
    }));

    if let Some(first_row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first_row));
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

fn parse_desktop_file(file_path: &PathBuf) -> Option<(String, String, String)> {
    let content = fs::read_to_string(file_path).ok()?;
    let mut name: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut exec: Option<String> = None;
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
        if line.starts_with("Exec=") && exec.is_none() {
            exec = Some(line.trim_start_matches("Exec=").to_string());
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

    match (name, icon, exec) {
        (Some(n), Some(i), Some(e)) => Some((n, i, e)),
        _ => None,
    }
}

fn launch_application(exec_command: &str) {
    let cleaned_command = clean_exec_command(exec_command).to_string();

    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let parts: Vec<&str> = cleaned_command.split_whitespace().collect();
            if let Some(program) = parts.first() {
                let args = &parts[1..];
                if let Err(e) = Command::new(program).args(args).spawn() {
                    LOG.error(&format!("Failed to launch application: {}", e));
                }
            }
        }).await.unwrap_or_else(|e| {
            LOG.error(&format!("Failed to spawn blocking task: {}", e));
        });
    });
}


fn clean_exec_command(exec: &str) -> String {
    exec.replace("%f", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%U", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .trim()
        .to_string()
}
