use gtk4::gdk::{self, Rectangle};
use gtk4::{self, prelude::*, ApplicationWindow, Builder, EventControllerKey};
use gtk4::{Box as HVBox, Entry, ScrolledWindow, Label, ListBox};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::launcher::Launcher;
use crate::actions::execute_from_attrs;
use super::util::*;


pub fn search(window: ApplicationWindow, launchers:Vec<Launcher>) -> ApplicationWindow {
    // Collect Modes
    let mode = Rc::new(RefCell::new("all".to_string()));
    let mut modes: HashMap<String, String> = HashMap::new();
    for item in launchers.iter(){
        let alias = item.alias();
        if !alias.is_empty() {
            let name= item.name();
            modes.insert(format!("{} ", alias), name);
        }
    }

    // Initialize the builder with the correct path
    let builder = Builder::from_resource("/com/skxxtz/sherlock/ui/search.ui");

    // Get the requred object references
    let vbox: HVBox = builder.object("vbox").unwrap();
    let search_bar: Entry = builder.object("search-bar").unwrap();
    let result_viewport: ScrolledWindow = builder.object("scrolled-window").unwrap();
    let mode_title: Label = builder.object("category-type-label").unwrap();
    let results: ListBox = builder.object("result-frame").unwrap();

    //RC cloning:
    let results = Rc::new(results);

    let mode_clone_ev_changed = Rc::clone(&mode);
    let mode_clone_ev_nav = Rc::clone(&mode);
    let mode_title_clone = mode_title.clone();

    let results_enter = Rc::clone(&results);
    let results_clone_ev_nav = Rc::clone(&results);

    let launchers_clone_ev_changed = launchers.clone();
    let launchers_clone_ev_nav = launchers.clone();

    // Initiallize the view to show all apps
    set_results("","all", &*results, &launchers);

    // Setting search window to active
    window.set_child(Some(&vbox));
    search_bar.grab_focus();

    // Eventhandling for text change inside search bar
    // EVENT: CHANGE
    search_bar.connect_changed(move |search_bar| {
        let current_text = search_bar.text().to_string();

        // Check if current text is present in modes
        if modes.contains_key(&current_text) {
            if let Some(mode_name) = modes.get(&current_text){
                set_mode(&mode_title_clone, &mode_clone_ev_changed, &current_text, mode_name);
                search_bar.set_text("");
            }
        } else {
            set_results(&current_text,&mode_clone_ev_changed.borrow(), &*results, &launchers_clone_ev_changed);
        }
    });


    // Eventhandling for navigation
    // EVENT: Navigate
    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed(move |_, key, _, modifiers| {
        match key {
            gdk::Key::Up => {
                let new_row = select_row(-1, &results_clone_ev_nav);

                let row_allocation = new_row.allocation();
                let row_rect = Rectangle::from(row_allocation);

                let row_start = (row_rect.y()) as f64;
                let vadjustment = result_viewport.vadjustment();

                let current_value = vadjustment.value();
                if current_value > row_start {
                    vadjustment.set_value(row_start);
                } 
            },
            gdk::Key::Down => {
                select_row(1, &results_clone_ev_nav);
                let allocation = result_viewport.allocation();
                let list_box_rect = Rectangle::from(allocation);

                let row_allocation = results_clone_ev_nav.selected_row().unwrap().allocation();
                let row_rect = Rectangle::from(row_allocation);

                let list_height = list_box_rect.height() as f64;
                let row_end = (row_rect.y() + row_rect.height() + 10) as f64;
                let vadjustment = result_viewport.vadjustment();

                let current_value = vadjustment.value();
                let list_end = list_height + current_value;
                if row_end > list_end {
                    let delta = row_end - list_end;
                    let new_value = current_value + delta;
                    vadjustment.set_value(new_value);
                }
            },
            gdk::Key::BackSpace => {
                let ctext = &search_bar.text();
                if ctext.is_empty(){
                    set_mode(&mode_title, &mode_clone_ev_nav, "all", &"All".to_string());
                    set_results(&ctext,&mode_clone_ev_nav.borrow(), &*results_clone_ev_nav, &launchers_clone_ev_nav);
                }
            },
            gdk::Key::Return => {
                if let Some(row) = results_enter.selected_row(){
                    let attrs: HashMap<String, String> = get_row_attrs(row);
                    execute_from_attrs(attrs);
                }
            },
            gdk::Key::_1 => {
                if modifiers.contains(gdk::ModifierType::CONTROL_MASK){
                    execute_by_index(&*results_clone_ev_nav, 1);
                }
            },
            gdk::Key::_2 => {
                if modifiers.contains(gdk::ModifierType::CONTROL_MASK){
                    execute_by_index(&*results_clone_ev_nav, 2);
                }
            },
            gdk::Key::_3 => {
                if modifiers.contains(gdk::ModifierType::CONTROL_MASK){
                    execute_by_index(&*results_clone_ev_nav, 3);
                }
            },
            gdk::Key::_4 => {
                if modifiers.contains(gdk::ModifierType::CONTROL_MASK){
                    execute_by_index(&*results_clone_ev_nav, 4);
                }
            },
            gdk::Key::_5 => {
                if modifiers.contains(gdk::ModifierType::CONTROL_MASK){
                    execute_by_index(&*results_clone_ev_nav, 5);
                }
            },

            _ => (),
        }
        false.into()
    });


    window.add_controller(event_controller);

    return window;
}

