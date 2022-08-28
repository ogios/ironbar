use crate::icon;
use crate::modules::{Module, ModuleInfo};
use crate::sway::get_client;
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{IconTheme, Image, Label, Orientation};
use serde::Deserialize;
use tokio::task::spawn_blocking;

#[derive(Debug, Deserialize, Clone)]
pub struct FocusedModule {
    /// Whether to show icon on the bar.
    #[serde(default = "crate::config::default_true")]
    show_icon: bool,
    /// Whether to show app name on the bar.
    #[serde(default = "crate::config::default_true")]
    show_title: bool,

    /// Icon size in pixels.
    #[serde(default = "default_icon_size")]
    icon_size: i32,
    /// GTK icon theme to use.
    icon_theme: Option<String>,
}

const fn default_icon_size() -> i32 {
    32
}

impl Module<gtk::Box> for FocusedModule {
    fn into_widget(self, _info: &ModuleInfo) -> Result<gtk::Box> {
        let icon_theme = IconTheme::new();

        if let Some(theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(&theme));
        }

        let container = gtk::Box::new(Orientation::Horizontal, 5);

        let icon = Image::builder().name("icon").build();
        let label = Label::builder().name("label").build();

        container.add(&icon);
        container.add(&label);

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let focused = {
            let sway = get_client();
            let mut sway = sway.lock().expect("Failed to get lock on Sway IPC client");
            sway.get_open_windows()?
                .into_iter()
                .find(|node| node.focused)
        };

        if let Some(focused) = focused {
            tx.send(focused)?;
        }

        spawn_blocking(move || {
            let srx = {
                let sway = get_client();
                let mut sway = sway.lock().expect("Failed to get lock on Sway IPC client");
                sway.subscribe_window()
            };

            while let Ok(payload) = srx.recv() {
                let update = match payload.change.as_str() {
                    "focus" => true,
                    "title" => payload.container.focused,
                    _ => false,
                };

                if update {
                    tx.send(payload.container)
                        .expect("Failed to sendf focus update");
                }
            }
        });

        {
            rx.attach(None, move |node| {
                let value = node.name.as_deref().unwrap_or_else(|| node.get_id());

                let pixbuf = icon::get_icon(&icon_theme, node.get_id(), self.icon_size);

                if self.show_icon {
                    icon.set_pixbuf(pixbuf.as_ref());
                }

                if self.show_title {
                    label.set_label(value);
                }

                Continue(true)
            });
        }

        Ok(container)
    }
}
