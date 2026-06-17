//! Render a specimen PNG of the real font, colored by affiliation.
//!
//! Loads the generated TTF into usvg's fontdb and rasterizes a grid of base
//! symbols (rows = class, columns = affiliation) via resvg — genuine proof the
//! font renders, not the source geometry.

use resvg::{tiny_skia, usvg};

use crate::codepoints::glyph;
use crate::shapes::{NtdsShapeClass, ShapeAffiliation};

// Specimen palette (this is a standalone preview, not dronecom UI).
const BG: &str = "#0d1117";
const LABEL: &str = "#8b949e";
const TITLE: &str = "#e6edf3";

const fn aff_color(aff: ShapeAffiliation) -> &'static str {
    match aff {
        ShapeAffiliation::Friend => "#4aa3ff",
        ShapeAffiliation::Enemy => "#ff5a5a",
        ShapeAffiliation::Unknown => "#ffd23f",
        ShapeAffiliation::Neutral => "#4ade80",
    }
}

const FONT_FAMILY: &str = "NTDS Icons";
const CELL: u32 = 90;
const LEFT: u32 = 120;
const TOP: u32 = 90;
const GLYPH_SIZE: u32 = 56;

const fn dims() -> (u32, u32) {
    let cols = ShapeAffiliation::ALL.len() as u32;
    let rows = NtdsShapeClass::ALL.len() as u32;
    (LEFT + cols * CELL, TOP + rows * CELL + 20)
}

/// Render the specimen as PNG bytes.
///
/// # Panics
/// Panics if the SVG fails to parse or the pixmap cannot be allocated.
#[must_use]
pub fn render_specimen(ttf: &[u8]) -> Vec<u8> {
    let (width, height) = dims();
    let svg = build_svg(width, height);
    rasterize(ttf, &svg, width, height).encode_png().expect("png encode")
}

/// Rasterize the specimen SVG with the NTDS font (for glyphs) and system fonts
/// (for the sans-serif labels) loaded.
///
/// System fonts are best-effort: the symbols render regardless; only the text
/// labels need them. The committed PNG is a convenience artifact (not
/// byte-asserted), so this mild environment dependence is acceptable.
fn rasterize(ttf: &[u8], svg: &str, width: u32, height: u32) -> tiny_skia::Pixmap {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    opt.fontdb_mut().load_font_data(ttf.to_vec());
    let tree = usvg::Tree::from_str(svg, &opt).expect("specimen svg parses");
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("pixmap alloc");
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap
}

fn build_svg(width: u32, height: u32) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\">"
    ));
    s.push_str(&format!("<rect width=\"{width}\" height=\"{height}\" fill=\"{BG}\"/>"));

    // Title.
    s.push_str(&format!(
        "<text x=\"20\" y=\"44\" font-family=\"sans-serif\" font-size=\"30\" \
         font-weight=\"bold\" fill=\"{TITLE}\">NTDS Icons</text>"
    ));

    // Affiliation column headers.
    for (col, aff) in ShapeAffiliation::ALL.into_iter().enumerate() {
        let cx = LEFT + col as u32 * CELL + CELL / 2;
        s.push_str(&format!(
            "<text x=\"{cx}\" y=\"{y}\" font-family=\"sans-serif\" font-size=\"13\" \
             text-anchor=\"middle\" fill=\"{}\">{aff:?}</text>",
            aff_color(aff),
            y = TOP - 12
        ));
    }

    for (row, class) in NtdsShapeClass::ALL.into_iter().enumerate() {
        let cy = TOP + row as u32 * CELL + CELL / 2;
        // Row (class) label.
        s.push_str(&format!(
            "<text x=\"16\" y=\"{ly}\" font-family=\"sans-serif\" font-size=\"13\" \
             fill=\"{LABEL}\">{class:?}</text>",
            ly = cy + 4
        ));
        for (col, aff) in ShapeAffiliation::ALL.into_iter().enumerate() {
            let cx = LEFT + col as u32 * CELL + CELL / 2;
            // Glyph baseline sits below the cell center so the symbol (centered
            // ~0.38em above the baseline) lands near the cell center.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let baseline = cy + (0.38 * GLYPH_SIZE as f32) as u32;
            let ch = glyph(class, aff) as u32;
            s.push_str(&format!(
                "<text x=\"{cx}\" y=\"{baseline}\" font-family=\"{FONT_FAMILY}\" \
                 font-size=\"{GLYPH_SIZE}\" text-anchor=\"middle\" fill=\"{}\">&#x{ch:X};</text>",
                aff_color(aff)
            ));
        }
    }

    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_png() {
        let png = render_specimen(crate::FONT_TTF);
        assert!(png.len() > 2000, "png suspiciously small: {}", png.len());
        assert_eq!(&png[0..8], &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']);
    }

    #[test]
    fn specimen_is_not_blank() {
        let (width, height) = dims();
        let pixmap = rasterize(crate::FONT_TTF, &build_svg(width, height), width, height);
        // Background is #0d1117; count pixels that differ from it.
        let differing = pixmap
            .pixels()
            .iter()
            .filter(|p| !(p.red() == 0x0d && p.green() == 0x11 && p.blue() == 0x17))
            .count();
        assert!(differing > 5000, "specimen looks blank: only {differing} non-bg pixels");
    }
}
