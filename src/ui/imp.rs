use crate::common::StarData;
use glib::subclass::InitializingObject;
use glib::Properties;
use gtk::glib::Binding;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, ApplicationWindow, CompositeTemplate, FileChooserNative, Label};
use std::cell::{OnceCell, RefCell};
use std::path::PathBuf;
use gtk::gio::Settings;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/de/geselle_ffm/peprunner/main_window.ui")]
pub struct MainWindow {
    #[template_child]
    pub main_menu_mb: TemplateChild<gtk::MenuButton>,
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
    pub star_type_dd: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub star_name_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    pub star_list_vw: TemplateChild<gtk::ListView>,
    #[template_child]
    pub execute_button: TemplateChild<gtk::Button>,
    pub stars: RefCell<Option<gio::ListStore>>,
    pub editing: RefCell<Option<u32>>,
    pub current_file: RefCell<Option<PathBuf>>,
    pub file_dialog: RefCell<Option<FileChooserNative>>,
    pub settings: OnceCell<Settings>,
    pub executing: bool,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWindow {
    const NAME: &'static str = "MainWindow";
    type Type = super::MainWindow;
    type ParentType = ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for MainWindow {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.setup_stars();
        obj.setup_callbacks();
        obj.setup_factory();
        obj.setup_actions();
        obj.setup_settings();
    }
}

impl WidgetImpl for MainWindow {}

impl WindowImpl for MainWindow {}

impl ApplicationWindowImpl for MainWindow {}

#[derive(Properties, Default)]
#[properties(wrapper_type = super::StarObject)]
pub struct StarObject {
    #[property(name = "star-type", get, set, type = String, member = star_type)]
    #[property(name = "name", get, set, type = String, member = name)]
    pub data: RefCell<StarData>,
}

#[glib::object_subclass]
impl ObjectSubclass for StarObject {
    const NAME: &'static str = "PEPStarObject";
    type Type = super::StarObject;
}

#[glib::derived_properties]
impl ObjectImpl for StarObject {}

#[derive(Default, CompositeTemplate)]
#[template(resource = "/de/geselle_ffm/peprunner/star_object_row.ui")]
pub struct StarObjectRow {
    #[template_child]
    pub type_label: TemplateChild<Label>,
    #[template_child]
    pub name_label: TemplateChild<Label>,
    pub bindings: RefCell<Vec<Binding>>,
}

#[glib::object_subclass]
impl ObjectSubclass for StarObjectRow {
    const NAME: &'static str = "StarObjectRow";
    type Type = super::StarObjectRow;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for StarObjectRow {}

impl WidgetImpl for StarObjectRow {}

impl BoxImpl for StarObjectRow {}