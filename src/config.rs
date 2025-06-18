use adw::ApplicationWindow;
use gtk::CssProvider;
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use lazy_static::lazy_static;
use crate::logger::{Logger, LogLevel};

lazy_static! {
    static ref LOG: Logger = Logger::new(LogLevel::Debug);
}

pub fn setup_layer_shell(window: &ApplicationWindow) {
    LayerShell::init_layer_shell(window);
    LayerShell::set_layer(window, Layer::Overlay);
    LayerShell::set_keyboard_mode(window, KeyboardMode::Exclusive);
}

pub fn load_css() {
    let css = "
        window{
            border-radius: 12px;
        }
    ";

    let css_provider = CssProvider::new();
    css_provider.load_from_string(css);
    gtk::gdk::Display::default()
        .map_or_else(
            || LOG.error("Failed to get default display"),
            |display| {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &css_provider,
                    gtk::STYLE_PROVIDER_PRIORITY_USER,
                );
                LOG.debug("CSS provider added to display");
            }
        );
}
