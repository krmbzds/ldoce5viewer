//! Image rendering layer.

pub mod renderer;
pub use renderer::{
    detect_capability, load_image, render_image, render_kitty, render_placeholder,
    render_with_viuer, ImageError, TerminalCapability,
};
