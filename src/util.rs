use std::fmt::Display;
use gtk::prelude::{DialogExt, GtkWindowExt, IsA, WidgetExt};
use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType, Window};

pub fn show_error<E: Display>(parent: Option<&impl IsA<Window>>, title: Option<&str>, error: E) {
    let msg_dialog = MessageDialog::new(
        parent,
        DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Ok,
        format!("{error}"));
    msg_dialog.set_title(title);
    msg_dialog.connect_response(|dlg, _| dlg.destroy());
    msg_dialog.show()
}