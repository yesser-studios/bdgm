use crate::core::{State, StateHandler};

#[derive(Default)]
pub struct CliStateHandler {}

impl StateHandler for CliStateHandler {
    fn change_state(&mut self, state: State) {
        println!("{state}");
    }

    fn revert_state(&mut self) {}
}
