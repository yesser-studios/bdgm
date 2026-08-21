use crate::app::run;

mod app;
mod args;
mod dump;
mod error;

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
