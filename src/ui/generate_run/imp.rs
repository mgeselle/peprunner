use gtk::{glib, CompositeTemplate, Dialog, TemplateChild};
use gtk::subclass::prelude::*;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/de/geselle_ffm/peprunner/generate_run_dialog.ui")]
pub struct GenerateRunDialog {
    #[template_child]
    pub filter_u: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub filter_b: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub filter_v: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub filter_r: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub filter_i: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub pgm_entry_1: TemplateChild<gtk::Entry>,
    #[template_child]
    pub pgm_entry_2: TemplateChild<gtk::Entry>,
    #[template_child]
    pub cmp_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    pub chk_entry_1: TemplateChild<gtk::Entry>,
    #[template_child]
    pub chk_entry_2: TemplateChild<gtk::Entry>,
}

#[glib::object_subclass]
impl ObjectSubclass for GenerateRunDialog {
    const NAME: &'static str = "GenerateRunDialog";

    type Type = super::GenerateRunDialog;
    type ParentType = Dialog;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for GenerateRunDialog {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for GenerateRunDialog {}

impl WindowImpl for GenerateRunDialog {}

impl DialogImpl for GenerateRunDialog {}