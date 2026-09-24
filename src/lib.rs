mod common;
mod error;
mod kitty_graphics;
mod plot;
pub mod plotting;
pub mod prelude;
pub mod terminal;
mod terminal_commands;
mod window_ctrl;

pub use error::{Error, Result};
pub use plot::{DEFAULT_PNG_SIZE, Plot};
pub use window_ctrl::{WindowSize, get_window_size};
