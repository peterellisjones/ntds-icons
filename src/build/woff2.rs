//! TTF → WOFF2 via the pure-Rust `ttf2woff2` crate.

use ttf2woff2::{BrotliQuality, encode};

/// Compress a TrueType font to WOFF2.
///
/// Uses the default (deterministic) Brotli quality.
///
/// # Panics
/// Panics if `ttf` is not a valid TrueType font.
#[must_use]
pub fn to_woff2(ttf: &[u8]) -> Vec<u8> {
    encode(ttf, BrotliQuality::default()).expect("woff2 encode of a valid font")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_woff2_magic() {
        let woff2 = to_woff2(crate::FONT_TTF);
        assert!(woff2.len() > 1000, "woff2 suspiciously small: {}", woff2.len());
        assert_eq!(&woff2[0..4], b"wOF2");
    }
}
