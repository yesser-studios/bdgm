mod app;
mod args;

fn main() {
    match app::run() {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
