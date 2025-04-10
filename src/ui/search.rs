use gio::ActionEntry;
use gtk4::{
    self,
    gdk::{self, Key, ModifierType},
    prelude::*,
    Builder, EventControllerKey, Image, Overlay,
};
use gtk4::{glib, ApplicationWindow, Entry};
use gtk4::{Box as HVBox, Label, ListBox, ScrolledWindow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::tiles::util::AsyncLauncherTile;
use super::util::*;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::{construct_tiles, Launcher, ResultItem};
use crate::{AppState, APP_STATE, CONFIG};

#[allow(dead_code)]
struct SearchUI {
    result_viewport: ScrolledWindow,
    // will be later used for split view to display information about apps/commands
    preview_box: HVBox,
    search_bar: Entry,
    search_icon_holder: HVBox,
    mode_title: Label,
}

pub fn search(launchers: &Vec<Launcher>, window: &ApplicationWindow) {
    // Initialize the view to show all apps
    let (mode, modes, stack_page, ui, results) = construct_window(&launchers);
    ui.result_viewport
        .set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    ui.search_bar.grab_focus();

    let search_bar_clone = ui.search_bar.clone();
    let modes_clone = modes.clone();
    let mode_clone = Rc::clone(&mode);

    let custom_binds = ConfKeys::new();

    change_event(
        &ui.search_bar,
        modes,
        &mode,
        &launchers,
        &results,
        &custom_binds,
    );
    nav_event(
        results,
        ui.search_bar,
        ui.result_viewport,
        mode,
        custom_binds,
    );
    APP_STATE.with(|state| {
        if let Some(ref state) = *state.borrow() {
            state.add_stack_page(stack_page, "search-page");
        }
    });

    // Improved mode selection
    let original_mode = String::from("all");
    let mode_action = ActionEntry::builder("switch-mode")
        .parameter_type(Some(&String::static_variant_type()))
        .state(original_mode.to_variant())
        .activate(move |_, action, parameter| {
            let state = action.state().and_then(|s| s.get::<String>());
            let parameter = parameter.and_then(|p| p.get::<String>());

            if let (Some(mut state), Some(mut parameter)) = (state, parameter) {
                parameter.push_str(" ");
                let mode_name = modes_clone.get(&parameter);
                match mode_name {
                    Some(name) => {
                        ui.search_icon_holder.set_css_classes(&["back"]);
                        *mode_clone.borrow_mut() = parameter.clone();
                        ui.mode_title.set_text(&name);
                        state = parameter;
                    }
                    _ => {
                        ui.search_icon_holder.set_css_classes(&["search"]);
                        ui.mode_title.set_text("All");
                        parameter = String::from("all ");
                        *mode_clone.borrow_mut() = parameter.clone();
                        state = parameter;
                    }
                }
                let search_bar_clone = search_bar_clone.clone();
                glib::idle_add_local(move || {
                    // to trigger homescreen rebuild
                    search_bar_clone.set_text("a");
                    search_bar_clone.set_text("");
                    glib::ControlFlow::Break
                });
                action.set_state(&state.to_variant());
            }
        })
        .build();
    window.add_action_entries([mode_action]);
}

fn construct_window(
    launchers: &Vec<Launcher>,
) -> (
    Rc<RefCell<String>>,
    HashMap<String, String>,
    HVBox,
    SearchUI,
    Rc<ListBox>,
) {
    // Collect Modes
    let mode = Rc::new(RefCell::new("all".to_string()));
    let mut modes: HashMap<String, String> = HashMap::new();
    launchers
        .iter()
        .filter_map(|item| item.alias.as_ref().map(|alias| (alias, &item.name)))
        .for_each(|(alias, name)| {
            modes.insert(format!("{} ", alias), name.clone());
        });

    // Initialize the builder with the correct path
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/search.ui");

    // Get the required object references
    let vbox: HVBox = builder.object("vbox").unwrap();
    let results: Rc<ListBox> = Rc::new(builder.object("result-frame").unwrap());

    let search_icon_holder: HVBox = builder.object("search-icon-holder").unwrap_or_default();
    search_icon_holder.add_css_class("search");
    // Create the search icon
    let search_icon = Image::new();
    search_icon.set_icon_name(Some("search"));
    search_icon.set_widget_name("search-icon");
    search_icon.set_halign(gtk4::Align::End);
    // Create the back arrow
    let search_icon_back = Image::new();
    search_icon_back.set_icon_name(Some("go-previous"));
    search_icon_back.set_widget_name("search-icon-back");
    search_icon_back.set_halign(gtk4::Align::End);

    let overlay = Overlay::new();
    overlay.set_child(Some(&search_icon));
    overlay.add_overlay(&search_icon_back);

    search_icon_holder.append(&overlay);

    let ui = SearchUI {
        result_viewport: builder.object("scrolled-window").unwrap_or_default(),
        preview_box: builder.object("preview_box").unwrap_or_default(),
        search_bar: builder.object("search-bar").unwrap_or_default(),
        search_icon_holder,
        mode_title: builder.object("category-type-label").unwrap_or_default(),
    };
    CONFIG.get().map(|c| {
        ui.result_viewport
            .set_size_request((c.appearance.width as f32 * 0.4) as i32, 10);
        ui.search_icon_holder.set_visible(c.appearance.search_icon);
        search_icon.set_pixel_size(c.appearance.icon_size);
        search_icon_back.set_pixel_size(c.appearance.icon_size);
    });

    APP_STATE.with(|app_state| {
        let new_state = app_state.borrow_mut().take().map(|old_state| {
            Rc::new(AppState {
                window: old_state.window.clone(),
                stack: old_state.stack.clone(),
                search_bar: Some(ui.search_bar.clone()),
            })
        });
        *app_state.borrow_mut() = new_state;
    });
    (mode, modes, vbox, ui, results)
}

fn nav_event(
    results: Rc<ListBox>,
    search_bar: Entry,
    result_viewport: ScrolledWindow,
    mode: Rc<RefCell<String>>,
    custom_binds: ConfKeys,
) {
    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed(move |_, key, i, modifiers| {
        match key {
            k if Some(k) == custom_binds.prev
                && custom_binds
                    .prev_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results.focus_prev(&result_viewport);
                return true.into();
            }
            k if Some(k) == custom_binds.next
                && custom_binds
                    .next_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results.focus_next(&result_viewport);
                return true.into();
            }
            gdk::Key::Up => {
                results.focus_prev(&result_viewport);
            }
            gdk::Key::Down => {
                results.focus_next(&result_viewport);
                return true.into();
            }
            gdk::Key::BackSpace => {
                let ctext = search_bar.text();
                if custom_binds
                    .shortcut_modifier
                    .map_or(false, |modifier| modifiers.contains(modifier))
                {
                    let _ = search_bar.set_text("");
                } else {
                    if ctext.is_empty() && mode.borrow().as_str() != "all" {
                        let _ = search_bar
                            .activate_action("win.switch-mode", Some(&"all".to_variant()));
                    }
                }
                results.focus_first();
            }
            gdk::Key::Return => {
                if let Some(row) = results.selected_row().and_downcast_ref::<SherlockRow>() {
                    row.emit_by_name::<()>("row-should-activate", &[]);
                }
            }
            Key::_1 | Key::_2 | Key::_3 | Key::_4 | Key::_5 => {
                if custom_binds
                    .shortcut_modifier
                    .map_or(false, |modifier| modifiers.contains(modifier))
                {
                    let key_index = match key {
                        Key::_1 => 1,
                        Key::_2 => 2,
                        Key::_3 => 3,
                        Key::_4 => 4,
                        Key::_5 => 5,
                        _ => return false.into(),
                    };
                    execute_by_index(&*results, key_index);
                    return true.into();
                }
            }
            // Pain - solution for shift-tab since gtk handles it as an individual event
            _ if i == 23 && modifiers.contains(ModifierType::SHIFT_MASK) => {
                let shift = Some(ModifierType::SHIFT_MASK);
                let tab = Some(Key::Tab);
                if custom_binds.prev_mod == shift && custom_binds.prev == tab {
                    results.focus_prev(&result_viewport);
                    return true.into();
                } else if custom_binds.next_mod == shift && custom_binds.next == tab {
                    results.focus_next(&result_viewport);
                    return true.into();
                }
            }
            _ => (),
        }
        false.into()
    });
    APP_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|s| s.add_event_listener(event_controller))
    });
}

fn change_event(
    search_bar: &Entry,
    modes: HashMap<String, String>,
    mode: &Rc<RefCell<String>>,
    launchers: &Vec<Launcher>,
    results: &Rc<ListBox>,
    custom_binds: &ConfKeys,
) {
    // Setting up async capabilities
    let current_task: Rc<RefCell<Option<glib::JoinHandle<()>>>> = Rc::new(RefCell::new(None));
    let cancel_flag = Rc::new(RefCell::new(false));

    // vars
    let mod_str = custom_binds.shortcut_modifier_str.clone();

    // Setting home screen
    async_calc(
        &cancel_flag,
        &current_task,
        &launchers,
        &mode,
        String::new(),
        &results,
        &mod_str,
        true,
    );

    search_bar.connect_changed({
        let launchers_clone = launchers.clone();
        let mode_clone = Rc::clone(mode);
        let results_clone = Rc::clone(results);

        move |search_bar| {
            let mut current_text = search_bar.text().to_string();
            if let Some(task) = current_task.borrow_mut().take() {
                task.abort();
            };
            *cancel_flag.borrow_mut() = true;
            let trimmed = current_text.trim();
            if !trimmed.is_empty() && modes.contains_key(&current_text) {
                // Logic to apply modes
                if modes.contains_key(&current_text) {
                    let _ =
                        search_bar.activate_action("win.switch-mode", Some(&trimmed.to_variant()));
                    current_text.clear();
                }
            }
            async_calc(
                &cancel_flag,
                &current_task,
                &launchers_clone,
                &mode_clone,
                current_text,
                &results_clone,
                &mod_str,
                false,
            );
        }
    });
}

pub fn async_calc(
    cancel_flag: &Rc<RefCell<bool>>,
    current_task: &Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    launchers: &[Launcher],
    mode: &Rc<RefCell<String>>,
    current_text: String,
    results: &Rc<ListBox>,
    mod_str: &str,
    animate: bool,
) {
    *cancel_flag.borrow_mut() = false;
    // If task is currently running, abort it
    if let Some(t) = current_task.borrow_mut().take() {
        t.abort();
    };
    let is_home = current_text.is_empty() && mode.borrow().as_str().trim() == "all";
    let cancel_flag = Rc::clone(&cancel_flag);
    let filtered_launchers: Vec<Launcher> = launchers
        .iter()
        .filter(|launcher| (is_home && launcher.home) || (!is_home && !launcher.only_home))
        .cloned()
        .collect();
    let (async_launchers, non_async_launchers): (Vec<Launcher>, Vec<Launcher>) = filtered_launchers
        .into_iter()
        .partition(|launcher| launcher.r#async);

    // Create loader widgets
    // TODO
    let current_mode_ref = mode.borrow();
    let current_mode = current_mode_ref.trim();

    // extract result items to reduce cloning
    let mut async_widgets: Vec<ResultItem> = Vec::with_capacity(async_launchers.capacity());
    let async_launchers: Vec<AsyncLauncherTile> = async_launchers
        .into_iter()
        .filter_map(|launcher| {
            if (launcher.priority == 0 && current_mode == launcher.alias.as_deref().unwrap_or(""))
                || (current_mode == "all" && launcher.priority > 0)
            {
                launcher.get_loader_widget(&current_text).map(
                    |(result_item, title, body, async_opts, attrs)| {
                        async_widgets.push(result_item.clone());
                        AsyncLauncherTile {
                            launcher,
                            title,
                            body,
                            result_item,
                            async_opts,
                            attrs,
                        }
                    },
                )
            } else {
                None
            }
        })
        .collect();
    populate(
        &current_text,
        &mode.borrow(),
        &*results,
        &non_async_launchers,
        Some(async_widgets),
        animate,
        mod_str,
    );

    // Gather results for asynchronous widgets
    let task = glib::MainContext::default().spawn_local({
        let current_task_clone = Rc::clone(current_task);
        async move {
            if *cancel_flag.borrow() {
                return;
            }
            // get results for aysnc launchers
            for widget in async_launchers.iter() {
                let mut attrs = widget.attrs.clone();
                if let Some((title, body, next_content)) =
                    widget.launcher.get_result(&current_text).await
                {
                    widget.title.as_ref().map(|t| t.set_text(&title));
                    widget.body.as_ref().map(|b| b.set_text(&body));
                    if let Some(next_content) = next_content {
                        attrs.insert(String::from("next_content"), next_content.to_string());
                    }
                }
                if let Some(opts) = &widget.async_opts {
                    // Replace one image with another
                    if let Some(overlay) = &opts.icon_holder_overlay {
                        if let Some((image, was_cached)) = widget.launcher.get_image().await {
                            // Also check for animate key
                            if !was_cached {
                                overlay.add_css_class("image-replace-overlay");
                            }
                            let gtk_image = Image::from_pixbuf(Some(&image));
                            gtk_image.set_widget_name("album-cover");
                            gtk_image.set_pixel_size(50);
                            overlay.add_overlay(&gtk_image);
                        }
                    }
                }
                widget
                    .result_item
                    .row_item
                    .connect("row-should-activate", false, move |row| {
                        let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                        execute_from_attrs(&row, &attrs);
                        None
                    });
            }
            *current_task_clone.borrow_mut() = None;
        }
    });
    *current_task.borrow_mut() = Some(task);
}

pub fn populate(
    keyword: &str,
    mode: &str,
    results_frame: &ListBox,
    launchers: &Vec<Launcher>,
    async_widgets: Option<Vec<ResultItem>>,
    animate: bool,
    mod_str: &str,
) {
    // Remove all elements inside to avoid duplicates
    while let Some(row) = results_frame.last_child() {
        results_frame.remove(&row);
    }
    let mut launcher_tiles = construct_tiles(&keyword.to_string(), &launchers, &mode.to_string());
    if let Some(widgets) = async_widgets {
        launcher_tiles.extend(widgets);
    }

    launcher_tiles.sort_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap());

    if let Some(c) = CONFIG.get() {
        let mut shortcut_index = 1;
        for widget in launcher_tiles {
            if animate && c.behavior.animate {
                widget.row_item.add_css_class("animate");
            }
            if let Some(shortcut_holder) = widget.shortcut_holder {
                shortcut_index += shortcut_holder.apply_shortcut(shortcut_index, mod_str);
            }
            results_frame.append(&widget.row_item);
        }
    }
    results_frame.focus_first();
}
