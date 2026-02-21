//! QR code generation and decoding for sync tickets.
//!
//! Provides bidirectional conversion between ticket strings and QR codes:
//! - `ticket_to_qr_lines()`: Encode a ticket into terminal-renderable QR lines
//! - `decode_qr_from_file()`: Decode a QR code image back into a ticket string
//! - `save_qr_to_png()`: Save a QR code as a PNG image file

use std::path::Path;

/// A single line of the QR code rendering, using Unicode half-block characters.
///
/// Each terminal row represents 2 vertical QR modules using:
/// - `▀` (upper half block) = top module dark, bottom module light
/// - `▄` (lower half block) = top module light, bottom module dark
/// - `█` (full block) = both modules dark
/// - ` ` (space) = both modules light
#[derive(Debug, Clone)]
pub struct QrLine {
    pub text: String,
}

// Quiet zone size for terminal display
const TERMINAL_QUIET_ZONE: usize = 1;

// Compile-time assertion that quiet zone is reasonable
const _: () = assert!(TERMINAL_QUIET_ZONE > 0, "quiet zone must be positive");
const _: () = assert!(TERMINAL_QUIET_ZONE < 10, "quiet zone should be small for terminal");

/// Generate QR code lines from a ticket string.
///
/// Returns a Vec of `QrLine` suitable for rendering in a terminal.
/// Each line uses Unicode half-block characters so one terminal row
/// encodes two rows of QR modules, keeping the code compact.
pub fn ticket_to_qr_lines(ticket: &str) -> Result<Vec<QrLine>, String> {
    use qrcode::QrCode;

    let code = QrCode::new(ticket.as_bytes()).map_err(|e| format!("QR encode error: {e}"))?;

    let modules = code.to_colors();
    let width = code.width();
    let quiet = TERMINAL_QUIET_ZONE;

    let total_w = width + 2 * quiet;
    let total_h = width + 2 * quiet;

    debug_assert!(total_w > 0, "postcondition: total width is positive");
    debug_assert!(total_h > 0, "postcondition: total height is positive");

    // Build a boolean grid (true = dark module)
    let mut grid = vec![vec![false; total_w]; total_h];
    for row in 0..width {
        for col in 0..width {
            grid[quiet + row][quiet + col] = modules[row * width + col] == qrcode::Color::Dark;
        }
    }

    // Render pairs of rows using half-block characters.
    // Each terminal row encodes two QR module rows via half-block glyphs.
    let mut lines = Vec::new();
    let mut row = 0;
    while row < total_h {
        let top_row = &grid[row];
        let bot_row = grid.get(row + 1);
        let mut line = String::with_capacity(total_w);
        for (top, bot) in top_row
            .iter()
            .zip(bot_row.map(|r| r.iter()).into_iter().flatten().chain(std::iter::repeat(&false)))
            .take(total_w)
        {
            let ch = match (top, bot) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            };
            line.push(ch);
        }
        lines.push(QrLine { text: line });
        row += 2;
    }

    debug_assert!(!lines.is_empty(), "postcondition: generated at least one line");
    debug_assert!(lines.iter().all(|l| l.text.chars().count() == total_w), 
                  "postcondition: all lines have same character width");

    Ok(lines)
}

/// Compute the rendered dimensions of a QR code for a ticket.
#[allow(dead_code)]
///
/// Returns `(width_cols, height_rows)` — the terminal columns and rows
/// needed to display the QR code.
pub fn qr_dimensions(ticket: &str) -> Result<(u32, u32), String> {
    use qrcode::QrCode;

    let code = QrCode::new(ticket.as_bytes()).map_err(|e| format!("QR encode error: {e}"))?;
    let width = code.width();
    let quiet = 1;
    let total_w = width + 2 * quiet;
    let total_h = width + 2 * quiet;
    
    debug_assert!(total_w > 0, "postcondition: width is positive");
    debug_assert!(total_h > 0, "postcondition: height is positive");
    
    // Each pair of rows -> 1 terminal row
    let term_rows = total_h.div_ceil(2);
    
    let w_u32 = u32::try_from(total_w).expect("QR width should fit in u32");
    let h_u32 = u32::try_from(term_rows).expect("QR height should fit in u32");
    
    Ok((w_u32, h_u32))
}

/// Decode a QR code from an image file and return the contained string.
///
/// Supports PNG and JPEG image formats. The image is converted to grayscale
/// and scanned for QR codes using the `rqrr` library.
pub fn decode_qr_from_file(path: &Path) -> Result<String, String> {
    debug_assert!(path.as_os_str().len() > 0, "precondition: path is non-empty");

    let img = image::open(path).map_err(|e| format!("Failed to open image: {e}"))?;

    let gray = img.to_luma8();
    
    debug_assert!(gray.width() > 0, "postcondition: image has width");
    debug_assert!(gray.height() > 0, "postcondition: image has height");

    let mut prepared = rqrr::PreparedImage::prepare(gray);
    let grids = prepared.detect_grids();

    if grids.is_empty() {
        return Err("No QR code found in image".to_string());
    }

    let (_meta, content) = grids[0]
        .decode()
        .map_err(|e| format!("Failed to decode QR code: {e}"))?;

    debug_assert!(!content.is_empty(), "postcondition: decoded content is non-empty");

    Ok(content)
}

// Image QR code constants
const IMAGE_QUIET_ZONE: usize = 4;
const MODULE_PIXELS: usize = 8;

// Compile-time assertions for image QR code parameters
const _: () = assert!(IMAGE_QUIET_ZONE >= 4, "image quiet zone should be at least 4 for standard compliance");
const _: () = assert!(MODULE_PIXELS >= 1, "module size must be positive");
const _: () = assert!(MODULE_PIXELS <= 32, "module size should be reasonable");

/// Save a QR code as a PNG image file.
///
/// Generates a QR code from the ticket string and saves it as a PNG image.
/// Each QR module is rendered as an 8x8 pixel block for good scannability.
/// Creates parent directories if they don't exist.
///
/// # Arguments
///
/// * `ticket` - The ticket string to encode
/// * `output_path` - The file path where the PNG should be saved
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error message on failure.
pub fn save_qr_to_png(ticket: &str, output_path: &Path) -> Result<(), String> {
    use qrcode::QrCode;
    use image::{GrayImage, Luma};

    debug_assert!(!ticket.is_empty(), "precondition: ticket is non-empty");
    debug_assert!(output_path.as_os_str().len() > 0, "precondition: output path is non-empty");

    // Generate QR code
    let code = QrCode::new(ticket.as_bytes()).map_err(|e| format!("QR encode error: {e}"))?;

    let modules = code.to_colors();
    let width = code.width();
    let quiet = IMAGE_QUIET_ZONE;

    let total_size = width + 2 * quiet;

    // Module size in pixels (8x8 for good scannability)
    let module_px = MODULE_PIXELS;
    let img_size = total_size * module_px;

    debug_assert!(img_size > 0, "postcondition: image size is positive");

    // Create a white image
    let img_size_u32 = u32::try_from(img_size).expect("QR image size should fit in u32");
    let mut img = GrayImage::from_pixel(img_size_u32, img_size_u32, Luma([255u8]));

    // Draw QR modules
    for row in 0..width {
        for col in 0..width {
            if modules[row * width + col] == qrcode::Color::Dark {
                // Draw a black module (8x8 pixels)
                let start_x = (quiet + col) * module_px;
                let start_y = (quiet + row) * module_px;
                
                for dy in 0..module_px {
                    for dx in 0..module_px {
                        let px = start_x + dx;
                        let py = start_y + dy;
                        let px_in_bounds = px < img_size;
                        let py_in_bounds = py < img_size;
                        if px_in_bounds {
                            if py_in_bounds {
                                let px_u32 = u32::try_from(px).expect("pixel x should fit in u32");
                                let py_u32 = u32::try_from(py).expect("pixel y should fit in u32");
                                img.put_pixel(px_u32, py_u32, Luma([0u8]));
                            }
                        }
                    }
                }
            }
        }
    }

    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    }

    // Save to PNG
    img.save(output_path)
        .map_err(|e| format!("Failed to save PNG: {e}"))?;

    debug_assert!(output_path.exists(), "postcondition: output file was created");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_to_qr_roundtrip_structure() {
        let ticket = "irohscii1ABCDEFGHIJKLMNOP";
        let lines = ticket_to_qr_lines(ticket).expect("should encode");

        // Should produce some lines
        assert!(!lines.is_empty());

        // All lines should have same width
        let width = lines[0].text.chars().count();
        for line in &lines {
            assert_eq!(line.text.chars().count(), width);
        }
    }

    #[test]
    fn test_qr_dimensions_consistent() {
        let ticket = "irohscii1TESTTICKET";
        let (w, h) = qr_dimensions(ticket).expect("should compute dimensions");
        let lines = ticket_to_qr_lines(ticket).expect("should encode");

        assert_eq!(lines.len(), h as usize);
        assert_eq!(lines[0].text.chars().count(), w as usize);
    }

    #[test]
    fn test_qr_only_uses_expected_chars() {
        let ticket = "collab1SOMEDATA";
        let lines = ticket_to_qr_lines(ticket).expect("should encode");

        for line in &lines {
            for ch in line.text.chars() {
                assert!(
                    ch == '█' || ch == '▀' || ch == '▄' || ch == ' ',
                    "unexpected character: {ch:?}"
                );
            }
        }
    }

    #[test]
    fn test_decode_nonexistent_file() {
        let result = decode_qr_from_file(Path::new("/nonexistent/qr.png"));
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_ticket_fails() {
        // Empty string should still produce a valid QR (QR codes can encode empty data)
        let result = ticket_to_qr_lines("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_qr_to_png() {
        use tempfile::TempDir;

        let ticket = "irohscii1TESTTICKET";
        let temp_dir = TempDir::new().expect("should create temp dir");
        let path = temp_dir.path().join("test-qr.png");

        // Save QR code
        let result = save_qr_to_png(ticket, &path);
        assert!(result.is_ok(), "save should succeed: {:?}", result);

        // Verify file exists
        assert!(path.exists(), "PNG file should exist");

        // Verify it's a valid PNG by opening it
        let img = image::open(&path);
        assert!(img.is_ok(), "should be a valid PNG image");

        // Verify dimensions are reasonable (should be multiple of 8, the module size)
        let img = img.unwrap();
        assert!(img.width() % 8 == 0, "width should be multiple of module size");
        assert!(img.height() % 8 == 0, "height should be multiple of module size");
        assert!(img.width() > 0, "image should have non-zero width");
        assert!(img.height() > 0, "image should have non-zero height");
    }
}
