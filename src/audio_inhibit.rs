use crate::mydbus::PMCookies;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub fn audio_auto_inhibit() {
    let mut audio_cookies = PMCookies::new().unwrap();
    loop {
        if get_audio_play_status() {
            if !audio_cookies.get_inhibit_status() {
                audio_cookies.add_inhibit("wfhelper", "audio playing");
            }
        } else {
            if audio_cookies.get_inhibit_status() {
                audio_cookies.release_inhibit();
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
