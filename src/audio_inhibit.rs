use crate::mydbus::Cookies;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub fn audio_auto_inhibit() {
    let mut cookie = Cookies::new().unwrap();
    let mut audio_inhibit = false;
    loop {
        if get_audio_play_status() {
            if !audio_inhibit {
                cookie.add_inhibit("wfhelper", "audio playing");
                audio_inhibit = true;
            }
        } else {
            if audio_inhibit {
                cookie.rel_inhibit();
                audio_inhibit = false;
            }
        }
        sleep(Duration::from_secs(30));
    }
}

fn get_audio_play_status() -> bool {
    let output = Command::new("pactl")
        .args(["list", "sink-inputs"])
        .output()
        .expect("execute pactl failed");
    let output_str = String::from_utf8(output.stdout).unwrap();
    output_str.contains("Corked: no")
}
