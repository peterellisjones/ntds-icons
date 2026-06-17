# `ntds-icons` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the standalone, bevy-free, font-first `ntds-icons` crate — pure NTDS geometry + a `char`-returning codepoint API + a `build`-feature font generator producing `ntds_icons.ttf` / `.woff2` / specimen PNG, all from one source of truth.

**Architecture:** Single crate, heavy deps gated behind a `build` feature so the default graph is `std`-only. A `layout` module is the single source of truth: one ordered list of glyph keys, codepoints auto-assigned sequentially from `U+E000`. Both the font builder (`build_font`) and the `codepoints` lookups derive from it. The font generator is a faithful port of dronecom's `crates/dc_ntds_icons/build.rs`.

**Tech Stack:** Rust 2024. `build` feature: `kurbo` (stroke expansion), `write-fonts` (TTF assembly), `ttf2woff2` (WOFF2), `resvg` (specimen), `clap` (CLI). Read-back tests: `skrifa`/`read-fonts`.

## Global Constraints

- **Default build = zero heavy deps.** `cargo build` (no features) pulls `std` only — no bevy, egui, `dc_types`, kurbo, or write-fonts. All generation deps are `optional = true`, enabled only by the `build` feature.
- **Codepoints return `char`** on the crate's own enums — never `&'static str`, never `dc_types`/egui types.
- **Single source of truth.** Codepoint assignment is stated once, in `layout`. `codepoints` and `build_font` both derive from it. No second copy.
- **Faithful port.** `build_font` reproduces dronecom's current font geometry/cmap/metrics. Every branch in `crates/dc_ntds_icons/build.rs` has a counterpart; produce a parity map.
- **Sequential codepoints.** Glyphs occupy `U+E000..=U+E0A9` (170 glyphs) contiguously; glyph 0 is `.notdef`. Position N → codepoint `0xE000 + N`, glyph id `N + 1`.
- **Licenses:** crate dual MIT/Apache-2.0; font SIL OFL 1.1.
- **Legacy sources** (read-only references in dronecom; do not modify):
  - geometry: `crates/dc_ntds_shapes/src/lib.rs`, `.../logo.rs`, `.../tests.rs`
  - codepoint lookups: `crates/dc_ntds_icons/src/lib.rs`
  - font generator: `crates/dc_ntds_icons/build.rs`
  - golden font: `assets/fonts/ntds_icons.ttf`

---

## File structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | package + `build` feature + optional deps + `[[bin]]` |
| `src/lib.rs` | module decls; `pub const FONT_TTF` |
| `src/shapes.rs` | geometry, verbatim port of `dc_ntds_shapes` |
| `src/shapes/tests.rs` | geometry tests, verbatim port |
| `src/layout.rs` | single source of truth: `GlyphKey`, `keys_in_order`, `codepoint`, `name`, `Advance` |
| `src/codepoints.rs` | `char`-returning lookups + `all_glyphs` + tests |
| `src/build/mod.rs` | `FontLayout`, `build_font`, re-exports (cfg `build`) |
| `src/build/font.rs` | `ShapeCmd → kurbo → write-fonts` (port of `build.rs`) |
| `src/build/woff2.rs` | `to_woff2` |
| `src/build/specimen.rs` | `render_specimen` |
| `src/build/artifacts.rs` | `codepoints.json` + README table emitters |
| `src/bin/ntds-font-gen.rs` | CLI (cfg `build`) |
| `assets/ntds_icons.ttf`,`.woff2`,`specimen.png`,`codepoints.json` | committed artifacts |
| `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `OFL.txt` | docs + licenses |

---

## Task 1: Scaffold + `shapes` geometry (verbatim port)

**Files:**
- Create: `Cargo.toml`, `src/lib.rs`, `src/shapes.rs`, `src/shapes/tests.rs`, `LICENSE-MIT`, `LICENSE-APACHE`, `OFL.txt`

**Interfaces:**
- Produces: the `shapes` module — `ShapeCmd`; enums `NtdsShapeClass` (`ALL: [Self;8]`), `GroupNtdsShapeClass` (`ALL: [Self;3]`, `to_shape_class`), `ShapeAffiliation` (`ALL: [Self;4]`), `BdaDecoration`, `NtdsNodeShape`; fns `describe_symbol`, `describe_group_symbol`, `describe_heading_vector`, `describe_bda_decoration`, `describe_node_symbol`, `base_shape_center_y`, `perceived_center_y`; consts `HEADING_VECTOR_LENGTH_RATIO`, `GROUP_OUTER_RADIUS_RATIO`.

- [ ] **Step 1: `Cargo.toml`**

```toml
[package]
name = "ntds-icons"
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
description = "NTDS (Naval Tactical Data System) tactical-symbol outline icon font and geometry."
repository = "https://github.com/peterellisjones/ntds-icons"
readme = "README.md"
keywords = ["ntds", "font", "icons", "tactical", "symbology"]
categories = ["graphics", "rendering"]

[lib]
doctest = false

[features]
build = ["dep:kurbo", "dep:write-fonts", "dep:ttf2woff2", "dep:resvg", "dep:clap"]

[dependencies]
kurbo = { version = "0.11", optional = true }
write-fonts = { version = "0.41", optional = true }
ttf2woff2 = { version = "0.10", optional = true }
resvg = { version = "0.45", optional = true }
clap = { version = "4", features = ["derive"], optional = true }

[dev-dependencies]
skrifa = "0.41"

[[bin]]
name = "ntds-font-gen"
path = "src/bin/ntds-font-gen.rs"
required-features = ["build"]
```

(Exact versions confirmed at impl time against crates.io; floors above. `write-fonts`/`skrifa` versions must be compatible — same fontations release train.)

- [ ] **Step 2: license files** — `LICENSE-MIT` (standard MIT, holder "Peter Ellis Jones"), `LICENSE-APACHE` (standard Apache-2.0 text), `OFL.txt` (SIL OFL 1.1 verbatim, reserved font name "NTDS Icons").

- [ ] **Step 3: port geometry** — copy `crates/dc_ntds_shapes/src/lib.rs` → `src/shapes.rs` **verbatim**, with exactly these edits:
  - Delete the `pub mod logo;` line (the `logo` module does not come over).
  - Change the doc comment's crate-internal path references (`dc_ntds_icons/build.rs`, `dc_ui/contacts/symbols.rs`) to neutral wording.
  - Keep `#[cfg(test)] mod tests;` → it resolves to `src/shapes/tests.rs`.

- [ ] **Step 4: port geometry tests** — copy `crates/dc_ntds_shapes/src/tests.rs` → `src/shapes/tests.rs` verbatim (it already does `use super::*`).

- [ ] **Step 5: `src/lib.rs` skeleton**

```rust
//! NTDS tactical-symbol geometry and outline icon font.
pub mod codepoints;
pub mod layout;
pub mod shapes;

#[cfg(feature = "build")]
pub mod build;

/// The pre-built NTDS icon font (TrueType). Generated by `build_font`
/// under the `build` feature and committed; byte-identical to a fresh build.
pub const FONT_TTF: &[u8] = include_bytes!("../assets/ntds_icons.ttf");
```

(`codepoints`/`layout` and the `assets/ntds_icons.ttf` include arrive in later tasks; to compile Task 1 alone, temporarily comment the `codepoints`/`layout`/`FONT_TTF` lines, or land Tasks 1–5 before the first `cargo build` of `lib.rs`. Simplest: in Task 1, `src/lib.rs` declares only `pub mod shapes;`; later tasks add the rest.)

For Task 1, `src/lib.rs` is just:
```rust
//! NTDS tactical-symbol geometry and outline icon font.
pub mod shapes;
```

- [ ] **Step 6: verify**

Run: `cargo test`
Expected: PASS — all ported geometry tests green.

Run: `cargo tree -e normal` (no features)
Expected: only `ntds-icons` — no dependencies listed.

- [ ] **Step 7: commit**

```bash
git add -A && git commit -m "feat: scaffold crate + port NTDS geometry (shapes)"
```

---

## Task 2: `layout` — single source of truth

**Files:**
- Create: `src/layout.rs`
- Modify: `src/lib.rs` (add `pub mod layout;`)

**Interfaces:**
- Consumes: `shapes::{NtdsShapeClass, GroupNtdsShapeClass, ShapeAffiliation, BdaDecoration}`.
- Produces: `GlyphKey` (enum, `Copy + Eq + Hash`), `Advance` (`Full`/`Zero`), `GlyphKey::advance(self) -> Advance`, `keys_in_order() -> Vec<GlyphKey>` (170 entries), `codepoint(GlyphKey) -> char`, `name(GlyphKey) -> String`, `PUA_START: u32`, `NUM_HEADING_GLYPHS: u8`.

- [ ] **Step 1: write the module**

```rust
//! Single source of truth for the NTDS font's glyph set and PUA codepoint
//! order. Glyphs occupy U+E000..=U+E0A9 contiguously, in `keys_in_order`
//! order; position N → codepoint 0xE000+N, glyph id N+1 (.notdef is id 0).
//! Both `build_font` (cmap/glyf/hmtx/post) and `codepoints` derive from here.

use std::{collections::HashMap, sync::OnceLock};

use crate::shapes::{BdaDecoration, GroupNtdsShapeClass, NtdsShapeClass, ShapeAffiliation};

/// First Private Use Area codepoint.
pub const PUA_START: u32 = 0xE000;
/// Heading-vector overlay glyph count (one per 5° step over 360°).
pub const NUM_HEADING_GLYPHS: u8 = 72;

/// Semantic identity of one glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphKey {
    Base { class: NtdsShapeClass, aff: ShapeAffiliation },
    Bda(BdaDecoration),
    Heading(u8),
    Geometric { class: NtdsShapeClass, aff: ShapeAffiliation },
    Group { class: GroupNtdsShapeClass, aff: ShapeAffiliation },
    GroupGeometric { class: GroupNtdsShapeClass, aff: ShapeAffiliation },
    Unknown { aff: ShapeAffiliation },
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
}

/// Every glyph in cmap / glyph-id order. Category order and the within-category
/// (class outer, affiliation inner) order match dronecom's `build.rs` exactly,
/// so sequential codepoint assignment reproduces the legacy codepoints.
#[must_use]
pub fn keys_in_order() -> Vec<GlyphKey> {
    let mut k = Vec::with_capacity(170);
    for class in NtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::Base { class, aff });
        }
    }
    for dec in [BdaDecoration::Uncertain, BdaDecoration::ProbablyDestroyed] {
        k.push(GlyphKey::Bda(dec));
    }
    for i in 0..NUM_HEADING_GLYPHS {
        k.push(GlyphKey::Heading(i));
    }
    for class in NtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::Geometric { class, aff });
        }
    }
    for class in GroupNtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::Group { class, aff });
        }
    }
    for class in GroupNtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            k.push(GlyphKey::GroupGeometric { class, aff });
        }
    }
    for aff in ShapeAffiliation::ALL {
        k.push(GlyphKey::Unknown { aff });
    }
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

/// The codepoint assigned to `key`.
#[must_use]
pub fn codepoint(key: GlyphKey) -> char {
    codepoint_map()[&key]
}

/// Stable glyph name (matches dronecom's `build.rs` post-table names; used by
/// the post table and `codepoints.json`).
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
        // First base glyph and last base glyph (E000, E01F).
        assert_eq!(codepoint(GlyphKey::Base { class: Air, aff: Friend }) as u32, 0xE000);
        assert_eq!(codepoint(GlyphKey::Base { class: Torpedo, aff: Neutral }) as u32, 0xE01F);
        // BDA, heading, geometric, group, unknown range starts.
        assert_eq!(codepoint(GlyphKey::Bda(BdaDecoration::Uncertain)) as u32, 0xE020);
        assert_eq!(codepoint(GlyphKey::Heading(0)) as u32, 0xE022);
        assert_eq!(codepoint(GlyphKey::Geometric { class: Air, aff: Friend }) as u32, 0xE06A);
        assert_eq!(codepoint(GlyphKey::Unknown { aff: Friend }) as u32, 0xE0A2);
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
}
```

- [ ] **Step 2: wire** — add `pub mod layout;` to `src/lib.rs`.

- [ ] **Step 3: verify**

Run: `cargo test layout::`
Expected: PASS (4 tests). The `legacy_codepoint_anchors` values are cross-checked against `dc_ntds_icons/src/lib.rs` match arms.

- [ ] **Step 4: commit**

```bash
git add -A && git commit -m "feat: layout module — single source of truth for codepoint assignment"
```

---

## Task 3: `codepoints` — char lookups + `all_glyphs`

**Files:**
- Create: `src/codepoints.rs`
- Modify: `src/lib.rs` (add `pub mod codepoints;`)

**Interfaces:**
- Consumes: `layout::{GlyphKey, codepoint, name, NUM_HEADING_GLYPHS}`, `shapes` enums.
- Produces: `glyph`, `geometric_glyph`, `group_glyph`, `group_geometric_glyph`, `bda_glyph`, `unknown_glyph`, `unknown_geometric_glyph` (all `-> char`); `heading_glyph(f32) -> char`; `GlyphEntry { codepoint, name, key }`; `all_glyphs() -> impl Iterator<Item = GlyphEntry>`.

- [ ] **Step 1: write the module**

```rust
//! `char`-returning PUA codepoint lookups on the crate's own enums. Every
//! value derives from `layout`, so the lookups and the font cannot drift.

use crate::layout::{self, GlyphKey, NUM_HEADING_GLYPHS};
use crate::shapes::{BdaDecoration, GroupNtdsShapeClass, NtdsShapeClass, ShapeAffiliation};

/// Construction-centered base symbol (composes with heading/BDA overlays).
#[must_use]
pub fn glyph(class: NtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Base { class, aff })
}

/// Geometric (perceived-center) base symbol — for buttons/labels; no overlays.
#[must_use]
pub fn geometric_glyph(class: NtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Geometric { class, aff })
}

#[must_use]
pub fn group_glyph(class: GroupNtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Group { class, aff })
}

#[must_use]
pub fn group_geometric_glyph(class: GroupNtdsShapeClass, aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::GroupGeometric { class, aff })
}

#[must_use]
pub fn bda_glyph(decoration: BdaDecoration) -> char {
    layout::codepoint(GlyphKey::Bda(decoration))
}

/// Flat unknown-class (bare affiliation outline, no interior).
#[must_use]
pub fn unknown_glyph(aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::Unknown { aff })
}

#[must_use]
pub fn unknown_geometric_glyph(aff: ShapeAffiliation) -> char {
    layout::codepoint(GlyphKey::UnknownGeometric { aff })
}

/// Heading-vector overlay glyph for the nearest 5° increment. `heading_degrees`
/// is clockwise from north; wraps negatives and values ≥ 360°. Zero-advance —
/// concatenate after a base glyph and it renders on top, like a combining mark.
#[must_use]
pub fn heading_glyph(heading_degrees: f32) -> char {
    let normalized = heading_degrees.rem_euclid(360.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = ((normalized / 5.0).round() as usize % NUM_HEADING_GLYPHS as usize) as u8;
    layout::codepoint(GlyphKey::Heading(index))
}

/// One entry in the full glyph table (drives `codepoints.json`, the README
/// table, and the future gallery).
pub struct GlyphEntry {
    pub codepoint: char,
    pub name: String,
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
```

- [ ] **Step 2: tests** (port the uniqueness/collision intent from `dc_ntds_icons/src/lib.rs`, re-keyed on crate enums):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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
        for i in 0..72 {
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
```

- [ ] **Step 3: wire** — add `pub mod codepoints;` to `src/lib.rs`.

- [ ] **Step 4: verify**

Run: `cargo test codepoints::`
Expected: PASS (6 tests).

- [ ] **Step 5: commit**

```bash
git add -A && git commit -m "feat: codepoints module — char lookups derived from layout"
```

---

## Task 4: `build::font` — `build_font` + `FontLayout` (port of build.rs)

**Files:**
- Create: `src/build/mod.rs`, `src/build/font.rs`
- Modify: `src/lib.rs` (add `#[cfg(feature = "build")] pub mod build;`)

**Interfaces:**
- Consumes: `layout::{keys_in_order, name, GlyphKey, Advance}`, all `shapes` items, `kurbo`, `write-fonts`.
- Produces: `FontLayout` (+ `Default`), `FontNames`, `build_font(&FontLayout) -> Vec<u8>`.

**Port reference:** `crates/dc_ntds_icons/build.rs` in full. Parity map is a required deliverable (see Step 4).

- [ ] **Step 1: `src/build/mod.rs`** — `FontLayout`, `FontNames`, re-exports.

```rust
//! Font generation (TTF/WOFF2/specimen). Only compiled with the `build`
//! feature; pulls kurbo + write-fonts + ttf2woff2 + resvg.

mod artifacts;
mod font;
mod specimen;
mod woff2;

pub use artifacts::{codepoints_json, readme_table};
pub use font::{build_font, FontLayout, FontNames};
pub use specimen::render_specimen;
pub use woff2::to_woff2;
```

(`artifacts`, `specimen`, `woff2` arrive in Tasks 6–8; for Task 4 include only `mod font; pub use font::{...};` and add the rest as those tasks land.)

`FontLayout` (in `font.rs`), defaults copied verbatim from `build.rs` constants (`UNITS_PER_EM=1024`, `SYMBOL_RADIUS=400`, `STROKE_WIDTH=60`, `ADVANCE_WIDTH=1024`, `CENTER=512`, `GLYPH_CENTER_Y=390`, `FLATTEN_TOLERANCE=0.5`, `CAP=Butt`, `JOIN=Miter`):

```rust
use kurbo::{Cap, Join};

pub struct FontNames {
    pub family: String,          // "NTDS Icons"
    pub subfamily: String,       // "Regular"
    pub unique_id: String,       // "NTDSIcons-1.0"
    pub full_name: String,       // "NTDS Icons Regular"
    pub version: String,         // "Version 1.0"
    pub postscript: String,      // "NTDSIcons-Regular"
}

pub struct FontLayout {
    pub units_per_em: u16,
    pub symbol_radius: f64,
    pub stroke_width: f64,
    pub advance_width: u16,
    pub center: f64,
    pub glyph_center_y: f64,
    pub flatten_tolerance: f64,
    pub cap: Cap,
    pub join: Join,
    pub names: FontNames,
}

impl Default for FontLayout {
    fn default() -> Self {
        Self {
            units_per_em: 1024,
            symbol_radius: 400.0,
            stroke_width: 60.0,
            advance_width: 1024,
            center: 512.0,
            glyph_center_y: 390.0,
            flatten_tolerance: 0.5,
            cap: Cap::Butt,
            join: Join::Miter,
            names: FontNames {
                family: "NTDS Icons".into(),
                subfamily: "Regular".into(),
                unique_id: "NTDSIcons-1.0".into(),
                full_name: "NTDS Icons Regular".into(),
                version: "Version 1.0".into(),
                postscript: "NTDSIcons-Regular".into(),
            },
        }
    }
}
```

(Legacy `build.rs` set name id UNIQUE_ID to `"DroneCarrier-NTDSIcons-1.0"`. We drop the `DroneCarrier-` prefix — the standalone font is not dronecom-branded. **Named behavior change**, harmless: name-table strings differ from the legacy file, which is *why* equivalence with the legacy font is structural (cmap/outlines), not byte-for-byte. The committed `FONT_TTF` is self-consistent with `build_font`.)

- [ ] **Step 2: port the geometry pipeline** into `font.rs`, **verbatim** from `build.rs` (parameterized by `FontLayout` where it used `const`s):
  - `translate_cmds` (build.rs L94–202) — verbatim.
  - `stroke_and_flatten` (L209–222) — take `stroke_width`/`flatten_tolerance`/`cap`/`join` from `FontLayout`.
  - `flatten_into` (L226–230) — verbatim.
  - `build_glyph` (L237–252) — take `FontLayout` for stroke params.

- [ ] **Step 3: the unified glyph driver** — replace `build.rs`'s eight inline loops in `generate_all_glyphs` (L258–412) with one switch keyed off `layout::GlyphKey`, iterating `layout::keys_in_order()`. Each arm reproduces the exact `describe_*` call + `(dx, dy)` shift of its legacy loop:

```rust
use crate::layout::{self, GlyphKey};
use crate::shapes::{
    describe_bda_decoration, describe_group_symbol, describe_heading_vector, describe_node_symbol,
    describe_symbol, perceived_center_y, NtdsNodeShape, NtdsShapeClass, ShapeCmd,
};

/// Geometry + (dx, dy) affine shift for one glyph. Mirrors the per-category
/// transforms in legacy `generate_all_glyphs` (build.rs L258–412).
fn glyph_geometry(key: GlyphKey, l: &FontLayout) -> (Vec<ShapeCmd>, f64, f64) {
    let c = l.center as f32;
    let r = l.symbol_radius as f32;
    let construction_dy = l.glyph_center_y - l.center; // GLYPH_CENTER_Y - CENTER
    let overlay_dx = -f64::from(l.advance_width);       // overlay glyphs shift left
    match key {
        GlyphKey::Base { class, aff } => (describe_symbol(class, aff, c, c, r), 0.0, construction_dy),
        GlyphKey::Bda(dec) => (describe_bda_decoration(dec, c, c, r), overlay_dx, construction_dy),
        GlyphKey::Heading(i) => {
            let degrees = f64::from(i) * 5.0;
            #[allow(clippy::cast_possible_truncation)]
            let math_angle = (90.0 - degrees).to_radians() as f32; // nautical CW→math CCW
            (describe_heading_vector(c, c, r, math_angle), overlay_dx, construction_dy)
        }
        GlyphKey::Geometric { class, aff } => {
            let dy = l.glyph_center_y - f64::from(perceived_center_y(class, aff, c, r));
            (describe_symbol(class, aff, c, c, r), 0.0, dy)
        }
        GlyphKey::Group { class, aff } => {
            (describe_group_symbol(class, aff, c, c, r), 0.0, construction_dy)
        }
        GlyphKey::GroupGeometric { class, aff } => {
            let mapped = class.to_shape_class();
            let dy = l.glyph_center_y - f64::from(perceived_center_y(mapped, aff, c, r));
            (describe_group_symbol(class, aff, c, c, r), 0.0, dy)
        }
        GlyphKey::Unknown { aff } => {
            (describe_node_symbol(NtdsNodeShape::Unknown, aff, c, c, r), 0.0, construction_dy)
        }
        GlyphKey::UnknownGeometric { aff } => {
            let dy = l.glyph_center_y
                - f64::from(perceived_center_y(NtdsShapeClass::Surface, aff, c, r));
            (describe_node_symbol(NtdsNodeShape::Unknown, aff, c, c, r), 0.0, dy)
        }
    }
}
```

- [ ] **Step 4: produce the parity map** as a doc comment in `font.rs` — a table: every legacy `build.rs` loop / branch / table → its counterpart here. Required deliverable (port-work rule). Confirm the eight `glyph_geometry` arms reproduce L271–409 transforms (verify each `dx`/`dy` against the legacy loop), and that the table-assembly (next step) reproduces L432–692.

- [ ] **Step 5: port table assembly** — `build_font` body, ported from `build.rs main()` (L419–698) minus the file-writing tail (L694–706, which moves to the CLI). Iterate `keys_in_order()` building glyphs + names; build `glyf`/`loca`; **cmap** = `(char(0xE000+i), GlyphId::new(i+1))` for each position (replaces L459–535's index arithmetic); **hmtx** advance = `if key.advance()==Advance::Full { layout.advance_width } else { 0 }` (replaces L581–609's range checks); `head`/`hhea`/`maxp`/`os2`/`post`/`name` exactly as legacy (parameterized by `FontLayout`); return `builder.build()` (`Vec<u8>`).

```rust
#[must_use]
pub fn build_font(layout: &FontLayout) -> Vec<u8> {
    let keys = layout::keys_in_order();
    // ... build glyphs (glyph 0 = SimpleGlyph::default(), name ".notdef") ...
    // ... glyf/loca via GlyfLocaBuilder; track global bbox, max_points, max_contours ...
    // cmap: contiguous from PUA_START
    let mappings: Vec<(char, GlyphId)> = keys
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let cp = 0xE000 + u32::try_from(i).unwrap();
            (char::from_u32(cp).unwrap(), GlyphId::new(u32::try_from(i + 1).unwrap()))
        })
        .collect();
    // hmtx: advance per key.advance(); head/hhea/maxp/os2/name(from layout.names)/post(from layout::name)
    // ... assemble via FontBuilder, return builder.build()
}
```

- [ ] **Step 6: smoke test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_font_produces_valid_sfnt() {
        let ttf = build_font(&FontLayout::default());
        assert!(ttf.len() > 10_000, "font suspiciously small: {}", ttf.len());
        // sfnt version for TrueType outlines is 0x00010000.
        assert_eq!(&ttf[0..4], &[0x00, 0x01, 0x00, 0x00]);
    }
}
```

- [ ] **Step 7: wire + verify**

Modify `src/lib.rs`: add `#[cfg(feature = "build")] pub mod build;`.

Run: `cargo test --features build build::`
Expected: PASS.

Run: `cargo build` (no features)
Expected: builds; `cargo tree -e normal` still shows no deps (build module is cfg'd out).

- [ ] **Step 8: commit**

```bash
git add -A && git commit -m "feat: build_font — port of dc_ntds_icons/build.rs, driven by layout"
```

---

## Task 5: Commit `FONT_TTF` + drift test + equivalence check

**Files:**
- Create: `assets/ntds_icons.ttf`, `tests/drift.rs`
- Modify: `src/lib.rs` (uncomment `pub const FONT_TTF`)

**Interfaces:**
- Consumes: `build::{build_font, FontLayout}`, `FONT_TTF`.

- [ ] **Step 1: generate + commit the golden font** — write a throwaway: `std::fs::write("assets/ntds_icons.ttf", ntds_icons::build::build_font(&Default::default()))` via a temporary test or the (Task 9) CLI. Confirm size ≈ 27 KB.

- [ ] **Step 2: enable `FONT_TTF`** in `src/lib.rs`:
```rust
pub const FONT_TTF: &[u8] = include_bytes!("../assets/ntds_icons.ttf");
```

- [ ] **Step 3: drift test** — `tests/drift.rs`:
```rust
#![cfg(feature = "build")]
#[test]
fn committed_font_matches_fresh_build() {
    let fresh = ntds_icons::build::build_font(&ntds_icons::build::FontLayout::default());
    assert_eq!(
        fresh.as_slice(),
        ntds_icons::FONT_TTF,
        "assets/ntds_icons.ttf is stale — re-run `cargo run --features build --bin ntds-font-gen`"
    );
}
```

- [ ] **Step 4: layout↔font cmap consistency** — `tests/drift.rs`, parse the built font with `skrifa` and assert every `codepoints::*()` char resolves to a non-`.notdef` glyph:
```rust
#[test]
fn every_lookup_char_is_in_the_font() {
    use skrifa::{MetadataProvider, raw::FontRef};
    let font = FontRef::new(ntds_icons::FONT_TTF).unwrap();
    let cmap = font.charmap();
    for entry in ntds_icons::codepoints::all_glyphs() {
        let gid = cmap.map(entry.codepoint).unwrap_or_default();
        assert_ne!(gid.to_u32(), 0, "{} ({:#x}) missing from cmap", entry.name, entry.codepoint as u32);
    }
}
```

- [ ] **Step 5: one-time equivalence verification vs dronecom** (documented, not a committed fixture test) — copy `/Users/pj/workspace/gamedev/dronecom/assets/fonts/ntds_icons.ttf` to `/tmp/legacy.ttf`; with a throwaway `skrifa` script, assert both fonts (a) map the same set of PUA codepoints and (b) produce the same outline points per shared codepoint. Record the result in the commit message. Expected: identical glyph outlines (name-table strings differ by the dropped `DroneCarrier-` prefix — that's fine). Remove the throwaway; do **not** commit the legacy font.

- [ ] **Step 6: verify**

Run: `cargo test --features build`
Expected: PASS (drift + consistency).

- [ ] **Step 7: commit**

```bash
git add assets/ntds_icons.ttf tests/drift.rs src/lib.rs
git commit -m "feat: commit FONT_TTF + drift/consistency tests (outlines == dronecom font)"
```

---

## Task 6: `build::woff2` — `to_woff2`

**Files:**
- Create: `src/build/woff2.rs`; Modify: `src/build/mod.rs` (add `mod woff2; pub use woff2::to_woff2;`)

**Interfaces:** Produces `to_woff2(ttf: &[u8]) -> Vec<u8>`.

- [ ] **Step 1: implement**
```rust
//! TTF → WOFF2 via the pure-Rust `ttf2woff2` crate.
use ttf2woff2::{encode, BrotliQuality};

/// Compress a TrueType font to WOFF2. Fixed Brotli quality for determinism.
#[must_use]
pub fn to_woff2(ttf: &[u8]) -> Vec<u8> {
    encode(ttf, BrotliQuality::best()).expect("woff2 encode of a valid font")
}
```
(Confirm `ttf2woff2`'s exact API — `encode` signature + the quality type — at impl time; adjust the call to match.)

- [ ] **Step 2: test**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn produces_woff2_magic() {
        let woff2 = to_woff2(crate::FONT_TTF);
        assert!(woff2.len() > 1000);
        assert_eq!(&woff2[0..4], b"wOF2");
    }
}
```

- [ ] **Step 3: verify + commit**

Run: `cargo test --features build woff2`  → PASS
```bash
git add -A && git commit -m "feat: to_woff2 via ttf2woff2"
```

---

## Task 7: `build::specimen` — `render_specimen`

**Files:**
- Create: `src/build/specimen.rs`; Modify: `src/build/mod.rs`

**Interfaces:** Produces `render_specimen(ttf: &[u8]) -> Vec<u8>` (PNG bytes).

- [ ] **Step 1: implement** — load the TTF into a `usvg` fontdb, build an SVG laying out the base symbols in a grid (rows = class, columns = affiliation), each as a `<text>` element in family "NTDS Icons" colored by affiliation, with small caps labels; rasterize via `resvg` to a `tiny_skia::Pixmap`; `pixmap.encode_png()`. Affiliation palette is a small fixed set local to this module (friend/unknown/enemy/neutral — this is a specimen, not dronecom UI, so a local palette is fine):
```rust
//! Render a specimen PNG of the real font (proof it works), colored by affiliation.
use resvg::tiny_skia;
use resvg::usvg::{self, fontdb};
// builds an <svg> with <text>&#xE000; ... </text> runs; see SVG assembly below.

const FRIEND: &str = "#3b8ee0";
const UNKNOWN: &str = "#e0c84b";
const ENEMY:  &str = "#e04b4b";
const NEUTRAL: &str = "#4be08a";

#[must_use]
pub fn render_specimen(ttf: &[u8]) -> Vec<u8> {
    let mut db = fontdb::Database::new();
    db.load_font_data(ttf.to_vec());
    let svg = build_specimen_svg(); // grid of <text> PUA runs, affiliation-colored
    let opt = usvg::Options { fontdb: db.into(), ..Default::default() };
    let tree = usvg::Tree::from_str(&svg, &opt).expect("specimen svg parses");
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).expect("pixmap");
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.encode_png().expect("png encode")
}
```
(`build_specimen_svg` composes the grid from `codepoints::glyph(class, aff)` over `NtdsShapeClass::ALL × ShapeAffiliation::ALL`, emitting each `char` as an SVG numeric entity. Confirm `resvg 0.45`'s exact `Options`/`render`/fontdb API at impl time.)

- [ ] **Step 2: test**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn produces_png() {
        let png = render_specimen(crate::FONT_TTF);
        assert!(png.len() > 2000);
        assert_eq!(&png[0..8], &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']);
    }
}
```

- [ ] **Step 3: verify + commit**

Run: `cargo test --features build specimen`  → PASS
```bash
git add -A && git commit -m "feat: render_specimen — PNG of the real font, affiliation-colored"
```

---

## Task 8: `build::artifacts` — `codepoints.json` + README table

**Files:** Create `src/build/artifacts.rs`; Modify `src/build/mod.rs`.

**Interfaces:** Produces `codepoints_json() -> String`, `readme_table() -> String`.

- [ ] **Step 1: implement** (hand-rolled JSON — no serde dep needed; entries from `codepoints::all_glyphs()`):
```rust
//! Emit the codepoint map (JSON) and the README cheat-sheet table (Markdown)
//! from the shared `all_glyphs` data, so neither drifts from the font.
use crate::codepoints::all_glyphs;

#[must_use]
pub fn codepoints_json() -> String {
    let mut s = String::from("[\n");
    let entries: Vec<_> = all_glyphs().collect();
    for (i, e) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        s.push_str(&format!(
            "  {{ \"codepoint\": \"U+{:04X}\", \"name\": \"{}\" }}{comma}\n",
            e.codepoint as u32, e.name
        ));
    }
    s.push_str("]\n");
    s
}

#[must_use]
pub fn readme_table() -> String {
    let mut s = String::from("| Codepoint | Glyph name |\n|---|---|\n");
    for e in all_glyphs() {
        s.push_str(&format!("| `U+{:04X}` | `{}` |\n", e.codepoint as u32, e.name));
    }
    s
}
```

- [ ] **Step 2: test**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_has_170_entries_and_anchors() {
        let j = codepoints_json();
        assert_eq!(j.matches("\"codepoint\"").count(), 170);
        assert!(j.contains("\"U+E000\""));
        assert!(j.contains("\"U+E0A9\""));
    }
    #[test]
    fn table_has_header_and_rows() {
        let t = readme_table();
        assert!(t.starts_with("| Codepoint |"));
        assert_eq!(t.matches("| `U+").count(), 170);
    }
}
```

- [ ] **Step 3: verify + commit**

Run: `cargo test --features build artifacts`  → PASS
```bash
git add -A && git commit -m "feat: codepoints.json + README table emitters from shared data"
```

---

## Task 9: `ntds-font-gen` CLI + regenerate committed artifacts

**Files:** Create `src/bin/ntds-font-gen.rs`; Create `assets/ntds_icons.woff2`, `assets/specimen.png`, `assets/codepoints.json`.

- [ ] **Step 1: implement the CLI**
```rust
//! Generate every NTDS font artifact from the shared codepoint data.
use std::{fs, path::PathBuf};

use clap::Parser;
use ntds_icons::build::{build_font, codepoints_json, render_specimen, to_woff2, FontLayout};

#[derive(Parser)]
#[command(about = "Generate ntds_icons.ttf/.woff2 + specimen.png + codepoints.json")]
struct Args {
    /// Output directory for artifacts.
    #[arg(long, default_value = "assets")]
    out_dir: PathBuf,
}

fn main() {
    let args = Args::parse();
    fs::create_dir_all(&args.out_dir).expect("create out dir");
    let ttf = build_font(&FontLayout::default());
    let woff2 = to_woff2(&ttf);
    let png = render_specimen(&ttf);
    let json = codepoints_json();
    fs::write(args.out_dir.join("ntds_icons.ttf"), &ttf).unwrap();
    fs::write(args.out_dir.join("ntds_icons.woff2"), &woff2).unwrap();
    fs::write(args.out_dir.join("specimen.png"), &png).unwrap();
    fs::write(args.out_dir.join("codepoints.json"), json).unwrap();
    eprintln!(
        "wrote ttf ({} B), woff2 ({} B), specimen.png ({} B), codepoints.json",
        ttf.len(), woff2.len(), png.len()
    );
}
```

- [ ] **Step 2: regenerate + verify drift**

Run: `cargo run --features build --bin ntds-font-gen`
Then: `cargo test --features build`
Expected: all PASS — the regenerated `ntds_icons.ttf` still byte-matches `FONT_TTF` (idempotent), woff2/specimen/json present.

- [ ] **Step 3: commit**
```bash
git add -A && git commit -m "feat: ntds-font-gen CLI + committed woff2/specimen/codepoints artifacts"
```

---

## Task 10: Font-first README

**Files:** Create `README.md`.

- [ ] **Step 1: write the README** — lead with the font. Sections, in order:
  1. **One-liner + specimen image** (`![specimen](assets/specimen.png)`).
  2. **What this is / prior art** — the 2525/APP-6 framing: filled NATO/joint fonts exist (MapSymbs); NTDS is the older *naval*, *outline* set (square = unknown), no good standalone font until now. Link the dronecom manual symbology chapter (https://dronecomgame.com/manual/symbology.html) as a live example of the font in use.
  3. **Install** — download `assets/ntds_icons.ttf` / `.woff2` from the repo; `@font-face` snippet for web; note PUA codepoints.
  4. **Codepoint cheat-sheet** — paste the output of `readme_table()` (the 170-row table). Note GitHub can't render a custom PUA font inline, hence the static table + specimen image.
  5. **Rust usage** — `ntds-icons` crate: default `codepoints::glyph(class, aff) -> char` + `FONT_TTF`; `build` feature for regeneration (`cargo run --features build --bin ntds-font-gen`).
  6. **Licenses** — font under SIL OFL 1.1 (`OFL.txt`); crate under MIT/Apache-2.0. Independent.
  7. **Deferred** (brief) — gallery/Action/Release/npm + egui integration are planned follow-ups.

- [ ] **Step 2: verify** — `readme_table()` output pasted matches current `codepoints.json`; specimen image renders; all links resolve.

- [ ] **Step 3: commit**
```bash
git add README.md && git commit -m "docs: font-first README with codepoint cheat-sheet + specimen"
```

---

## Self-review (done before handoff)

**Spec coverage:** every spec §7 AC maps to a task — zero-deps default (T1/T4), shapes verbatim (T1), layout single-source (T2), char codepoints + heading relocation (T3), build_font behind feature (T4), FONT_TTF + drift (T5), structural equivalence (T5), CLI all-artifacts (T6–T9), README (T10), licenses (T1). ✔

**Placeholder scan:** the three "confirm exact API at impl time" notes (ttf2woff2/resvg/clap versions + signatures) are deliberate version-pinning deferrals against live crates, not vague requirements — each names the exact symbol to confirm. No TODO/TBD requirements. ✔

**Type consistency:** `GlyphKey`, `Advance`, `codepoint`, `name`, `keys_in_order`, `all_glyphs`, `GlyphEntry`, `build_font`, `FontLayout`, `to_woff2`, `render_specimen`, `codepoints_json`, `readme_table` are used with identical signatures across tasks. ✔

**Named behavior change:** UNIQUE_ID name-table string drops the `DroneCarrier-` prefix (T4 Step 1) → legacy equivalence is structural (outlines/cmap), not byte-for-byte; the committed `FONT_TTF` is self-consistent with `build_font`. ✔
