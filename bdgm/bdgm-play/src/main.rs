use crate::app::run;

mod app;
mod args;
mod dump;
mod error;
mod server;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
