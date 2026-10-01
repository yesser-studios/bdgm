use crate::app::run;

mod app;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
