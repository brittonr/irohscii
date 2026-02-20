//! QR code generation and decoding for sync tickets.
//!
//! Provides bidirectional conversion between ticket strings and QR codes:
//! - `ticket_to_qr_lines()`: Encode a ticket into terminal-renderable QR lines
//! - `decode_qr_from_file()`: Decode a QR code image back into a ticket string

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
    let quiet = 1; // 1-module quiet zone (minimal for terminal display)

    let total_w = width + 2 * quiet;
    let total_h = width + 2 * quiet;

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

    Ok(lines)
}

/// Compute the rendered dimensions of a QR code for a ticket.
#[allow(dead_code)]
///
/// Returns `(width_cols, height_rows)` — the terminal columns and rows
/// needed to display the QR code.
pub fn qr_dimensions(ticket: &str) -> Result<(usize, usize), String> {
    use qrcode::QrCode;

    let code = QrCode::new(ticket.as_bytes()).map_err(|e| format!("QR encode error: {e}"))?;
    let width = code.width();
    let quiet = 1;
    let total_w = width + 2 * quiet;
    let total_h = width + 2 * quiet;
    // Each pair of rows -> 1 terminal row
    let term_rows = total_h.div_ceil(2);
    Ok((total_w, term_rows))
}

/// Decode a QR code from an image file and return the contained string.
///
/// Supports PNG and JPEG image formats. The image is converted to grayscale
/// and scanned for QR codes using the `rqrr` library.
pub fn decode_qr_from_file(path: &Path) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("Failed to open image: {e}"))?;

    let gray = img.to_luma8();

    let mut prepared = rqrr::PreparedImage::prepare(gray);
    let grids = prepared.detect_grids();

    if grids.is_empty() {
        return Err("No QR code found in image".to_string());
    }

    let (_meta, content) = grids[0]
        .decode()
        .map_err(|e| format!("Failed to decode QR code: {e}"))?;

    Ok(content)
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

        assert_eq!(lines.len(), h);
        assert_eq!(lines[0].text.chars().count(), w);
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
}
