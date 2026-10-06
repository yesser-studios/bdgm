use bdgm_play::{
    cli::{app::run, args::Args},
    gui::app::run_gui,
};
use clap::Parser;

fn main() {
    let args = Args::parse();

    let status = match args.location {
        Some(_) => match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(run(args)).map(Some),
            Err(e) => Err(e.into()),
        },
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
