mod commands;
mod config;
mod utils;

use clap::Parser;
use log::info;

fn main() {
    let cfg = config::Config::default();
    info!("Starting CLI app: {}", cfg.app_name);
    commands::run_command(&cfg.app_name);
}
