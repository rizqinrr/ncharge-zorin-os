mod meter;

use gtk4::prelude::*;
use libadwaita::prelude::*;
use meter::BatteryMeter;
use ncharge_apply::config::{read_effective_limit, ConfigPaths};
use ncharge_apply::power::{find_supported_batteries, scan_batteries, DEFAULT_POWER_SUPPLY_ROOT};
use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;

const APP_ID: &str = "io.github.ncharge";

fn main() {
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_startup(|_| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(build_ui);
    app.run();
}

#[derive(Clone)]
struct AppState {
    current_capacity: Rc<Cell<u8>>,
    active_limit: Rc<Cell<u8>>,
    selected_preset: Rc<Cell<u8>>,
    is_override: Rc<Cell<bool>>,
}

fn build_ui(app: &libadwaita::Application) {
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("ncharge")
        .default_width(400)
        .resizable(false)
        .build();

    let toast_overlay = libadwaita::ToastOverlay::new();

    let supported_bats = find_supported_batteries(DEFAULT_POWER_SUPPLY_ROOT);
    if supported_bats.is_empty() {
        let unsupported_box = build_unsupported_ui();
        toast_overlay.set_child(Some(&unsupported_box));
        window.set_content(Some(&toast_overlay));
        window.present();
        return;
    }

    let paths = ConfigPaths::default();
    let (eff_limit, is_ovr) = read_effective_limit(&paths);
    let initial_limit = supported_bats[0]
        .charge_control_end_threshold
        .unwrap_or(eff_limit);

    let state = AppState {
        current_capacity: Rc::new(Cell::new(supported_bats[0].capacity.unwrap_or(100))),
        active_limit: Rc::new(Cell::new(initial_limit)),
        selected_preset: Rc::new(Cell::new(initial_limit)),
        is_override: Rc::new(Cell::new(is_ovr)),
    };

    // Header bar
    let header_bar = libadwaita::HeaderBar::builder()
        .show_end_title_buttons(true)
        .show_start_title_buttons(false)
        .build();

    let title_lbl = gtk4::Label::builder()
        .label("ncharge")
        .css_classes(["header-title"])
        .build();
    header_bar.set_title_widget(Some(&title_lbl));

    let save_btn = gtk4::Button::builder()
        .label("Save")
        .css_classes(["save-btn"])
        .sensitive(false)
        .build();
    header_bar.pack_end(&save_btn);

    // Main content container
    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(24)
        .margin_top(6)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(20)
        .build();

    // 1. Battery Status Card
    let card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(14)
        .css_classes(["battery-card"])
        .build();

    // Level row
    let level_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let level_lbl = gtk4::Label::builder()
        .label(format!("{}%", state.current_capacity.get()))
        .css_classes(["battery-level"])
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .build();
    let status_lbl = gtk4::Label::builder()
        .label(supported_bats[0].status.as_deref().unwrap_or("Unknown"))
        .css_classes(["battery-status"])
        .halign(gtk4::Align::End)
        .build();
    level_row.append(&level_lbl);
    level_row.append(&status_lbl);
    card.append(&level_row);

    // Banner above the limit
    let above_limit_banner = gtk4::Label::builder()
        .css_classes(["above-limit-banner"])
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .build();
    card.append(&above_limit_banner);

    // Battery meter
    let meter = BatteryMeter::new();
    meter.set_level_and_limit(
        state.current_capacity.get() as f64,
        state.active_limit.get() as f64,
    );
    card.append(&meter);

    // Divider
    let divider = gtk4::Box::builder().css_classes(["card-divider"]).build();
    card.append(&divider);

    // Active limit row
    let active_limit_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let left_limit_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .build();
    let active_limit_title = gtk4::Label::builder()
        .label("Active limit")
        .css_classes(["active-limit-label"])
        .xalign(0.0)
        .build();
    let active_limit_sublabel = gtk4::Label::builder()
        .css_classes(["active-limit-sublabel"])
        .xalign(0.0)
        .visible(state.is_override.get())
        .build();
    if state.is_override.get() {
        active_limit_sublabel.set_text(&format!(
            "Until next restart, then {}%",
            read_effective_limit(&paths).0
        ));
    }
    left_limit_box.append(&active_limit_title);
    left_limit_box.append(&active_limit_sublabel);

    let active_limit_val = gtk4::Label::builder()
        .label(format!("{}%", state.active_limit.get()))
        .css_classes(["active-limit-val"])
        .halign(gtk4::Align::End)
        .build();

    active_limit_row.append(&left_limit_box);
    active_limit_row.append(&active_limit_val);
    card.append(&active_limit_row);

    content_box.append(&card);

    // 2. Charge Limit Group
    let group = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();

    let group_header = gtk4::Label::builder()
        .label("Charge limit")
        .css_classes(["group-header"])
        .xalign(0.0)
        .build();
    group.append(&group_header);

    // Chips container
    let chips_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .homogeneous(true)
        .build();

    // 3 Chips: 60, 80, 100
    let chip60 = build_chip(60, "Max lifespan");
    let chip80 = build_chip(80, "Everyday");
    let chip100 = build_chip(100, "Full capacity");

    chips_box.append(&chip60.0);
    chips_box.append(&chip80.0);
    chips_box.append(&chip100.0);
    group.append(&chips_box);

    // Description text
    let desc_lbl = gtk4::Label::builder()
        .css_classes(["preset-desc"])
        .wrap(true)
        .xalign(0.0)
        .build();
    group.append(&desc_lbl);

    // Unsaved notice
    let unsaved_lbl = gtk4::Label::builder()
        .css_classes(["unsaved-warning"])
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .build();
    group.append(&unsaved_lbl);

    content_box.append(&group);

    // Wrapper layout
    let main_wrapper = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    main_wrapper.append(&header_bar);
    main_wrapper.append(&content_box);

    toast_overlay.set_child(Some(&main_wrapper));
    window.set_content(Some(&toast_overlay));

    // Update UI helper
    let update_ui_fn = {
        let state = state.clone();
        let save_btn = save_btn.clone();
        let chip60 = chip60.clone();
        let chip80 = chip80.clone();
        let chip100 = chip100.clone();
        let desc_lbl = desc_lbl.clone();
        let unsaved_lbl = unsaved_lbl.clone();
        let above_limit_banner = above_limit_banner.clone();
        let meter = meter.clone();
        let active_limit_val = active_limit_val.clone();
        let active_limit_sublabel = active_limit_sublabel.clone();
        let paths = ConfigPaths::default();

        Rc::new(move || {
            let sel = state.selected_preset.get();
            let act = state.active_limit.get();
            let cap = state.current_capacity.get();
            let is_ovr = state.is_override.get();

            // Highlight chips
            set_chip_selected(&chip60, sel == 60);
            set_chip_selected(&chip80, sel == 80);
            set_chip_selected(&chip100, sel == 100);

            // Preset description text
            match sel {
                60 => desc_lbl.set_text(
                    "Charging stops at 60%. Best when the charger stays plugged in all day.",
                ),
                80 => desc_lbl.set_text(
                    "Charging stops at 80%, so the battery does not sit at full voltage while the charger stays plugged in.",
                ),
                100 => {
                    let saved = read_effective_limit(&paths).0;
                    desc_lbl.set_text(&format!(
                        "Charges to 100% until the next restart. Then your saved limit of {}% comes back.",
                        saved
                    ));
                }
                _ => {}
            }

            // Unsaved note
            if sel != act {
                unsaved_lbl.set_text(&format!("Not saved. The battery still stops at {}%.", act));
                unsaved_lbl.set_visible(true);
                save_btn.set_sensitive(true);
            } else {
                unsaved_lbl.set_visible(false);
                save_btn.set_sensitive(false);
            }

            // Above limit banner
            if cap > act {
                above_limit_banner.set_text(&format!(
                    "Above the limit. The battery drops to {}% with normal use.",
                    act
                ));
                above_limit_banner.set_visible(true);
            } else {
                above_limit_banner.set_visible(false);
            }

            // Meter & Active limit
            meter.set_level_and_limit(cap as f64, act as f64);
            active_limit_val.set_text(&format!("{}%", act));
            active_limit_sublabel.set_visible(is_ovr);
            if is_ovr {
                let saved = read_effective_limit(&paths).0;
                active_limit_sublabel.set_text(&format!("Until next restart, then {}%", saved));
            }
        })
    };

    // Initial update
    update_ui_fn();

    // Chip Click Handlers
    {
        let state = state.clone();
        let update_ui = update_ui_fn.clone();
        chip60.1.connect_clicked(move |_| {
            state.selected_preset.set(60);
            update_ui();
        });
    }
    {
        let state = state.clone();
        let update_ui = update_ui_fn.clone();
        chip80.1.connect_clicked(move |_| {
            state.selected_preset.set(80);
            update_ui();
        });
    }
    {
        let state = state.clone();
        let update_ui = update_ui_fn.clone();
        chip100.1.connect_clicked(move |_| {
            state.selected_preset.set(100);
            update_ui();
        });
    }

    // Save Button Handler
    {
        let state = state.clone();
        let update_ui = update_ui_fn.clone();
        let toast_overlay = toast_overlay.clone();

        save_btn.connect_clicked(move |_| {
            let limit = state.selected_preset.get();
            let is_once = limit == 100;

            // Spawn pkexec helper
            let mut cmd = Command::new("pkexec");
            // Check if installed binary exists, fallback to local release/debug during development
            let helper_bin = if std::path::Path::new("/usr/local/bin/ncharge-apply").exists() {
                "/usr/local/bin/ncharge-apply".to_string()
            } else {
                let current_exe = std::env::current_exe().unwrap_or_default();
                let dir = current_exe
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."));
                dir.join("ncharge-apply").to_string_lossy().to_string()
            };

            cmd.arg(&helper_bin).arg("apply").arg(limit.to_string());
            if is_once {
                cmd.arg("--once");
            }

            match cmd.status() {
                Ok(status) => match status.code() {
                    Some(0) => {
                        state.active_limit.set(limit);
                        state.is_override.set(is_once);
                        let msg = if is_once {
                            "Charge limit set to 100% until restart".to_string()
                        } else {
                            format!("Charge limit set to {}%", limit)
                        };
                        toast_overlay.add_toast(libadwaita::Toast::new(&msg));
                    }
                    Some(126) => {
                        // User cancelled password dialog
                        state.selected_preset.set(state.active_limit.get());
                        toast_overlay
                            .add_toast(libadwaita::Toast::new("Cancelled. Nothing was changed."));
                    }
                    Some(127) => {
                        // Not authorized
                        state.selected_preset.set(state.active_limit.get());
                        toast_overlay.add_toast(libadwaita::Toast::new("Not authorized"));
                    }
                    _ => {
                        toast_overlay.add_toast(libadwaita::Toast::new(
                            "Could not set the limit. See journalctl -xeu ncharge-apply",
                        ));
                    }
                },
                Err(_) => {
                    toast_overlay.add_toast(libadwaita::Toast::new(
                        "Failed to execute authorization helper",
                    ));
                }
            }
            update_ui();
        });
    }

    // 2-Second Polling Timer to sync live status without spawning processes
    {
        let state = state.clone();
        let level_lbl = level_lbl.clone();
        let status_lbl = status_lbl.clone();
        let update_ui = update_ui_fn.clone();

        glib::timeout_add_seconds_local(2, move || {
            let bats = scan_batteries(DEFAULT_POWER_SUPPLY_ROOT);
            if !bats.is_empty() {
                if let Some(cap) = bats[0].capacity {
                    state.current_capacity.set(cap);
                    level_lbl.set_text(&format!("{}%", cap));
                }
                if let Some(ref st) = bats[0].status {
                    status_lbl.set_text(st);
                }
                if let Some(thr) = bats[0].charge_control_end_threshold {
                    if state.active_limit.get() != thr {
                        state.active_limit.set(thr);
                    }
                }
                let paths = ConfigPaths::default();
                let (_, is_ovr) = read_effective_limit(&paths);
                state.is_override.set(is_ovr);

                update_ui();
            }
            glib::ControlFlow::Continue
        });
    }

    window.present();
}

#[derive(Clone)]
struct ChipWidgets {
    pct_label: gtk4::Label,
    title_label: gtk4::Label,
}

fn build_chip(percent: u8, title: &str) -> (gtk4::Widget, gtk4::Button, ChipWidgets) {
    let btn = gtk4::Button::builder().css_classes(["preset-chip"]).build();

    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .build();

    let pct_lbl = gtk4::Label::builder()
        .label(format!("{}%", percent))
        .css_classes(["preset-percent"])
        .xalign(0.0)
        .build();

    let title_lbl = gtk4::Label::builder()
        .label(title)
        .css_classes(["preset-title"])
        .xalign(0.0)
        .build();

    vbox.append(&pct_lbl);
    vbox.append(&title_lbl);
    btn.set_child(Some(&vbox));

    let widgets = ChipWidgets {
        pct_label: pct_lbl,
        title_label: title_lbl,
    };

    (btn.clone().upcast(), btn, widgets)
}

fn set_chip_selected(chip: &(gtk4::Widget, gtk4::Button, ChipWidgets), selected: bool) {
    if selected {
        chip.1.add_css_class("preset-chip-selected");
        chip.2.pct_label.add_css_class("preset-percent-selected");
        chip.2.title_label.add_css_class("preset-title-selected");
    } else {
        chip.1.remove_css_class("preset-chip-selected");
        chip.2.pct_label.remove_css_class("preset-percent-selected");
        chip.2.title_label.remove_css_class("preset-title-selected");
    }
}

fn build_unsupported_ui() -> gtk4::Box {
    let main_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_top(40)
        .margin_bottom(40)
        .margin_start(28)
        .margin_end(28)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let icon = gtk4::Image::from_icon_name("battery-missing-symbolic");
    icon.set_pixel_size(48);
    icon.set_opacity(0.6);
    main_box.append(&icon);

    let title = gtk4::Label::builder()
        .label("Charge limit not available")
        .css_classes(["unsupported-title"])
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();
    main_box.append(&title);

    let body = gtk4::Label::builder()
        .label("This laptop does not give Linux a charge threshold file. It comes from the vendor driver, such as asus_wmi on ASUS laptops. To check, run:")
        .css_classes(["unsupported-body"])
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();
    main_box.append(&body);

    let code = gtk4::Label::builder()
        .label("ls /sys/class/power_supply/BAT0/")
        .css_classes(["code-box"])
        .selectable(true)
        .build();
    main_box.append(&code);

    main_box
}
