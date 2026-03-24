mod app;
mod api;
mod assets;
mod collector;
mod model;
mod platform;
mod report;
mod storage;
mod web;

use anyhow::Result;
use app::run;
use clap::{Parser, Subcommand};
use model::KeyboardLayout;

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}

#[derive(Parser)]
#[command(name = "keystroke-visualizer")]
#[command(about = "Record global keypress sessions and render a local HTML report.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Start {
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum, default_value_t = KeyboardLayout::Ansi104)]
        layout: KeyboardLayout,
    },
    Status,
    Stop {
        #[arg(long)]
        open: bool,
    },
    Report {
        session_id: String,
        #[arg(long)]
        open: bool,
    },
    List,
    Doctor,
    #[command(hide = true)]
    Daemon {
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        control_token: String,
    },
}
