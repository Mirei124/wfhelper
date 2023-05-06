use crate::mydbus::Cookies;
use crate::screen_off;
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use std::cell::RefCell;
use std::env;
use std::path::Path;
use std::process;

#[cfg(feature = "dialog")]
fn show_info(title: &str, text: &str) {
    let dialog = gtk::builders::MessageDialogBuilder::new()
        .title(title)
        .text(text)
        .buttons(gtk::ButtonsType::Close)
        .build();
    dialog.run();
    dialog.close();
}

#[cfg(not(feature = "dialog"))]
fn show_info(title: &str, text: &str) {
    process::Command::new("notify-send")
        .args([title, text])
        .spawn()
        .unwrap();
}

pub fn create_tray(inhibit_once: bool) {
    gtk::init().unwrap();

    let indicator = RefCell::new(AppIndicator::new("wfhelper", ""));
    indicator
        .borrow_mut()
        .set_status(AppIndicatorStatus::Active);
    let icon_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("res");
    indicator
        .borrow_mut()
        .set_icon_theme_path(icon_path.to_str().unwrap());
    indicator.borrow_mut().set_icon("run");

    let mut menu = gtk::Menu::new();
    let menu_tog = gtk::MenuItem::with_label("TOGGLE");
    let menu_sta = gtk::MenuItem::with_label("STATUS");
    let menu_scr = gtk::MenuItem::with_label("SCROFF");
    let menu_qui = gtk::MenuItem::with_label("QUIT");
    menu.append(&menu_tog);
    menu.append(&menu_sta);
    menu.append(&menu_scr);
    menu.append(&menu_qui);
    indicator.borrow_mut().set_menu(&mut menu);
    menu.show_all();

    match Cookies::new() {
        Ok(mut cookies) => {
            if inhibit_once {
                cookies.add_inhibit("wfhelper", "keep screen on");
                indicator.borrow_mut().set_icon("pause");
                cookies.state = true;
            }
            let cookies2: RefCell<Cookies> = RefCell::new(cookies);
            menu_tog.connect_activate(move |_| {
                if !cookies2.borrow().state {
                    cookies2
                        .borrow_mut()
                        .add_inhibit("wfhelper", "keep screen on");
                    indicator.borrow_mut().set_icon("pause");
                    cookies2.borrow_mut().state = true;
                } else {
                    cookies2.borrow_mut().rel_inhibit();
                    indicator.borrow_mut().set_icon("run");
                    cookies2.borrow_mut().state = false;
                }
            });
            menu_sta.connect_activate(|_| match Cookies::get_status() {
                Ok(v) => {
                    if v.len() > 0 {
                        show_info("STATUS", &v);
                    } else {
                        show_info("STATUS", "No inhibition");
                    }
                }
                Err(e) => {
                    show_info("ERROR", &*e.to_string());
                }
            });
        }
        Err(e) => {
            menu_tog.set_can_focus(false);
            show_info("ERROR", &e.to_string());
        }
    };

    menu_scr.connect_activate(|_| {
        screen_off::do_off(None);
    });
    menu_qui.connect_activate(|_| {
        process::exit(0);
    });

    gtk::main();
}
