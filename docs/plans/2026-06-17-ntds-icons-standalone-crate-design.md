# `ntds-icons` — standalone NTDS symbol font crate

**Date:** 2026-06-17
**Upstream issue:** dronecom #1465 — "Extract NTDS symbol font into a standalone open-source crate/repo"
**This document covers:** the standalone crate only (subsystem 1 of 3). The
dronecom cutover (subsystem 2) and distribution infra — GitHub Pages gallery,
Action, Release, npm (subsystem 3) — are explicitly deferred.

---

## 1. Motivation

The NTDS (Naval Tactical Data System) tactical-symbol work currently lives in
two dronecom crates:

- `dc_ntds_shapes` — zero-dep, bevy-free pure geometry: `ShapeCmd` +
  `describe_*` + the `NtdsShapeClass` / `GroupNtdsShapeClass` /
  `ShapeAffiliation` / `BdaDecoration` / `NtdsNodeShape` enums, plus a `logo`
  module with `heading_glyph` and three dronecom-logo-specific heading
  constants.
- `dc_ntds_icons` — a `build.rs` that turns the geometry into a TrueType font
  (kurbo stroke-expansion → `write-fonts`), plus a `lib.rs` that couples the
  resulting glyph codepoints to `bevy_egui` and `dc_types`.

**The deliverable with an audience is the font itself**: there are good
TrueType fonts for the modern NATO/joint symbology (MIL-STD-2525, APP-6A —
filled, land-centric, e.g. MapSymbs), but **no good standalone NTDS font**.
NTDS is the older naval set: *outline* shapes (not filled), a *square* for
unknown (vs 2525's club/quatrefoil), with the same warfare-area encoding
(closed = surface, top-open = air, bottom-open = subsurface). The Rust crate
is incidental — it's how we generate the font and how the game reuses the
geometry to draw symbols directly (map gizmos, SVG export, Steam icons).

This crate extracts that work into a clean, bevy-free, font-first package.

## 2. Goals for this session (subsystem 1)

A standalone crate, built and tested in its own repo at
`/Users/pj/workspace/gamedev/ntds-icons` (fresh `git init`, independent of
dronecom). The crate:

1. Builds **bevy-free with zero heavy deps by default** — `std` only.
2. Carries the pure geometry over **verbatim**.
3. Lifts font generation out of `build.rs` into a public
   `build_font(&FontLayout) -> Vec<u8>` behind a `build` feature.
4. Ships a pre-built **`FONT_TTF`** via `include_bytes!`, with a roundtrip
   drift test asserting the committed artifact byte-matches a fresh
   `build_font()`.
5. Exposes codepoint lookups in a `codepoints` module on the crate's **own**
   enums, returning `char`; `heading_glyph` lives here (no longer in `logo`).
6. Has a single source of truth for the PUA codepoint assignment shared by the
   font builder and the codepoint lookups (the "no drift" requirement).
7. Generates, from that shared data via the `ntds-font-gen` CLI:
   `ntds_icons.ttf`, `ntds_icons.woff2`, a specimen PNG, a `codepoints.json`
   map, and the README codepoint table.
8. Leads with a **font-first README**: install, codepoint cheat-sheet, specimen
   image, the 2525/APP-6 prior-art framing, and a link to the dronecom manual's
   symbology chapter as a live example.
9. Is dual-licensed MIT/Apache-2.0 (crate) with the font under SIL OFL 1.1.

## 3. Non-goals (deferred — named, not silently dropped)

- **The dronecom cutover.** No deletion of `dc_ntds_shapes` /
  `dc_ntds_icons`, no egui shim, no rewiring of consumers. dronecom is
  untouched this session. (Held until the final repo/crate/font name is
  decided.)
- **The egui shim** (`add_fonts`, `RichText`/`LayoutJob` helpers,
  `ContactPresentation`, the `dc_types` ↔ crate-enum mapping including the
  `Own|Allied → Friend` policy). Belongs in dronecom post-cutover.
- **Distribution infra:** GitHub Pages gallery (`index.html`), GitHub Action,
  GitHub Releases, npm package, OFL directory submissions. (The CLI *does*
  emit `.woff2` and the specimen PNG now — those are font artifacts, generated
  from the shared data — but the gallery/Action/Release that consume them are
  deferred.)
- **crates.io / npm publish** — the user's call, non-blocking.
- **The `logo` trio constants** (`FRIENDLY_HEADING_DEG`, `UNKNOWN_HEADING_DEG`,
  `ENEMY_HEADING_DEG`) — dronecom-logo-specific (main menu / scene overlay /
  Steam icon / website hero). They stay in dronecom and get re-homed during the
  cutover. Only the generic `heading_glyph` comes over (into `codepoints`).

## 4. Architecture

### 4.1 Crate layout

A single crate (not a workspace); heavy deps gated behind a `build` feature.

```
ntds-icons/
  Cargo.toml
  README.md
  LICENSE-MIT
  LICENSE-APACHE
  OFL.txt                 # font license (SIL OFL 1.1)
  assets/
    ntds_icons.ttf        # committed, versioned — the headline artifact
    ntds_icons.woff2      # committed (web)
    specimen.png          # committed (preview / README image)
    codepoints.json       # committed (codepoint → glyph-name/class/affiliation)
  src/
    lib.rs                # re-exports; pub const FONT_TTF = include_bytes!("../assets/ntds_icons.ttf")
    shapes.rs             # geometry, moved verbatim from dc_ntds_shapes
    layout.rs             # SINGLE SOURCE OF TRUTH for the glyph set + codepoint order
    codepoints.rs         # char-returning lookups derived from `layout`
    build/                # #[cfg(feature = "build")]
      mod.rs              # build_font(&FontLayout) -> Vec<u8>; FontLayout
      font.rs             # ShapeCmd → kurbo → write-fonts (lift of build.rs)
      woff2.rs            # to_woff2(&[u8]) -> Vec<u8>
      specimen.rs         # render_specimen(&[u8]) -> Vec<u8> (PNG)
      artifacts.rs        # codepoints.json + README-table emitters
    bin/
      ntds-font-gen.rs    # #[cfg(feature = "build")] CLI driving the above
  docs/
    plans/2026-06-17-ntds-icons-standalone-crate-design.md   # this file
```

### 4.2 `shapes` — pure geometry (verbatim)

The entire contents of `dc_ntds_shapes/src/lib.rs` move into `shapes.rs`
unchanged: `ShapeCmd`; the enums (`NtdsShapeClass`, `GroupNtdsShapeClass`,
`ShapeAffiliation`, `BdaDecoration`, `NtdsNodeShape`); `describe_symbol`,
`describe_group_symbol`, `describe_heading_vector`, `describe_bda_decoration`,
`describe_node_symbol`; `base_shape_center_y`, `perceived_center_y`; and the
constants (`HEADING_VECTOR_LENGTH_RATIO`, `GROUP_OUTER_RADIUS_RATIO`,
`CROSS_INSET`, perceived-center factors). Zero dependencies. The geometry stays
public so the eventual cutover can move dronecom's gizmo / SVG / Steam /
website consumers onto it.

The `dc_ntds_shapes/src/tests.rs` suite ports verbatim.

### 4.3 `layout` — the single source of truth

The crate's defining design decision. Today the PUA assignment is stated
twice: `build.rs` derives codepoints by index arithmetic to build the cmap,
and `dc_ntds_icons` hard-codes the identical codepoints in 170+ match arms.
That duplication is the drift risk #1465 keeps naming.

`layout` replaces both with **one ordered list** of glyph descriptors. Each
descriptor names:

- its **semantic key** (e.g. `Base { class, aff }`, `Geometric { class, aff }`,
  `Bda(decoration)`, `Heading(index)`, `Group { class, aff }`,
  `GroupGeometric { class, aff }`, `Unknown { aff }`,
  `UnknownGeometric { aff }`),
- how to **produce its geometry** (which `describe_*` call + center mode),
- its **centering mode** (construction vs perceived/geometric),
- its **advance mode** (full advance for base symbols; zero advance for the
  BDA / heading overlay glyphs so they compose like combining accents).

Codepoints are **auto-assigned sequentially from `U+E000`** in list order.
Today's eight categories pack densely with no gaps — `E000..=E0A9`, 170 glyphs
— so sequential assignment **reproduces the existing codepoints exactly**:

| Range        | Count | Category                              |
|--------------|-------|---------------------------------------|
| E000–E01F    | 32    | base, construction-centered (8×4)     |
| E020–E021    | 2     | BDA decorations                       |
| E022–E069    | 72    | heading vectors (5° steps)            |
| E06A–E089    | 32    | base, geometric-centered (8×4)        |
| E08A–E095    | 12    | group, construction-centered (3×4)    |
| E096–E0A1    | 12    | group, geometric-centered (3×4)       |
| E0A2–E0A5    | 4     | unknown-class, construction (×4 aff)  |
| E0A6–E0A9    | 4     | unknown-class, geometric (×4 aff)     |

Total 170 glyphs + `.notdef` (glyph 0) = 171, matching the current font.

Both `build_font` (glyf/loca order, cmap, hmtx advance classification) and
`codepoints` derive from this list. The order is the contract; the codepoints
fall out of it.

### 4.4 `codepoints` — char-returning lookups

Public functions, all returning **`char`** (per the issue's "exposes `char` +
`&[u8]` + its own enums only"):

```rust
pub fn glyph(class: NtdsShapeClass, aff: ShapeAffiliation) -> char;
pub fn geometric_glyph(class: NtdsShapeClass, aff: ShapeAffiliation) -> char;
pub fn group_glyph(class: GroupNtdsShapeClass, aff: ShapeAffiliation) -> char;
pub fn group_geometric_glyph(class: GroupNtdsShapeClass, aff: ShapeAffiliation) -> char;
pub fn bda_glyph(decoration: BdaDecoration) -> char;
pub fn heading_glyph(heading_degrees: f32) -> char;   // moved out of `logo`
pub fn unknown_glyph(aff: ShapeAffiliation) -> char;            // flat unknown-class
pub fn unknown_geometric_glyph(aff: ShapeAffiliation) -> char;
```

Each looks its semantic key up against `layout`'s ordering (a `OnceLock`
key→codepoint map built once from the list, or the equivalent positional
formula). Returning `char` rather than `&'static str` means losing `const fn`;
that's fine — the future dronecom shim wraps these for egui, and per-frame cost
is a branch-free map hit.

`heading_glyph(deg)` keeps the existing contract: nautical degrees (CW from
north), nearest-5° rounding, wraps negatives and ≥360°; returns the
zero-advance overlay glyph at `E022 + round(deg/5)`.

Also exposed: `all_glyphs() -> impl Iterator<Item = GlyphEntry>` where
`GlyphEntry` carries `{ codepoint: char, name: &'static str, category, class,
affiliation }`. This is the data source the README table, the specimen, and
the future gallery all read — so they cannot drift from the font.

### 4.5 `build` feature — `build_font` and friends

A faithful lift of the current `build.rs` (a port, not a rewrite — every branch
in the legacy `main()` / `generate_all_glyphs()` has a counterpart):

```rust
pub struct FontLayout {
    pub units_per_em: u16,        // 1024
    pub symbol_radius: f64,       // 400.0
    pub stroke_width: f64,        // 60.0
    pub advance_width: u16,       // 1024
    pub center: f64,              // 512.0
    pub glyph_center_y: f64,      // 390.0
    pub flatten_tolerance: f64,   // 0.5
    pub cap: Cap,                 // Butt
    pub join: Join,               // Miter
    pub names: FontNames,         // family "NTDS Icons", etc.
}
impl Default for FontLayout { /* the current constants */ }

pub fn build_font(layout: &FontLayout) -> Vec<u8>;   // glyf/loca/cmap/hmtx/head/hhea/maxp/OS2/name/post
pub fn to_woff2(ttf: &[u8]) -> Vec<u8>;               // ttf2woff2::encode, fixed brotli quality
pub fn render_specimen(ttf: &[u8]) -> Vec<u8>;        // PNG bytes
```

`build_font` iterates `layout::glyphs_in_order()`, building each `SimpleGlyph`
exactly as `build.rs` does (translate `ShapeCmd` → kurbo paths, stroke-expand +
flatten, apply the construction/perceived dy shift and the overlay dx shift),
then assembles the ten tables. `FontLayout::default()` reproduces the committed
font byte-for-byte.

`render_specimen` loads the generated TTF into a `fontdb`, lays the PUA glyphs
out as `<text>` runs in an SVG colored by affiliation (friend/unknown/enemy/
neutral from a small fixed palette), and rasterizes via `resvg` →
`tiny_skia::Pixmap::encode_png`. Rendering the **real font** (not the source
geometry) makes the specimen genuine proof the font works. If resvg text
layout of PUA codepoints proves fiddly, the fallback is rendering the
`ShapeCmd` geometry directly to SVG (the `dc_site/src/svg.rs` pattern) — noted,
not preferred.

### 4.6 `ntds-font-gen` CLI

`#[cfg(feature = "build")]` binary. `ntds-font-gen [--out-dir DIR]` writes all
artifacts from the shared data in one pass: `ntds_icons.ttf`,
`ntds_icons.woff2`, `specimen.png`, `codepoints.json`, and the README codepoint
table (to stdout or a snippet file). Re-running it regenerates the committed
`assets/` artifacts — the single command that keeps everything in sync.

## 5. Dependencies

| Crate         | Feature   | Purpose                                   |
|---------------|-----------|-------------------------------------------|
| (none)        | default   | `std` only — zero heavy deps              |
| `kurbo`       | `build`   | stroke expansion + flattening             |
| `write-fonts` | `build`   | TrueType table assembly                   |
| `ttf2woff2`   | `build`   | pure-Rust TTF → WOFF2 (Brotli)            |
| `resvg`       | `build`   | specimen rendering (pulls usvg/tiny-skia/fontdb) |
| `clap`        | `build`   | CLI arg parsing                           |

Exact versions pinned during implementation; `resvg 0.45` matches dronecom's
`dc_steam_assets`.

## 6. Tests

- **Geometry** — `dc_ntds_shapes/tests.rs` ported verbatim.
- **Codepoints** — uniqueness per (class, distinct affiliation); base vs
  geometric distinct; group distinct from individual; heading distinct from
  symbol/BDA; the `heading_glyph` wrap/round cases. Re-keyed onto the crate's
  own enums.
- **Layout↔font consistency** — every `codepoints::*()` char appears in a
  freshly built font's cmap (chains lookups ↔ `layout` ↔ `build_font`).
- **Drift guard** — `build_font(&FontLayout::default())` byte-equals the
  committed `FONT_TTF` (`cargo test --features build`).
- **Structural equivalence to dronecom's current font** — the generated font's
  cmap → glyph-contours match the current `assets/fonts/ntds_icons.ttf`, so the
  deferred cutover is visually a no-op. (Structural, not necessarily a byte
  match against the legacy file — `write-fonts` version differences could shift
  bytes without changing rendering.)
- **Artifact smoke** — `to_woff2` / `render_specimen` produce non-empty output
  with the expected magic bytes (`wOF2` / PNG signature). Not a hard byte-match
  (brotli/resvg byte-stability across versions isn't worth the brittleness).

The egui-coupled tests (`add_fonts_registers_family`, the `RichText`/
`ContactPresentation` ones) do **not** port — no egui in this crate.

## 7. Acceptance criteria (this session)

- [ ] Fresh `ntds-icons` repo; default `cargo build` pulls no
      bevy/egui/`dc_types`/kurbo (std-only graph).
- [ ] `shapes` geometry + tests ported verbatim; zero-dep.
- [ ] `layout` is the single source of truth; `codepoints` and `build_font`
      both derive from it; consistency test passes.
- [ ] `codepoints` lookups return `char` on the crate's own enums;
      `heading_glyph` lives in `codepoints`, not `logo`.
- [ ] `build_font(&FontLayout) -> Vec<u8>` behind the `build` feature; faithful
      lift of `build.rs` (parity map in the plan).
- [ ] Committed `FONT_TTF` via `include_bytes!`; roundtrip byte-match drift
      test green under `--features build`.
- [ ] Generated font structurally equivalent to dronecom's current font.
- [ ] `ntds-font-gen` emits `.ttf` + `.woff2` + `specimen.png` +
      `codepoints.json` + README table from the shared data.
- [ ] Font-first README: install, codepoint table, specimen image, 2525/APP-6
      prior-art framing, link to the dronecom manual symbology chapter.
- [ ] LICENSE-MIT + LICENSE-APACHE (crate) and OFL.txt (font) present; README
      states which applies to what.
- [ ] `cargo test` (default) and `cargo test --features build` both green.

## 8. dronecom bookkeeping

dronecom #1465 → In Progress; a comment links this plan and records that
subsystem 1 is being built in the standalone repo with the cutover held. No
dronecom worktree or PR this session.
