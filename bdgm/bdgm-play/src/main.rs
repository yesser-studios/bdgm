use crate::app::run;

mod app;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(status) => {
            if let Some(code) = status.code() {
                std::process::exit(code);
            } else if !status.success() {
                // For example on SIGKILL
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
