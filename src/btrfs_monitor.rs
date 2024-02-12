use notify_rust::{Hint, Notification, Urgency};
use regex::Regex;
use std::{process::Command, thread::sleep, time::Duration};

pub fn monitor_usage() {
    sleep(Duration::from_secs(60));
    loop {
        let raw_size = obtain_unallocated("/").expect("Obtain unallocated failed");
        if raw_size <= 2 * 1024 * 1024 * 1024 {
            Notification::new()
                .summary("Btrfs warning")
                .body(&format!(
                    "Btrfs device unallocated is low ({})",
                    format_size(raw_size)
                ))
                .icon("gnome-warning")
                .appname("wfhelper")
                .hint(Hint::Urgency(Urgency::Critical))
                .show()
                .unwrap();
            break;
        }
        sleep(Duration::from_secs(3600));
    }
}

fn obtain_unallocated(path: &str) -> Option<u64> {
    let output = Command::new("btrfs")
        .args(["filesystem", "usage", "--raw", path])
        .output()
        .expect("Failed to get btrfs filesystem usage");
    let output_str = String::from_utf8(output.stdout).unwrap();

    let re = Regex::new(r"Device unallocated:\s+(\d+)").unwrap();
    let caps = re.captures(&output_str).unwrap();

    caps.get(1)
        .map_or(None, |m| Some(m.as_str().parse::<u64>().unwrap()))
}

fn format_size(mut size: u64) -> String {
    let unit_list = ["", "K", "M", "G", "T"];
    let mut flag = 0;
    for _ in 0..unit_list.len() - 1 {
        if size >= 1024 {
            size /= 1024;
            flag += 1;
        } else {
            break;
        }
    }
    format!("{}{}B", size, unit_list[flag])
}
