//! `char`-returning PUA codepoint lookups on the crate's own enums.
//!
//! Every value derives from [`crate::layout`], so these lookups and the
//! generated font cannot drift. Lookups return [`char`] (not `&'static str`
//! or any `dc_types`/egui type) — consumers own their own UI wiring.

use crate::layout::{self, GlyphKey, NUM_HEADING_GLYPHS};
use crate::shapes::{BdaDecoration, GroupNtdsShapeClass, NtdsShapeClass, ShapeAffiliation};

/// Construction-centered base symbol. Composes with [`heading_glyph`] /
/// [`bda_glyph`] overlays (zero-advance combining glyphs).
#[must_use]
pub fn glyph(class: NtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Base { class, aff })
}

/// Perceived-center base symbol — vertically centered for buttons and labels.
/// NOT compatible with overlay composition; use [`glyph`] for tactical display.
#[must_use]
pub fn geometric_glyph(class: NtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Geometric { class, aff })
}

/// Construction-centered group symbol (doubled concentric outline). Composes
/// with [`heading_glyph`]; BDA overlays are not supported for groups.
#[must_use]
pub fn group_glyph(class: GroupNtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Group { class, aff })
}

/// Perceived-center group symbol — for buttons and labels; no overlay support.
#[must_use]
pub fn group_geometric_glyph(class: GroupNtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::GroupGeometric { class, aff })
}

/// BDA (battle-damage-assessment) overlay glyph. Zero-advance — concatenate
/// after a base glyph and it renders on top, like a combining mark.
#[must_use]
pub fn bda_glyph(decoration: BdaDecoration) -> char {
    layout::codepoint(GlyphKey::Bda(decoration))
}

/// Flat unknown-class glyph: a bare affiliation outline with no interior.
#[must_use]
pub fn unknown_glyph(aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Unknown { aff })
}

/// Perceived-center flat unknown-class glyph — for buttons and labels.
#[must_use]
pub fn unknown_geometric_glyph(aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::UnknownGeometric { aff })
}

/// Heading-vector overlay glyph for the nearest 5° increment.
///
/// `heading_degrees` is clockwise from north (nautical convention); negative
/// values and values `>= 360` wrap. Zero-advance — concatenate after a base
/// glyph and it renders on top, like a combining accent.
#[must_use]
pub fn heading_glyph(heading_degrees: f32) -> char {
    let normalized = heading_degrees.rem_euclid(360.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = ((normalized / 5.0).round() as usize % NUM_HEADING_GLYPHS as usize) as u8;
    layout::codepoint(GlyphKey::Heading(index))
}

/// One entry in the full glyph table (drives `codepoints.json`, the README
/// cheat-sheet, and any future gallery).
pub struct GlyphEntry {
    /// The PUA codepoint this glyph is mapped to.
    pub codepoint: char,
    /// The stable glyph name (e.g. `"Air.Friend"`, `"heading.135"`).
    pub name: String,
    /// The glyph's semantic identity.
    pub key: GlyphKey,
}

/// Every glyph in codepoint order.
pub fn all_glyphs() -> impl Iterator<Item = GlyphEntry> {
    layout::keys_in_order().into_iter().map(|key| GlyphEntry {
        codepoint: layout::codepoint(key),
        name: layout::name(key),
        key,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const AFFS: [ShapeAffiliation; 4] = ShapeAffiliation::ALL;

    #[test]
    fn base_glyphs_unique() {
        let mut seen = HashSet::new();
        for class in NtdsShapeClass::ALL {
            for aff in AFFS {
                assert!(seen.insert(glyph(class, aff)), "dup {class:?}/{aff:?}");
            }
        }
    }

    #[test]
    fn base_and_geometric_differ() {
        for class in NtdsShapeClass::ALL {
            for aff in AFFS {
                assert_ne!(glyph(class, aff), geometric_glyph(class, aff));
            }
        }
    }

    #[test]
    fn headings_distinct_from_symbols_and_bda() {
        let mut symbols = HashSet::new();
        for class in NtdsShapeClass::ALL {
            for aff in AFFS {
                symbols.insert(glyph(class, aff));
            }
        }
        symbols.insert(bda_glyph(BdaDecoration::Uncertain));
        symbols.insert(bda_glyph(BdaDecoration::ProbablyDestroyed));
        for i in 0u16..72 {
            assert!(!symbols.contains(&heading_glyph(f32::from(i) * 5.0)));
        }
    }

    #[test]
    fn group_glyphs_distinct_from_individual() {
        let mut individual = HashSet::new();
        for class in NtdsShapeClass::ALL {
            for aff in AFFS {
                individual.insert(glyph(class, aff));
                individual.insert(geometric_glyph(class, aff));
            }
        }
        for class in GroupNtdsShapeClass::ALL {
            for aff in AFFS {
                assert!(!individual.contains(&group_glyph(class, aff)));
                assert!(!individual.contains(&group_geometric_glyph(class, aff)));
            }
        }
    }

    #[test]
    fn heading_glyph_wraps_and_rounds() {
        assert_eq!(heading_glyph(0.0) as u32, 0xE022);
        assert_eq!(heading_glyph(-5.0), heading_glyph(355.0));
        assert_eq!(heading_glyph(360.0), heading_glyph(0.0));
        assert_eq!(heading_glyph(2.0), heading_glyph(0.0));
        assert_eq!(heading_glyph(3.0), heading_glyph(5.0));
        // 135° / 5 = 27 → E022 + 27 = E03D (legacy logo anchor).
        assert_eq!(heading_glyph(135.0) as u32, 0xE03D);
    }

    #[test]
    fn all_glyphs_has_170_in_order() {
        let v: Vec<_> = all_glyphs().collect();
        assert_eq!(v.len(), 170);
        assert_eq!(v[0].codepoint as u32, 0xE000);
        assert_eq!(v[169].codepoint as u32, 0xE0A9);
    }
}
