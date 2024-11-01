use crate::ui::MainWindow;
use gtk::gio::Settings;
use gtk::glib::Object;
use gtk::prelude::{DialogExt, EntryBufferExtManual, EntryExt, GtkWindowExt, SettingsExt, WidgetExt};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{glib, Accessible, Application, Buildable, ConstraintTarget, Dialog, Native, ResponseType, Root, ShortcutManager, Widget, Window};

mod imp;

glib::wrapper! {
    pub struct ConfigDialog(ObjectSubclass<imp::ConfigDialog>)
    @extends Dialog, Window, Widget,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl ConfigDialog {
    pub fn new(app: &Application, parent: &MainWindow, settings: Settings) -> Self {
        let result: Self = Object::builder()
            .property("application", app)
            .build();
        result.set_transient_for(Some(parent));
        result.set_modal(true);
        let device = settings.string("device");
        if !device.is_empty() {
            result.imp().device_entry.buffer().set_text(device.as_str());
        }
        result.imp().settings.set(settings).expect("Failed to set settings");
        result
    }

    fn initialize(&self) {
        self.connect_response(|dialog, response| {
            dialog.hide();
           if response == ResponseType::Ok {
               let settings = dialog.imp().settings.get().expect("Failed to get settings");
               let device = dialog.imp().device_entry.buffer().text().as_str().to_string();
               settings.set_string("device", &device).expect("Failed to set settings");
           }
            dialog.destroy();
        });
    }
}