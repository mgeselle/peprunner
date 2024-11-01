mod imp;
mod config;
mod generate_run;

use crate::common::{PepRun, StarData};
use crate::{util, APP_ID};
use glib::{clone, Object};
use gtk::gdk::Key;
use gtk::gio::{ActionEntry, Cancellable, File, FileCreateFlags, FileQueryInfoFlags, Settings, FILE_ATTRIBUTE_STANDARD_SIZE};
use gtk::glib::Propagation;
use gtk::prelude::{ActionMapExtManual, ButtonExt, Cast, CastNone, CheckButtonExt, DialogExt, EntryBufferExtManual, EntryExt, FileChooserExt, FileChooserExtManual, FileExt, GtkWindowExt, InputStreamExtManual, ListItemExt, ListModelExt, NativeDialogExt, ObjectExt, OutputStreamExt, SettingsExt, SettingsExtManual, WidgetExt};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gio, glib, Application, EventControllerKey, FileChooserAction, FileChooserNative, ListItem, ResponseType, SignalListItemFactory, SingleSelection, StringList, StringObject, INVALID_LIST_POSITION};
use std::env::var;
use std::path::Path;
use crate::measurement::execute_run;
use crate::ui::config::ConfigDialog;
use crate::ui::generate_run::GenerateRunDialog;
use crate::util::show_error;

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MainWindow {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn stars(&self) -> gio::ListStore {
        self.imp()
            .stars
            .borrow()
            .clone()
            .expect("no stars available")
    }

    fn setup_stars(&self) {
        let model = gio::ListStore::new::<StarObject>();
        self.imp().stars.replace(Some(model));

        let selection_model = SingleSelection::new(Some(self.stars()));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);
        self.imp().star_list_vw.set_model(Some(&selection_model));
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("settings shouldn't be set yet");
        let settings = self.settings();
        self.imp().filter_u.set_active(settings.get("filter-u"));
        self.imp().filter_b.set_active(settings.get("filter-b"));
        self.imp().filter_v.set_active(settings.get("filter-v"));
        self.imp().filter_r.set_active(settings.get("filter-r"));
        self.imp().filter_i.set_active(settings.get("filter-i"));
    }

    fn settings(&self) -> &Settings {
        self.imp()
            .settings
            .get()
            .expect("settings should already be initialized")
    }

    fn new_star(&self) {
        if let Some(star_data) = self.extract_star() {
            let star = StarObject::new(star_data.star_type, star_data.name);
            self.stars().append(&star);
        }
    }

    fn extract_star(&self) -> Option<StarData> {
        let star_type = self.imp()
            .star_type_dd
            .selected_item().expect("something should always be selected in a DropDown")
            .downcast_ref::<StringObject>()
            .expect("selected should be a StringObject")
            .string()
            .to_string();

        let name_buffer = self.imp().star_name_entry.buffer();
        let star_name = name_buffer.text().to_string().trim().to_string();
        if star_name.is_empty() {
            return None;
        }
        name_buffer.set_text("");

        Some(StarData {star_type, name: star_name })
    }

    fn update_star(&self, pos: u32) {
        if let Some(star_data) = self.extract_star() {
            let star_object = self.imp().star_list_vw
                .model()
                .unwrap()
                .item(pos)
                .and_downcast::<StarObject>()
                .expect("item should be a StarObject");

            star_object.set_property("star-type", &star_data.star_type);
            star_object.set_property("name", &star_data.name);
        }
    }

    fn select_star(&self, star: &StarData) {
        self.imp().star_name_entry.buffer().set_text(&star.name);
        self.imp().star_name_entry.set_placeholder_text(None);
        let type_list = self.imp().star_type_dd
            .model()
            .unwrap()
            .downcast::<StringList>()
            .expect("dropdown model should be a StringList");
        let mut pos = 0;
        while let Some(gstring) = type_list.string(pos) {
            if gstring.to_string().eq(&star.star_type) {
                self.imp().star_type_dd.set_selected(pos);
                break;
            }
            pos += 1;
        }
    }

    fn handle_escape(&self) {
        if let Some(_) = self.imp().editing.replace(None) {
            self.imp().star_name_entry.buffer().set_text("");
        }
    }

    fn remove_star(&self, key: &Key) -> Propagation {
        if *key != Key::Delete && *key != Key::BackSpace {
            return Propagation::Proceed
        }
        let selection = self.imp().star_list_vw
            .model()
            .unwrap()
            .downcast::<SingleSelection>()
            .expect("model should be a SingleSelection");
        let selected_idx = selection.selected();
        if selected_idx != INVALID_LIST_POSITION {
            selection.set_selected(INVALID_LIST_POSITION);
            self.stars().remove(selected_idx);
            Propagation::Stop
        } else {
            Propagation::Proceed
        }
    }

    fn handle_new_action(&self) {
        self.stars().remove_all();
        self.imp().current_file.replace(None);
    }

    fn handle_save_action(&self) {
        if self.stars().n_items() == 0 || self.imp().file_dialog.borrow().is_some() {
            return;
        }

        let dialog = FileChooserNative::new(Some("Save Run"), Some(self), FileChooserAction::Save, None, None);
        let file_ref = self.imp().current_file.borrow();
        if let Some(path_buf) = file_ref.as_ref() {
            let path = path_buf.as_path();
            let parent = File::for_path(path.parent().unwrap());
            dialog.set_current_folder(Some(&parent)).expect("expected setting folder to succeed");
            dialog.set_current_name(path.file_name().unwrap().to_str().unwrap());
        } else {
            dialog.set_current_folder(Some(&self.get_last_dir())).expect("expected setting folder to succeed");
        }
        let main_window = self.clone();
        dialog.connect_response(move |dlg: &FileChooserNative, response| {
            if response != ResponseType::Cancel {
                // File is None, if user hits escape
                if let Some(file) = dlg.file() {
                    if let Some(run) = main_window.extract_run() {
                        match file.replace(None, false, FileCreateFlags::NONE, None::<&Cancellable>) {
                            Ok(stream) => {
                                let data_str = serde_json::to_string(&run).unwrap();
                                stream.write(data_str.as_bytes(), None::<&Cancellable>).expect("expected write to succeed");
                                stream.close(None::<&Cancellable>).expect("expected close to succeed");
                                let file_dir = file.path().unwrap().parent().unwrap().to_str().unwrap().to_string();
                                main_window.settings().set("last-dir", file_dir).expect("expected setting last dir to succeed");
                                run.filters.into_iter().for_each(|f| {
                                    match f {
                                        0 => {
                                            main_window.settings().set("filter-u", true).expect("expected setting filter to succeed");
                                        }
                                        1 => {
                                            main_window.settings().set("filter-b", true).expect("expected setting filter to succeed");
                                        }
                                        2 => {
                                            main_window.settings().set("filter-v", true).expect("expected setting filter to succeed");
                                        }
                                        3 => {
                                            main_window.settings().set("filter-r", true).expect("expected setting filter to succeed");
                                        }
                                        4 => {
                                            main_window.settings().set("filter-i", true).expect("expected setting filter to succeed");
                                        }
                                        _ => {
                                            panic!("invalid filter index")
                                        }
                                    }
                                })
                            }
                            Err(e) => {
                                util::show_error(Some(&main_window), Some("Error Saving Run"), e);
                            }
                        }
                    }
                }
            }
            main_window.imp().file_dialog.replace(None);
        });
        dialog.show();
        self.imp().file_dialog.replace(Some(dialog));

    }

    fn handle_generate_action(&self) {
        let imp = self.imp();
        let filter_settings = FilterSettings {
            u: imp.filter_u.is_active(),
            b: imp.filter_b.is_active(),
            v: imp.filter_v.is_active(),
            r: imp.filter_r.is_active(),
            i: imp.filter_i.is_active(),
        };
        let dialog = GenerateRunDialog::new(&self.application().unwrap(), self, &filter_settings);
        let main_window = self.clone();
        dialog.connect_response(move |dlg: &GenerateRunDialog, response| {
           if response == ResponseType::Ok {
               if let Some(run) = dlg.get_run() {
                   dlg.hide();
                   dlg.destroy();
                   main_window.replace_run(run)
               }
           } else {
               dlg.hide();
               dlg.destroy();
           }
        });

        dialog.show();
    }

    fn extract_run(&self) -> Option<PepRun> {
        let mut filters: Vec<u8> = Vec::new();
        let mw_imp = self.imp();
        if mw_imp.filter_u.is_active() {
            filters.push(0);
        }
        if mw_imp.filter_b.is_active() {
            filters.push(1);
        }
        if mw_imp.filter_v.is_active() {
            filters.push(2);
        }
        if mw_imp.filter_r.is_active() {
            filters.push(3);
        }
        if mw_imp.filter_i.is_active() {
            filters.push(4);
        }
        if filters.is_empty() {
            return None;
        }

        let stars = self.stars()
            .into_iter()
            .flatten()
            .map(|obj| {
                let star = obj.downcast_ref::<StarObject>()
                    .expect("object should be a StarObject")
                    .imp()
                    .data.borrow();
                StarData::new(&star.star_type, &star.name)
            })
            .collect::<Vec<_>>();
        if stars.is_empty() {
            return None;
        }
        Some(PepRun::new(filters, stars))
    }

    fn handle_open_action(&self) {
        if self.imp().file_dialog.borrow().is_some() {
            return;
        }
        let dialog = FileChooserNative::new(Some("Save Run"),
                                            Some(self),
                                            FileChooserAction::Open,
                                            None, None);
        let last_dir = self.get_last_dir();
        dialog.set_current_folder(Some(&last_dir)).expect("expected setting folder to succeed");

        let main_window = self.clone();
        dialog.connect_response(move |dlg: &FileChooserNative, response| {
            if response != ResponseType::Cancel {
                let file = dlg.file().unwrap();

                match file.query_info(FILE_ATTRIBUTE_STANDARD_SIZE, FileQueryInfoFlags::NONE, None::<&Cancellable>) {
                    Ok(info) => {
                        let size= info.size() as usize;
                        match file.read(None::<&Cancellable>) {
                            Ok(stream) => {
                                let mut buf_vec = vec![0u8; size];
                                stream.read(buf_vec.as_mut_slice(), None::<&Cancellable>).expect("expected read to succeed");
                                match String::from_utf8(buf_vec) {
                                    Ok(str) => {
                                        let serde_result : Result<PepRun, serde_json::Error> = serde_json::from_str(&str);
                                        match serde_result {
                                            Ok(run) => {
                                                main_window.replace_run(run);
                                                main_window.imp().current_file.replace(file.path());
                                                let file_dir = file.path().unwrap().parent().unwrap().to_str().unwrap().to_string();
                                                main_window.settings().set("last-dir", file_dir).expect("expected setting last dir to succeed");
                                            }
                                            Err(e) => {
                                                show_error(Some(&main_window), Some("No PEP Run in File"), e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        show_error(Some(&main_window), Some("Error Converting File to String"), e);
                                    }
                                }

                            }
                            Err(e) => {
                                show_error(Some(&main_window), Some("Error Reading Run"), e);
                            }
                        }
                    }
                    Err(e) => {
                       show_error(Some(&main_window), Some("Error Getting Run Size"), e);
                    }
                }
            }
            main_window.imp().file_dialog.replace(None);
        }
        );
        dialog.show();
        self.imp().file_dialog.replace(Some(dialog));
    }

    fn get_last_dir(&self) -> File {
        let settings_dir = self.settings().get::<String>("last-dir");
        if !settings_dir.is_empty() {
            return File::for_path(Path::new(&settings_dir));
        }
        match var("HOME") {
            Ok(dir) => { File::for_path(Path::new(&dir)) }
            Err(_) => { File::for_path(Path::new(".")) }
        }
    }

    pub fn replace_run(&self, run: PepRun) {
        let imp = self.imp();
        imp.filter_u.set_active(false);
        imp.filter_b.set_active(false);
        imp.filter_v.set_active(false);
        imp.filter_r.set_active(false);
        imp.filter_i.set_active(false);

        run.filters.into_iter().for_each(|filter| {
            match filter {
                0 => { imp.filter_u.set_active(true) }
                1 => { imp.filter_b.set_active(true) }
                2 => { imp.filter_v.set_active(true) }
                3 => { imp.filter_r.set_active(true) }
                4 => { imp.filter_i.set_active(true) }
                _ => { panic!("invalid filter"); }
            }
        });

        self.stars().remove_all();

        run.items.into_iter().for_each(|item| {
            let star = StarObject::new(item.star_type, item.name);
            self.stars().append(&star);
        })
    }

    fn start_execution(&self) {
        let imp = self.imp();
        let device = imp.settings.get().unwrap().string("device").as_str().trim().to_string();
        if device.is_empty() {
            show_error(Some(self), Some("No Device"), "Please configure a device.");
            return;
        }

        if let Some(run) = self.extract_run() {
            imp.main_menu_mb.set_sensitive(false);
            imp.execute_button.set_sensitive(false);
            execute_run(&device, run, self.get_last_dir(), self.clone(), clone!(
                #[weak(rename_to = main_window)]
                self,
                move || {
                    main_window.end_execution()
                }
            ));
        } else {
            show_error(Some(self), Some("No Run"), "Please enter or load a run.");
            return;
        }
    }

    fn end_execution(&self) {
        let imp = self.imp();
        imp.main_menu_mb.set_sensitive(true);
        imp.execute_button.set_sensitive(true);
    }

    fn setup_callbacks(&self) {
        self.imp().star_name_entry.connect_activate(clone!(
            #[weak(rename_to = main_window)]
            self,
            move |_| {
                match main_window.imp().editing.replace(None) {
                    Some(pos) => {
                        main_window.update_star(pos);
                    }
                    None => {
                        main_window.new_star();
                    }
                }
            }
        ));

        let evt_ctrl = EventControllerKey::new();
        // Need to manually clone here because the macro expects unit result.
        let main_window = self.clone();
        evt_ctrl.connect_key_pressed(
            move |_, key, _, _| {
                if key == Key::Escape {
                    main_window.handle_escape();
                    Propagation::Stop
                } else {
                    Propagation::Proceed
                }
            }
        );
        self.imp().star_name_entry.add_controller(evt_ctrl);

        let evt_ctrl = EventControllerKey::new();
        let main_window = self.clone();
        evt_ctrl.connect_key_pressed(
            move |_, key, _, _| {
                if main_window.imp().execute_button.get_sensitive() {
                    main_window.remove_star(&key)
                } else {
                    Propagation::Proceed
                }
            }
        );
        self.imp().star_list_vw.add_controller(evt_ctrl);

        self.imp().star_list_vw.connect_activate(clone!(
            #[weak(rename_to = main_window)]
            self,
            move |list_view, pos| {
                if main_window.imp().execute_button.get_sensitive() {
                    let star_object = list_view
                        .model()
                        .unwrap()
                        .item(pos)
                        .and_downcast::<StarObject>()
                        .expect("item should be a StarObject");

                    let star_data = star_object.imp().data.borrow();
                    main_window.imp().editing.replace(Some(pos));
                    main_window.select_star(&star_data);
                }
            }
        ));

        self.imp().execute_button.connect_clicked(clone!(
            #[weak(rename_to = main_window)]
            self,
            move |_| {
                main_window.start_execution()
            }
        ));
    }

    fn setup_factory(&self) {
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let star_row = StarObjectRow::new();
            list_item
                .downcast_ref::<ListItem>()
                .expect("should be a ListItem")
                .set_child(Some(&star_row));

        });

        factory.connect_bind(move |_, list_item| {
            let star_object = list_item
                .item()
                .and_downcast::<StarObject>()
                .expect("item should be a StarObject");

            let star_row = list_item
                .child()
                .and_downcast::<StarObjectRow>()
                .expect("child should be a StarObjectRow");

            star_row.bind(&star_object);
        });


        factory.connect_unbind(move |_, list_item| {
            let star_row = list_item
                .downcast_ref::<ListItem>()
                .expect("should be a ListItem")
                .child()
                .and_downcast::<StarObjectRow>()
                .expect("child should be a StarObjectRow");
            star_row.unbind();
        });

        self.imp().star_list_vw.set_factory(Some(&factory));
    }

    fn setup_actions(&self) {
        let action_new = ActionEntry::builder("file_new")
            .activate(
                move |window: &MainWindow, _, _| {
                    window.handle_new_action();
                }
            )
            .build();

        let action_save = ActionEntry::builder("file_save")
            .activate(
                move |window: &MainWindow, _, _| {
                    window.handle_save_action();
                }
            )
            .build();

        let action_open = ActionEntry::builder("file_open")
            .activate(
                move |window: &MainWindow, _, _| {
                    window.handle_open_action();
                }
            )
            .build();

        let action_configure = ActionEntry::builder("configure")
            .activate(
                move |window: &MainWindow, _, _| {
                    let dlg = ConfigDialog::new(&window.application().unwrap(), window, window.settings().clone());
                    dlg.show();
                }
            )
            .build();

        let action_gen_run = ActionEntry::builder("gen_run")
            .activate(
                move |window: &MainWindow, _, _| {
                    window.handle_generate_action();
                }
            )
            .build();

        self.add_action_entries([action_new, action_save, action_open, action_configure, action_gen_run]);
    }
}

glib::wrapper! {
    pub struct StarObject(ObjectSubclass<imp::StarObject>);
}

impl StarObject {
    pub fn new(star_type: String, name: String) -> Self {
        Object::builder()
            .property("star-type", star_type)
            .property("name", name)
            .build()
    }
}

glib::wrapper! {
    pub struct StarObjectRow(ObjectSubclass<imp::StarObjectRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for StarObjectRow {
    fn default() -> Self {
        Self::new()
    }
}

impl StarObjectRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(self: &Self, star_object: &StarObject) {
        let star_type_label = self.imp().type_label.get();
        let star_name_label = self.imp().name_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let star_type_binding = star_object
            .bind_property("star-type", &star_type_label, "label")
            .sync_create()
            .build();
        bindings.push(star_type_binding);

        let star_name_binding = star_object
            .bind_property("name", &star_name_label, "label")
            .sync_create()
            .build();
        bindings.push(star_name_binding);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

pub struct FilterSettings {
    u: bool,
    b: bool,
    v: bool,
    r: bool,
    i: bool,
}