//! Drift guards (require the `build` feature to regenerate the font).
#![cfg(feature = "build")]

use ntds_icons::build::{FontLayout, build_font};
use ntds_icons::{FONT_TTF, codepoints};
use skrifa::raw::{FontRef, TableProvider};

/// The committed `assets/ntds_icons.ttf` must byte-match a fresh `build_font`.
#[test]
fn committed_font_matches_fresh_build() {
    let fresh = build_font(&FontLayout::default());
    assert_eq!(
        fresh.as_slice(),
        FONT_TTF,
        "assets/ntds_icons.ttf is stale — re-run `cargo run --features build --bin ntds-font-gen`"
    );
}

/// Every codepoint the lookups can return must resolve to a real glyph in the
/// font — chains `codepoints` ↔ `layout` ↔ `build_font` ↔ committed `FONT_TTF`.
#[test]
fn every_lookup_char_is_in_the_font() {
    let font = FontRef::new(FONT_TTF).unwrap();
    let cmap = font.cmap().unwrap();
    for entry in codepoints::all_glyphs() {
        let gid = cmap.map_codepoint(entry.codepoint);
        assert!(
            gid.is_some_and(|g| g.to_u32() != 0),
            "{} ({:#x}) missing from cmap",
            entry.name,
            entry.codepoint as u32
        );
    }
}
