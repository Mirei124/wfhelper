mod audio_inhibit;
mod break_reminder;
mod mydbus;
mod mytray;
mod screen_off;

use clap::Parser;
use std::process;
use std::thread;

#[derive(Parser)]
#[command(about = "A simple tool")]
struct Cli {
    /// inhibit dpms
    #[arg(short, long)]
    inhibit: bool,

    /// turn off screen
    #[arg(short, long)]
    screen_off: bool,

    /// enable break reminder
    #[arg(short, long)]
    break_reminder: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.screen_off {
        screen_off::do_screen_off(None);
        process::exit(0);
    }

    if cli.break_reminder {
        thread::spawn(|| break_reminder::break_reminder());
    }

    thread::spawn(|| audio_inhibit::audio_auto_inhibit());
    mytray::create_tray(cli.inhibit);
}
