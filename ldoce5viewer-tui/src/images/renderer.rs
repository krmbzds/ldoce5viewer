//! Terminal image rendering.
//!
//! Tries to render images using:
//! 1. **Kitty graphics protocol** (most capable, true-colour pixels)
//! 2. **Sixel** (supported by many terminals: xterm, mlterm, VTE-based)
//! 3. **Text placeholder** (fallback for terminals without image support)
//!
//! The capability is detected once at startup and cached.

use std::io::Write;
use std::sync::OnceLock;

use base64::engine::Engine;
use image::{DynamicImage, ImageFormat};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("Failed to decode image: {0}")]
    Decode(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// --------------------------------------------------------------------------
// Terminal capability detection
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCapability {
    Kitty,
    Sixel,
    TextOnly,
}

static CAPABILITY: OnceLock<TerminalCapability> = OnceLock::new();

/// Detect the terminal's image capability (cached after the first call).
pub fn detect_capability() -> TerminalCapability {
    *CAPABILITY.get_or_init(|| {
        // Check $TERM and $TERM_PROGRAM env vars as a quick heuristic.
        // A proper detection would involve sending a DA2 query, but that
        // requires raw terminal access we don't have here at init time.
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();

        if term_program.to_lowercase().contains("kitty")
            || term.to_lowercase().contains("kitty")
            || std::env::var("KITTY_WINDOW_ID").is_ok()
        {
            TerminalCapability::Kitty
        } else if term.contains("xterm")
            || term.contains("vte")
            || term_program.to_lowercase().contains("iterm")
            || std::env::var("TERM_PROGRAM")
                .unwrap_or_default()
                .to_lowercase()
                .contains("iterm")
        {
            // Many xterm variants support Sixel; we probe $TERM for common ones
            TerminalCapability::Sixel
        } else {
            TerminalCapability::TextOnly
        }
    })
}

// --------------------------------------------------------------------------
// Image loading
// --------------------------------------------------------------------------

/// Decode image bytes into a `DynamicImage`.
pub fn load_image(data: &[u8]) -> Result<DynamicImage, ImageError> {
    image::load_from_memory(data).map_err(|e| ImageError::Decode(e.to_string()))
}

// --------------------------------------------------------------------------
// Kitty protocol renderer
// --------------------------------------------------------------------------

/// Render `img` to the current terminal position using the Kitty Graphics
/// Protocol.  `max_cols` and `max_rows` are the cell dimensions available.
pub fn render_kitty<W: Write>(
    out: &mut W,
    img: &DynamicImage,
    max_cols: u32,
    max_rows: u32,
) -> Result<(), ImageError> {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    // Encode raw RGBA as base64
    let raw: Vec<u8> = rgba.into_raw();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&raw);

    // Send in chunks of 4096 characters (Kitty spec recommends ≤ 4096)
    const CHUNK: usize = 4096;
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(CHUNK)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i + 1 < chunks.len() { 1u8 } else { 0u8 };
        if i == 0 {
            // First chunk: include all parameters
            write!(
                out,
                "\x1b_Ga=T,f=32,s={w},v={h},c={max_cols},r={max_rows},m={more};{chunk}\x1b\\"
            )?;
        } else {
            write!(out, "\x1b_Gm={more};{chunk}\x1b\\")?;
        }
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Sixel renderer  (simple, using the `viuer` crate if available)
// --------------------------------------------------------------------------

/// Render `img` using viuer (handles both Kitty and Sixel depending on
/// detected capability).
pub fn render_with_viuer(img: &DynamicImage, x: u16, y: u16, width: u32, height: u32) -> Result<(), ImageError> {
    let config = viuer::Config {
        absolute_offset: true,
        x,
        y: y as i16,
        width: Some(width),
        height: Some(height),
        ..Default::default()
    };
    viuer::print(img, &config)
        .map(|_| ())
        .map_err(|e| ImageError::Decode(e.to_string()))
}

// --------------------------------------------------------------------------
// Text placeholder fallback
// --------------------------------------------------------------------------

/// Write a text placeholder for terminals without image support.
pub fn render_placeholder<W: Write>(out: &mut W, filename: &str) -> Result<(), ImageError> {
    write!(out, "[image: {filename}]")?;
    Ok(())
}

// --------------------------------------------------------------------------
// High-level render function
// --------------------------------------------------------------------------

/// Render `data` (raw image bytes) at terminal position `(x, y)` within a
/// `width × height` cell area, using the best available method.
pub fn render_image(
    data: &[u8],
    x: u16,
    y: u16,
    width: u32,
    height: u32,
    filename: &str,
) -> Result<(), ImageError> {
    match detect_capability() {
        TerminalCapability::Kitty | TerminalCapability::Sixel => {
            let img = load_image(data)?;
            render_with_viuer(&img, x, y, width, height)
        }
        TerminalCapability::TextOnly => {
            let mut stdout = std::io::stdout();
            render_placeholder(&mut stdout, filename)
        }
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    #[test]
    fn test_detect_capability_does_not_panic() {
        let _cap = detect_capability();
    }

    #[test]
    fn test_detect_returns_valid_variant() {
        let cap = detect_capability();
        matches!(
            cap,
            TerminalCapability::Kitty | TerminalCapability::Sixel | TerminalCapability::TextOnly
        );
    }

    #[test]
    fn test_load_image_invalid_data() {
        let result = load_image(b"not an image");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_image_valid_png() {
        // Generate a 1×1 red PNG programmatically to ensure it's a valid image
        let img = DynamicImage::new_rgb8(1, 1);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        let png_bytes = buf.into_inner();
        let result = load_image(&png_bytes);
        assert!(result.is_ok(), "1×1 PNG should load: {:?}", result.err());
    }

    #[test]
    fn test_render_placeholder() {
        let mut buf = Vec::new();
        render_placeholder(&mut buf, "test.jpg").unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("test.jpg"));
    }

    #[test]
    fn test_kitty_render_produces_escape_codes() {
        // Create a tiny 2×2 RGBA image
        let img = DynamicImage::new_rgba8(2, 2);
        let mut buf = Vec::new();
        render_kitty(&mut buf, &img, 20, 5).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // Should start with the Kitty APC escape
        assert!(s.contains("\x1b_G"), "missing Kitty escape sequence");
        assert!(s.contains("\x1b\\"),  "missing Kitty string terminator");
    }
}
