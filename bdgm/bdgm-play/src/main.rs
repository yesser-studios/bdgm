use bdgm_play::args::Args;
use clap::Parser;

use crate::{app::run, gui::run_gui};

mod app;
mod gui;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let status = match args.location {
        Some(_) => run(args).await.map(Some),
        None => {
            if args.is_raw_disc() {
                eprintln!(
                    "--raw-disc is selected but no path is specified. Specify a path or remove the argument to launch GUI."
                );
                std::process::exit(1);
            } else if args.image {
                eprintln!(
                    "--image is selected but no path is specified. Specify a path or remove the argument to launch GUI."
                );
                std::process::exit(1);
            } else {
                match run_gui(args) {
                    Ok(_) => Ok(None),
                    Err(e) => {
                        eprintln!("Error while running app: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    };

    match status {
        Ok(status) => {
            if let Some(status) = status {
                if let Some(code) = status.code() {
                    std::process::exit(code);
                } else if !status.success() {
                    // For example on SIGKILL
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
