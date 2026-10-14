use bdgm_play::{
    cli::{app::run, args::Args},
    gui::app::run_gui,
};
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
                #[cfg(windows)]
                let console_hidden =
                    bdgm_play::gui::console::hide_console_if_launched_without_shell();
                match run_gui(args) {
                    Ok(_) => Ok(None),
                    Err(e) => {
                        eprintln!("Error while running app: {e}");
                        // The console is gone in Explorer launches, so surface
                        // the failure in a dialog instead of losing it.
                        #[cfg(windows)]
                        if console_hidden {
                            rfd::MessageDialog::new()
                                .set_title("bdgm-play error")
                                .set_description(format!("Error while running app: {e}"))
                                .set_level(rfd::MessageLevel::Error)
                                .show();
                        }
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

    Ok(())
}
