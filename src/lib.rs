mod common;
mod error;
pub mod kitty_graphics;
pub mod plotting;
pub mod terminal_commands;
mod window_ctrl;

// re-export here to hide implicitly public unsafe functions
pub use error::{Error, Result};
pub use window_ctrl::{WindowSize, get_window_size};
