use std::cell::OnceCell;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Dialog, TemplateChild};
use gtk::gio::Settings;
use gtk::glib::subclass::InitializingObject;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/de/geselle_ffm/peprunner/config_dialog.ui")]
pub struct ConfigDialog {
    #[template_child]
    pub device_entry: TemplateChild<gtk::Entry>,
    pub settings: OnceCell<Settings>,
}

#[glib::object_subclass]
impl ObjectSubclass for ConfigDialog {
    const NAME: &'static str = "ConfigDialog";
    type Type = super::ConfigDialog;
    type ParentType = Dialog;

    fn class_init(_klass: &mut Self::Class) {
        _klass.bind_template();
    }

    fn instance_init(_obj: &InitializingObject<Self>) {
        _obj.init_template();
    }
}

impl ObjectImpl for ConfigDialog {
    fn constructed(&self) {
        self.parent_constructed();

        self.obj().initialize();
    }
}

impl WidgetImpl for ConfigDialog {}

impl WindowImpl for ConfigDialog {}

impl DialogImpl for ConfigDialog {}