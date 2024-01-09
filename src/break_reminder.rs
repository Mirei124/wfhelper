use notify_rust::{Hint, Notification, Urgency};
use std::thread::sleep;
use std::time::Duration;

pub fn break_reminder() {
    let inter_minute = 25;
    loop {
        sleep(Duration::from_secs(inter_minute * 60));
        Notification::new()
            .summary("Rest Notice")
            .body(&format!("您已连续注视屏幕{}分钟", inter_minute))
            .icon("xeyes")
            .appname("wfhelper")
            .hint(Hint::Urgency(Urgency::Critical))
            .show()
            .unwrap();
    }
}
