#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
mod common;
mod error;
mod kitty_graphics;
mod plot;
/// The building blocks: series, graphs, styles and the canvas they are drawn on.
pub mod plotting;
pub mod prelude;
pub mod terminal;
mod terminal_commands;
mod window_ctrl;

pub use error::{Error, Result};
pub use plot::{DEFAULT_PNG_SIZE, Plot};
pub use window_ctrl::{WindowSize, get_window_size};
