use bdgm_play::{args::Args, cli::app::run, gui::app::run_gui};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let mut args = Args::parse();
    args.runtime = match args.runtime.take() {
        Some(path)
            if path.components().count() == 1
                && matches!(
                    path.components().next(),
                    Some(std::path::Component::Normal(_))
                ) =>
        {
            Some(path)
        }
        Some(p) => Some(std::path::absolute(p)?),
        None => None,
    };

    let status = match args.location.clone() {
        Some(location) => match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(run(args, location)),
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
                    Err(_) => {
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
        Err(_) => {
            std::process::exit(1);
        }
    }

    Ok(())
}
