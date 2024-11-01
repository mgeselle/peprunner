mod ui;
mod common;
mod util;
mod ssp3;
mod measurement;

use gtk::prelude::*;
use gtk::{gio, glib, Application};
use ui::MainWindow;

const APP_ID: &str = "de.geselle_ffm.PepRunner";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    // Create new window and present it
    let window = MainWindow::new(app);
    window.present();
}