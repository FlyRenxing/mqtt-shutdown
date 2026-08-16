#![windows_subsystem = "windows"]

mod autostart;
mod config;
mod mqtt;
mod power;
mod state;
mod tray;
mod ui;

use windows_reactor::*;

use crate::config::{APP_TITLE, icon_path};

fn main() {
    if !tray::claim_single_instance() {
        return;
    }
    if let Err(err) = run() {
        tray::message_box(&format!("{err}"));
    }
}

fn run() -> Result<()> {
    bootstrap()?;
    let mut app = App::new()
        .title(APP_TITLE)
        .inner_size(920.0, 680.0)
        .inner_constraints(InnerConstraints {
            min_width: Some(760.0),
            min_height: Some(540.0),
            max_width: None,
            max_height: None,
        })
        .backdrop(Backdrop::Mica)
        .on_exit(|| {
            tray::uninstall_hooks();
            mqtt::stop();
        });
    if let Some(icon) = icon_path() {
        app = app.icon(icon.to_string_lossy().into_owned());
    }
    app.render(ui::app)
}
