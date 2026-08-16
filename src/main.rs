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
    install_panic_log();
    if !tray::claim_single_instance() {
        return;
    }
    if let Err(err) = run() {
        tray::message_box(&format!("{err}"));
    }
}

fn install_panic_log() {
    let path = crate::config::data_dir().join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let text = format!("{info}\n{backtrace}\n");
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(&path, &text);
        crate::tray::message_box(&format!("程序出错，已写入\n{}", path.display()));
    }));
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
