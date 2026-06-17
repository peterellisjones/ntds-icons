//! Single source of truth for the NTDS font's glyph set and PUA codepoint
//! order.
//!
//! Glyphs occupy `U+E000..=U+E0A9` contiguously, in [`keys_in_order`] order;
//! position N maps to codepoint `0xE000 + N` and glyph id `N + 1` (glyph 0 is
//! `.notdef`). Both the font builder (`build::build_font`) and the
//! [`crate::codepoints`] lookups derive from here, so the lookups and the font
//! cannot drift.

use std::{collections::HashMap, sync::OnceLock};

use crate::shapes::{BdaDecoration, GroupNtdsShapeClass, NtdsShapeClass, ShapeAffiliation};

/// First Private Use Area codepoint.
pub const PUA_START: u32 = 0xE000;

/// Heading-vector overlay glyph count (one per 5° step over 360°).
pub const NUM_HEADING_GLYPHS: u8 = 72;

/// Semantic identity of one glyph in the font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphKey {
    /// Construction-centered base symbol (composes with heading/BDA overlays).
    Base { class: NtdsShapeClass, aff: ShapeAffiliation },
    /// Battle-damage-assessment overlay (zero advance).
    Bda(BdaDecoration),
    /// Heading-vector overlay at `5° × index` (zero advance).
    Heading(u8),
    /// Perceived-center base symbol (for buttons/labels; no overlay support).
    Geometric { class: NtdsShapeClass, aff: ShapeAffiliation },
    /// Construction-centered group symbol (doubled outline).
    Group { class: GroupNtdsShapeClass, aff: ShapeAffiliation },
    /// Perceived-center group symbol.
    GroupGeometric { class: GroupNtdsShapeClass, aff: ShapeAffiliation },
    /// Flat unknown-class — bare affiliation outline, no interior.
    Unknown { aff: ShapeAffiliation },
    /// Perceived-center flat unknown-class.
    UnknownGeometric { aff: ShapeAffiliation },
}

/// Horizontal advance class: base symbols advance the pen; BDA + heading
/// overlays are zero-advance so they composite onto the preceding glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    Full,
    Zero,
}

impl GlyphKey {
    /// Whether this glyph advances the pen (base symbol) or composites onto the
    /// preceding glyph (overlay).
    #[must_use]
    pub fn advance(self) -> Advance {
        match self {
            GlyphKey::Bda(_) | GlyphKey::Heading(_) => Advance::Zero,
            GlyphKey::Base { .. }
            | GlyphKey::Geometric { .. }
            | GlyphKey::Group { .. }
            | GlyphKey::GroupGeometric { .. }
            | GlyphKey::Unknown { .. }
            | GlyphKey::UnknownGeometric { .. } => Advance::Full,
        }
    }

    /// The affiliation this glyph belongs to, or `None` for the
    /// affiliation-agnostic overlays (BDA, heading).
    #[must_use]
    pub fn affiliation(self) -> Option<ShapeAffiliation> {
        match self {
            GlyphKey::Base { aff, .. }
            | GlyphKey::Geometric { aff, .. }
            | GlyphKey::Group { aff, .. }
            | GlyphKey::GroupGeometric { aff, .. }
            | GlyphKey::Unknown { aff }
            | GlyphKey::UnknownGeometric { aff } => Some(aff),
            GlyphKey::Bda(_) | GlyphKey::Heading(_) => None,
        }
    }
}

/// Every glyph in cmap / glyph-id order.
///
/// Category order, and the within-category (class outer, affiliation inner)
/// order, match dronecom's `dc_ntds_icons/build.rs` exactly — so sequential
/// codepoint assignment from `PUA_START` reproduces the legacy codepoints.
#[must_use]
pub fn keys_in_order() -> Vec<GlyphKey> {
    let mut k = Vec::with_capacity(170);
    // 1. base, construction-centered (E000–E01F)
    for class in NtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::Base { class, aff });
        }
    }
    // 2. BDA overlays (E020–E021)
    for dec in [BdaDecoration::Uncertain, BdaDecoration::ProbablyDestroyed] {
        k.push(GlyphKey::Bda(dec));
    }
    // 3. heading overlays (E022–E069)
    for i in 0..NUM_HEADING_GLYPHS {
        k.push(GlyphKey::Heading(i));
    }
    // 4. base, geometric-centered (E06A–E089)
    for class in NtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::Geometric { class, aff });
        }
    }
    // 5. group, construction-centered (E08A–E095)
    for class in GroupNtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::Group { class, aff });
        }
    }
    // 6. group, geometric-centered (E096–E0A1)
    for class in GroupNtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::GroupGeometric { class, aff });
        }
    }
    // 7. unknown-class, construction (E0A2–E0A5)
    for aff in ShapeAffiliation::ALL {
        k.push(GlyphKey::Unknown { aff });
    }
    // 8. unknown-class, geometric (E0A6–E0A9)
    for aff in ShapeAffiliation::ALL {
        k.push(GlyphKey::UnknownGeometric { aff });
    }
    k
}

fn codepoint_map() -> &'static HashMap<GlyphKey, char> {
    static MAP: OnceLock<HashMap<GlyphKey, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        keys_in_order()
            .into_iter()
            .enumerate()
            .map(|(i, key)| {
                let cp = PUA_START + u32::try_from(i).expect("index fits u32");
                (key, char::from_u32(cp).expect("valid PUA codepoint"))
            })
            .collect()
    })
}

/// The codepoint assigned to `key` (its position in [`keys_in_order`] plus
/// [`PUA_START`]).
#[must_use]
pub fn codepoint(key: GlyphKey) -> char {
    codepoint_map()[&key]
}

/// Stable glyph name for `key`.
///
/// Matches dronecom's `build.rs` post-table names, and is reused for the
/// `codepoints.json` map so the two cannot drift.
#[must_use]
pub fn name(key: GlyphKey) -> String {
    match key {
        GlyphKey::Base { class, aff } => format!("{class:?}.{aff:?}"),
        GlyphKey::Bda(dec) => format!("bda.{dec:?}"),
        GlyphKey::Heading(i) => format!("heading.{}", f64::from(i) * 5.0),
        GlyphKey::Geometric { class, aff } => format!("geo.{class:?}.{aff:?}"),
        GlyphKey::Group { class, aff } => format!("group.{class:?}.{aff:?}"),
        GlyphKey::GroupGeometric { class, aff } => format!("geo.group.{class:?}.{aff:?}"),
        GlyphKey::Unknown { aff } => format!("Unknown.{aff:?}"),
        GlyphKey::UnknownGeometric { aff } => format!("geo.Unknown.{aff:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_170_glyphs() {
        assert_eq!(keys_in_order().len(), 170);
    }

    #[test]
    fn codepoints_contiguous_e000_to_e0a9() {
        let keys = keys_in_order();
        for (i, &key) in keys.iter().enumerate() {
            assert_eq!(codepoint(key) as u32, 0xE000 + i as u32);
        }
        assert_eq!(codepoint(keys[169]) as u32, 0xE0A9);
    }

    #[test]
    fn legacy_codepoint_anchors() {
        use NtdsShapeClass::{Air, Torpedo};
        use ShapeAffiliation::{Friend, Neutral};
        // First and last base glyphs (E000, E01F).
        assert_eq!(codepoint(GlyphKey::Base { class: Air, aff: Friend }) as u32, 0xE000);
        assert_eq!(codepoint(GlyphKey::Base { class: Torpedo, aff: Neutral }) as u32, 0xE01F);
        // Category range starts, cross-checked against dc_ntds_icons/src/lib.rs.
        assert_eq!(codepoint(GlyphKey::Bda(BdaDecoration::Uncertain)) as u32, 0xE020);
        assert_eq!(codepoint(GlyphKey::Heading(0)) as u32, 0xE022);
        assert_eq!(codepoint(GlyphKey::Geometric { class: Air, aff: Friend }) as u32, 0xE06A);
        assert_eq!(
            codepoint(GlyphKey::Group { class: GroupNtdsShapeClass::Air, aff: Friend }) as u32,
            0xE08A
        );
        assert_eq!(codepoint(GlyphKey::Unknown { aff: Friend }) as u32, 0xE0A2);
        assert_eq!(codepoint(GlyphKey::UnknownGeometric { aff: Friend }) as u32, 0xE0A6);
    }

    #[test]
    fn affiliation_some_for_symbols_none_for_overlays() {
        use ShapeAffiliation::Friend;
        assert_eq!(
            GlyphKey::Base { class: NtdsShapeClass::Air, aff: Friend }.affiliation(),
            Some(Friend)
        );
        assert_eq!(GlyphKey::Unknown { aff: Friend }.affiliation(), Some(Friend));
        assert_eq!(GlyphKey::Bda(BdaDecoration::Uncertain).affiliation(), None);
        assert_eq!(GlyphKey::Heading(3).affiliation(), None);
    }

    #[test]
    fn overlays_are_zero_advance() {
        assert_eq!(GlyphKey::Bda(BdaDecoration::Uncertain).advance(), Advance::Zero);
        assert_eq!(GlyphKey::Heading(10).advance(), Advance::Zero);
        assert_eq!(
            GlyphKey::Base { class: NtdsShapeClass::Air, aff: ShapeAffiliation::Friend }.advance(),
            Advance::Full
        );
    }

    #[test]
    fn names_match_legacy_format() {
        assert_eq!(
            name(GlyphKey::Base { class: NtdsShapeClass::Air, aff: ShapeAffiliation::Friend }),
            "Air.Friend"
        );
        assert_eq!(name(GlyphKey::Heading(27)), "heading.135");
        assert_eq!(name(GlyphKey::Bda(BdaDecoration::ProbablyDestroyed)), "bda.ProbablyDestroyed");
        assert_eq!(
            name(GlyphKey::GroupGeometric {
                class: GroupNtdsShapeClass::Surface,
                aff: ShapeAffiliation::Enemy
            }),
            "geo.group.Surface.Enemy"
        );
    }
}
