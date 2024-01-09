use std::env;
use std::path::Path;

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use notify_rust::Notification;

use crate::mydbus::PMCookies;
use crate::screen_off::do_screen_off;
use std::sync::{Arc, Mutex};

pub fn create_tray(inhibit_on_start: bool) {
    gtk::init().unwrap();

    let indicator_ = Arc::new(Mutex::new(AppIndicator::new("wfhelper", "")));
    let mut indicator = indicator_.lock().unwrap();

    indicator.set_status(AppIndicatorStatus::Active);
    indicator.set_icon_theme_path(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("res")
            .to_str()
            .unwrap(),
    );
    indicator.set_icon("run");

    let mut menu = gtk::Menu::new();
    let menu_toggle = gtk::MenuItem::with_label("Toggle");
    let menu_status = gtk::MenuItem::with_label("Status");
    let menu_screen_off = gtk::MenuItem::with_label("ScrOff");
    let menu_quit = gtk::MenuItem::with_label("Quit");
    menu.append(&menu_toggle);
    menu.append(&menu_status);
    menu.append(&menu_screen_off);
    menu.append(&menu_quit);
    menu.show_all();

    indicator.set_menu(&mut menu);

    let cookies = Arc::new(Mutex::new(PMCookies::new().unwrap()));
    if inhibit_on_start {
        cookies
            .lock()
            .unwrap()
            .add_inhibit("wfhelper", "keep screen on");
        indicator.set_icon("pause");
    }

    let cookies_toggle = Arc::clone(&cookies);
    let indicator_toggle = Arc::clone(&indicator_);
    menu_toggle.connect_activate(move |_| {
        let mut cookies = cookies_toggle.lock().unwrap();
        let mut indicator = indicator_toggle.lock().unwrap();
        if !cookies.get_inhibit_status() {
            cookies.add_inhibit("wfhelper", "keep screen on");
            indicator.set_icon("pause");
        } else {
            cookies.release_inhibit();
            indicator.set_icon("run");
        }
    });

    menu_status.connect_activate(|_| {
        let status = PMCookies::get_inhibitions().unwrap();
        let mut status_str = String::new();
        if status.len() > 0 {
            status_str.push_str(&status.join("\n"));
        } else {
            status_str.push_str("No inhibition");
        }
        Notification::new()
            .summary("Inhibitions")
            .body(&status_str)
            .icon("computer")
            .appname("wfhelper")
            .show()
            .unwrap();
    });

    menu_screen_off.connect_activate(|_| {
        do_screen_off(None);
    });

    menu_quit.connect_activate(|_| {
        gtk::main_quit();
    });

    drop(indicator);
    gtk::main();
}
