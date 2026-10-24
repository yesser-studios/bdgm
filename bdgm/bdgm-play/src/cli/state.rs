use crate::core::{State, StateHandler};

#[derive(Default)]
pub struct CliStateHandler {}

impl StateHandler for CliStateHandler {
    /// Prints a preparation state to stdout.
    fn change_state(&mut self, state: State) {
        println!("{state}");
    }

    /// Leaves prior CLI output in place when a temporary state ends.
    fn revert_state(&mut self) {}
}
