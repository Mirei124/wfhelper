mod mydbus;
mod mytray;
mod screen_off;
use clap::Parser;
use std::process;

#[derive(Parser)]
#[command(about = "A simple tool")]
struct Cli {
    /// inhibit dpms
    #[arg(short, long)]
    inhibit: bool,

    /// turn off screen
    #[arg(short, long)]
    screen_off: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.screen_off {
        screen_off::do_off(None);
        process::exit(0);
    }

    mytray::create_tray(cli.inhibit);
}
