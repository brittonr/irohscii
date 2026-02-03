//! ASCII and SVG export for irohscii.
//!
//! This crate provides file I/O functionality:
//! - ASCII art rendering to text files
//! - SVG export with proper shape rendering

mod ascii;
mod svg;

pub use ascii::{load_ascii, save_ascii};
pub use svg::{export_svg, save_svg};

// Re-export core types for convenience
pub use irohscii_core::{Position, ShapeColor, ShapeKind, ShapeView};
