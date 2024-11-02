use gtk::{glib, Accessible, Application, Buildable, ConstraintTarget, Dialog, Native, Root, ShortcutManager, Widget, Window};
use gtk::glib::Object;
use gtk::prelude::{CheckButtonExt, EntryBufferExtManual, EntryExt, GtkWindowExt};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use crate::common::{PepRun, StarData};
use crate::ui::{FilterSettings, MainWindow};

mod imp;

glib::wrapper! {
    pub struct GenerateRunDialog(ObjectSubclass<imp::GenerateRunDialog>)
    @extends Dialog, Window, Widget,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl GenerateRunDialog {
    pub fn new(app: &Application, parent: &MainWindow, filter_settings: &FilterSettings) -> Self {
        let result: Self = Object::builder()
            .property("application", app)
            .build();
        result.set_transient_for(Some(parent));
        result.set_modal(true);
        result.imp().filter_u.set_active(filter_settings.u);
        result.imp().filter_b.set_active(filter_settings.b);
        result.imp().filter_v.set_active(filter_settings.v);
        result.imp().filter_r.set_active(filter_settings.r);
        result.imp().filter_i.set_active(filter_settings.i);
        result
    }

    pub fn get_run(&self) -> Option<PepRun> {
        let imp = self.imp();
        let pgm1 = imp.pgm_entry_1.buffer().text().to_string();
        let pgm2 = imp.pgm_entry_2.buffer().text().to_string();
        let cmp = imp.cmp_entry.buffer().text().to_string();
        let chk1 = imp.chk_entry_1.buffer().text().to_string();
        let chk2 = imp.chk_entry_2.buffer().text().to_string();

        if pgm1.is_empty() || cmp.is_empty() || chk1.is_empty() {
            return None;
        }

        let mut filters = Vec::with_capacity(5);
        if imp.filter_u.is_active() {
            filters.push(0);
        }
        if imp.filter_b.is_active() {
            filters.push(1);
        }
        if imp.filter_v.is_active() {
            filters.push(2);
        }
        if imp.filter_r.is_active() {
            filters.push(3);
        }
        if imp.filter_i.is_active() {
            filters.push(4);
        }

        let mut stars = Vec::with_capacity(13);
        let cmp_str = "CMP".to_string();
        let chk_str = "CHK".to_string();
        let pgm_str = "PGM".to_string();
        for _ in 0..3 {
            stars.push(StarData::new(&cmp_str, &cmp));
            stars.push(StarData::new(&pgm_str, &pgm1));
            if ! pgm2.is_empty() {
                stars.push(StarData::new(&pgm_str, &pgm2));
            }
        }
        stars.push(StarData::new(&cmp_str, &cmp));
        stars.push(StarData::new(&chk_str, &chk1));
        if ! chk2.is_empty() {
            stars.push(StarData::new(&chk_str, &chk2));
        }
        stars.push(StarData::new(&cmp_str, &cmp));

        Some(PepRun::new(filters, stars))
    }
}