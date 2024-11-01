use crate::common::PepRun;
use crate::ssp3;
use crate::ssp3::Ssp3;
use crate::util::show_error;
use chrono::{DateTime, Utc};
use gtk::prelude::{DialogExt, FileChooserExt, FileChooserExtManual, FileExt, GtkWindowExt, IsA, NativeDialogExt, WidgetExt};
use gtk::{gio, glib, ButtonsType, DialogFlags, FileChooserAction, FileChooserNative, MessageDialog, MessageType, ResponseType, Window};
use serde::Serialize;
use std::borrow::BorrowMut;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::sync::{Arc, Mutex};
use async_channel::{Receiver, Sender};
use csv::Writer;
use gtk::glib::IntoGStr;
use crate::measurement::SspRequest::{Finish, Measure};

struct State {
    run: PepRun,
    i_time_by_star: HashMap<String, HashMap<u8, u16>>,
    star_index: u16,
    filter_index: u8,
    sky: bool,
}

impl State {
    fn new(run: PepRun) -> Self {
        let star_keys = run.items.iter().map(|s| s.name.clone()).collect::<HashSet<String>>();
        let mut i_time_by_star = HashMap::with_capacity(star_keys.len());
        star_keys.iter().for_each(|k| {
           i_time_by_star.insert(k.clone(), HashMap::with_capacity(run.filters.len()));
        });

        State {
            run,
            i_time_by_star,
            star_index: 0,
            filter_index: 0,
            sky: false,
        }

    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Measurement<'a> {
    timestamp: DateTime<Utc>,
    index: u16,
    star_id: &'a str,
    star_type: &'a str,
    is_star: bool,
    filter: &'a str,
    integration_time: u16,
    count1: u16,
    count2: u16,
    count3: u16,
}

enum SspResponse {
    Ok(),
    Error(ssp3::Error),
    Counts(u16),
}

enum SspRequest {
    Init(),
    Measure(u8, u16),
    Finish(),
}

pub fn execute_run<F> (device: &str, run: PepRun, last_dir: gio::File, parent: impl IsA<Window>, completion_callback: F)
where F: Clone + FnOnce() -> () + 'static {
    let (gui_ssp_snd, gui_ssp_rcv) = async_channel::bounded(1);
    let (ssp_gui_snd, ssp_gui_rcv) = async_channel::bounded(1);

    let dev = String::from(device);
    gio::spawn_blocking(move || {
       run_ssp(dev, gui_ssp_rcv.clone(), ssp_gui_snd);
    });

    if let Err(_) = gui_ssp_snd.send_blocking(SspRequest::Init()) {
        show_error(Some(&parent), Some("Channel Closed"), "Channel to SSP3 is closed");
        completion_callback();
        return;
    }

    match ssp_gui_rcv.recv_blocking() {
        Ok(SspResponse::Ok()) => {

        }
        Ok(SspResponse::Error(e)) => {
            show_error(Some(&parent), Some("Error Initializing SSP3"), e);
            completion_callback();
            return;
        }
        Ok(SspResponse::Counts(_)) => {
            show_error(Some(&parent), Some("Unexpected SSP3 Response"), "Received counts response on initialization.");
            completion_callback();
            return;
        }
        Err(_) => {
            show_error(Some(&parent), Some("Channel Closed"), "Channel to SSP3 is closed");
            completion_callback();
            return;
        }
    }

    let dialog = FileChooserNative::new(Some("Save Run Log"), Some(&parent), FileChooserAction::Save, None, None);
    dialog.set_current_folder(Some(&last_dir)).expect("Unable to set current directory");
    dialog.connect_response(move |dlg, response| {
        if response != ResponseType::Cancel {
           if let Some(file) = dlg.file() {
               let path = file.path().unwrap();
               match Writer::from_path(path) {
                   Ok(writer) => {
                       let state = State::new(run.clone());
                       let state_arc = Arc::new(Mutex::new(state));
                       let writer_arc = Arc::new(Mutex::new(writer));
                       run_step(state_arc.clone(), writer_arc.clone(), parent.clone(), gui_ssp_snd.clone(), ssp_gui_rcv.clone(), completion_callback.clone());
                   }
                   Err(e) => {
                       gui_ssp_snd.send_blocking(SspRequest::Finish()).expect("Error shutting down SSP");
                       show_error(Some(&parent), Some("Error Opening Run Log"), e);
                       completion_callback.clone()();
                   }
               }

           }
        }
    });

}

fn run_step<F>(state: Arc<Mutex<State>>, writer: Arc<Mutex<Writer<File>>>, parent: impl IsA<Window>, sender: Sender<SspRequest>, receiver: Receiver<SspResponse>, completion_callback: F)
where F: FnOnce() -> () + Clone + 'static {
    let state_data = state.lock().unwrap();
    let msg = match state_data.sky {
        true => {
            "Go to sky".to_string()
        }
        false => {
            format!("Go to {}", state_data.run.items[state_data.star_index as usize].name)
        }
    };
    drop(state_data);
    run_with_msg(state, writer, parent, sender, receiver, completion_callback, msg);
}

fn run_with_msg<F>(state: Arc<Mutex<State>>, writer: Arc<Mutex<Writer<File>>>, parent: impl IsA<Window>, sender: Sender<SspRequest>, receiver: Receiver<SspResponse>, completion_callback: F, msg: impl IntoGStr)
where F: FnOnce() -> () + Clone + 'static {
    let dialog = MessageDialog::new(Some(&parent), DialogFlags::MODAL, MessageType::Question, ButtonsType::OkCancel, msg);
    dialog.set_title(Some("Operator Action"));
    let cloned_state = state.clone();
    dialog.connect_response(move |dlg, response| {
        dlg.hide();
        dlg.destroy();
        if response != ResponseType::Ok {
            sender.clone().send_blocking(Finish()).expect("Error shutting down SSP");
            completion_callback.clone()();
            return;
        }

        let cloned_state = cloned_state.clone();
        let cloned_writer = writer.clone();
        let cloned_parent = parent.clone();
        let cloned_sender = sender.clone();
        let cloned_receiver = receiver.clone();
        let cloned_completion_callback = completion_callback.clone();
        glib::spawn_future_local(async move {
            measure(cloned_state, cloned_writer, cloned_parent, cloned_sender, cloned_receiver, cloned_completion_callback)
        });
    });
    dialog.show();
}

fn abort_with_msg(parent: impl IsA<Window>, message: impl IntoGStr, completion_callback: impl FnOnce() -> () + Clone + 'static) {
    let dialog = MessageDialog::new(Some(&parent), DialogFlags::MODAL, MessageType::Error, ButtonsType::Ok, message);
    dialog.set_title(Some("Error"));
    dialog.connect_response(move |dlg, _| {
        dlg.hide();
        dlg.destroy();
        completion_callback.clone()();
    });
    dialog.show();
}

async fn measure<F>(state: Arc<Mutex<State>>, writer: Arc<Mutex<Writer<File>>>, parent: impl IsA<Window>, sender: Sender<SspRequest>, receiver: Receiver<SspResponse>, completion_callback: F)
where F: FnOnce() -> () + Clone + 'static
{
    let mut state_data = state.lock().unwrap();
    let star_index = state_data.star_index as usize;
    let star_name = String::from(&state_data.run.items[star_index].name);
    let star_type = String::from(&state_data.run.items[star_index].star_type);

    for filter_index in state_data.filter_index as usize .. state_data.run.filters.len() {
        let filter_map = state_data.i_time_by_star.get(&star_name).unwrap();
        let filter_slot = state_data.run.filters[filter_index];
        let (i_time, mut calibrate_i_time) = match filter_map.get(&filter_slot) {
            Some(m_i_time) => {
                (*m_i_time, false)
            }
            None => {
                if filter_slot < 2 {
                    (2000u16, true)
                } else {
                    (1000u16, true)
                }
            }
        };
        let mut used_i_time = i_time;
        let was_calibrated = calibrate_i_time;
        let mut count_slots: [u16; 3] = [0; 3];
        let mut count_slot: usize = 0;

        let start_time = Utc::now();
        loop {
            let command = Measure(filter_slot + 1, used_i_time);
            match sender.send(command).await {
                Ok(_) => {
                    match receiver.recv().await {
                        Ok(SspResponse::Counts(counts)) => {
                            if calibrate_i_time && count_slot == 0 {
                                if counts > 5000 {
                                    calibrate_i_time = false;
                                    count_slots[0] = counts;
                                } else if count_slots[0] == 0 {
                                    count_slots[0] = counts;
                                    used_i_time = i_time + 500;
                                } else {
                                    if counts > count_slots[0] {
                                        let delta_t_s = (5 * (5000 - count_slots[0])) / (counts - count_slots[0]);
                                        if delta_t_s > 0 {
                                            used_i_time = i_time + 100 * delta_t_s;
                                        } else {
                                            used_i_time = i_time;
                                            count_slot += 1;
                                        }
                                    } else {
                                        used_i_time = 3000;
                                    }
                                }
                            } else {
                                count_slots[count_slot] = counts;
                                count_slot += 1;
                                if count_slot == 3 {
                                    break;
                                }
                            }
                        }
                        Ok(SspResponse::Error(e)) => {
                            sender.send(Finish()).await.expect("Error shutting down SSP");
                            drop(state_data);
                            abort_with_msg(parent.clone(), e.to_string(), completion_callback.clone());
                            return;
                        }
                        Ok(_) => {
                            sender.send(Finish()).await.expect("Error shutting down SSP");
                            drop(state_data);
                            abort_with_msg(parent.clone(), "Unexpected SSP response", completion_callback.clone());
                            return;
                        }
                        Err(_) => {
                            drop(state_data);
                            abort_with_msg(parent.clone(), "Error receiving from SSP", completion_callback.clone());
                            return;
                        }
                    }
                }
                Err(_) => {
                    drop(state_data);
                    abort_with_msg(parent.clone(), "Error sending to SSP", completion_callback.clone());
                    return;
                }
            }
        }

        if !state_data.sky {
            let avg = count_slots.iter().sum::<u16>() as f32 / count_slots.len() as f32;
            let min = *count_slots.iter().min().unwrap() as f32;
            let max = *count_slots.iter().max().unwrap() as f32;
            if min < 0.99 * avg || max > 1.01 * avg {
                // More than 1% deviation: ask user to re-center
                drop(state_data);
                run_with_msg(state.clone(), writer, parent, sender, receiver, completion_callback, "Deviation > 1%. Please re-center.");
                return;
            }
        }

        let end_time = Utc::now();
        let middle_time = start_time + end_time.signed_duration_since(start_time);
        let filter = match filter_index {
            0 => { "U" }
            1 => { "B" }
            2 => { "V" }
            3 => { "R" }
            4 => { "I" }
            5 => { "C" }
            _ => { panic!("Unknown filter index {}", filter_index); }
        };
        let measurement = Measurement {
            timestamp: middle_time,
            index: state_data.star_index,
            star_id: &star_name,
            star_type: &star_type,
            is_star: !state_data.sky,
            filter,
            integration_time: used_i_time,
            count1: count_slots[0],
            count2: count_slots[1],
            count3: count_slots[2],
        };
        let mut writer_obj = writer.lock().unwrap();
        writer_obj.serialize(measurement).expect("Error serializing measurement");

        if was_calibrated {
            let filter_map: &mut HashMap<u8, u16> = state_data.i_time_by_star
                .get_mut(&star_name).unwrap();
            filter_map.insert(filter_slot, used_i_time);
        }
    }

    if state_data.sky {
        state_data.sky = false;
        state_data.star_index += 1;
        if state_data.star_index as usize == state_data.run.items.len() {
            sender.send(Finish()).await.expect("Error shutting down SSP");
            completion_callback();
            return;
        }
    } else {
        state_data.sky = true;
    }
    // Release Mutex
    drop(state_data);

    run_step(state, writer, parent, sender, receiver, completion_callback);
}

fn run_ssp(device: String, gui_ssp_rcv: async_channel::Receiver<SspRequest>, ssp_gui_snd: async_channel::Sender<SspResponse>) {
    match Ssp3::new(device.as_str()) {
        Ok(mut ssp3) => {
            ssp_main_loop(ssp3.borrow_mut(), &gui_ssp_rcv, &ssp_gui_snd);
        }
        Err(e) => {
            let err_msg = e.to_string();
            ssp_gui_snd.send_blocking(SspResponse::Error(e)).expect(&format!("Failed to send open error: {}", err_msg));
        }
    }
}

fn ssp_main_loop(device: &mut Ssp3, gui_ssp_rcv: &async_channel::Receiver<SspRequest>, ssp_gui_snd: &async_channel::Sender<SspResponse>) {
    while let Ok(request) = gui_ssp_rcv.recv_blocking() {
        match request {
            SspRequest::Init() => {
                match device.init() {
                    Ok(_) => {
                        if let Err(_) = ssp_gui_snd.send_blocking(SspResponse::Ok()) {
                            eprintln!("Error sending init OK");
                            break;
                        }
                    },
                    Err(e) => {
                        let err_msg = e.to_string();
                        if let Err(_) = ssp_gui_snd.send_blocking(SspResponse::Error(e)) {
                            eprintln!("Error sending init error: {}", err_msg);
                            break;
                        }
                    }
                }
            }
            SspRequest::Measure(filter, time) => {
                match device.measure(filter, time) {
                    Ok(counts) => {
                        if let Err(_) = ssp_gui_snd.send_blocking(SspResponse::Counts(counts)) {
                            eprintln!("Error sending counts.");
                            break;
                        }
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        if let Err(_) = ssp_gui_snd.send_blocking(SspResponse::Error(e)) {
                            eprintln!("Error sending counting error: {}", err_msg);
                            break;
                        }
                    }
                }
            }
            SspRequest::Finish() => {
                if let Err(_) = ssp_gui_snd.send_blocking(SspResponse::Ok()) {
                    eprintln!("Error sending finish OK");
                }
                break;
            }
        }
    }

    if let Err(e) = device.finish() {
        eprintln!("Error finishing device: {}", e);
    }
}