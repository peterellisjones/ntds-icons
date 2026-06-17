# Distribution (gallery + deploy) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate a dark-tactical GitHub Pages gallery from `all_glyphs()` and wire GitHub Actions to deploy it (on push to main) and publish `.ttf`/`.woff2` releases (on `v*` tag).

**Architecture:** A new `build::gallery` module `include_str!`s a static `template.html` (dark-tactical CSS + vanilla JS) and injects a JSON glyph array + the woff2 href. `ntds-font-gen` writes `index.html` alongside its other artifacts. Two workflow files handle Pages deploy and tagged releases.

**Tech Stack:** Rust 2024 (`build` feature). Vanilla HTML/CSS/JS (no framework, no CDN). GitHub Actions (`actions/deploy-pages`, `softprops/action-gh-release`).

## Global Constraints

- **No drift:** the gallery is generated from `codepoints::all_glyphs()` — never hand-maintained.
- **Self-contained page:** inline CSS + JS; the only external asset is the committed `ntds_icons.woff2` (system sans for prose). No CDN, no framework.
- **Dark-tactical aesthetic:** bg `#0d1117`; affiliation palette friend `#4aa3ff`, enemy `#ff5a5a`, unknown `#ffd23f`, neutral `#4ade80`; neutral accent `#8b949e` for overlays; mono for codepoints/labels.
- **`build`-feature only:** gallery generation lives behind the `build` feature; the default crate graph stays std-only.
- **Deploy triggers:** gallery on push to `main` (+ `workflow_dispatch`); release on `v*` tag.
- Repo: `peterellisjones/ntds-icons`, branch `distribution`.

---

## File structure

| File | Responsibility |
|---|---|
| `src/layout.rs` | add `GlyphKey::affiliation()` |
| `src/build/gallery/mod.rs` | `render_gallery(woff2_href) -> String` + the JSON model |
| `src/build/gallery/template.html` | static dark-tactical page with `__WOFF2_HREF__` + `__GLYPH_DATA__` |
| `src/build/mod.rs` | `pub use gallery::render_gallery;` |
| `src/bin/ntds-font-gen.rs` | also write `index.html` |
| `assets/index.html` | committed generated gallery |
| `.github/workflows/pages.yml` | deploy gallery to Pages on push-to-main |
| `.github/workflows/release.yml` | drift-check + release `.ttf`/`.woff2` on tag |
| `README.md` | link the live gallery + document the one-time Pages setting |

---

## Task 1: `GlyphKey::affiliation()`

**Files:** Modify `src/layout.rs`.

**Interfaces:** Produces `GlyphKey::affiliation(self) -> Option<ShapeAffiliation>`.

- [ ] **Step 1: implement** (add to the existing `impl GlyphKey`):

```rust
/// The affiliation this glyph belongs to, or `None` for the affiliation-
/// agnostic overlays (BDA, heading).
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
```

- [ ] **Step 2: test** (in `layout::tests`):

```rust
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
```

- [ ] **Step 3: verify + commit**

Run: `cargo test layout::` → PASS.
```bash
git add -A && git commit -m "feat: GlyphKey::affiliation() helper"
```

---

## Task 2: `build::gallery` — model + template + render

**Files:** Create `src/build/gallery/mod.rs`, `src/build/gallery/template.html`; Modify `src/build/mod.rs`.

**Interfaces:**
- Consumes: `codepoints::all_glyphs`, `layout::GlyphKey`, `shapes` enums.
- Produces: `render_gallery(woff2_href: &str) -> String`.

- [ ] **Step 1: the JSON model + render fn** — `src/build/gallery/mod.rs`. For each glyph, emit a JS object with the fields the page needs, derived by matching the `GlyphKey`. Hand-rolled JSON (no serde):

```rust
//! Generate the dark-tactical GitHub Pages gallery from the shared glyph data.

use crate::codepoints::all_glyphs;
use crate::layout::GlyphKey;
use crate::shapes::BdaDecoration;

const TEMPLATE: &str = include_str!("template.html");

/// Render the self-contained gallery page. `woff2_href` is the `@font-face`
/// `src` (e.g. `"ntds_icons.woff2"`).
#[must_use]
pub fn render_gallery(woff2_href: &str) -> String {
    let data = glyph_data_json();
    TEMPLATE
        .replace("__WOFF2_HREF__", woff2_href)
        .replace("__GLYPH_DATA__", &data)
}

/// A JSON array of every glyph: `{cp, ch, name, cat, class, aff, deg, bda}`.
/// `ch` is the glyph as a JSON `\uXXXX` escape so it embeds safely in HTML.
fn glyph_data_json() -> String {
    let mut s = String::from("[");
    for (i, e) in all_glyphs().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let cp = e.codepoint as u32;
        let (cat, class, aff, deg, bda) = classify(e.key);
        s.push_str(&format!(
            "{{\"cp\":\"{cp:04X}\",\"ch\":\"\\u{cp:04x}\",\"name\":\"{}\",\
             \"cat\":\"{cat}\",\"class\":{class},\"aff\":{aff},\"deg\":{deg},\"bda\":{bda}}}",
            e.name
        ));
    }
    s.push(']');
    s
}

/// (category, class-or-null, aff-or-null, deg-or-null, bda-or-null) — each as a
/// JSON literal string ready to splice in.
fn classify(key: GlyphKey) -> (&'static str, String, String, String, String) {
    let q = |s: String| format!("\"{s}\"");
    let null = || "null".to_string();
    let aff = key.affiliation().map_or_else(null, |a| q(format!("{a:?}")));
    match key {
        GlyphKey::Base { class, .. } => ("symbol", q(format!("{class:?}")), aff, null(), null()),
        GlyphKey::Geometric { class, .. } => ("geometric", q(format!("{class:?}")), aff, null(), null()),
        GlyphKey::Group { class, .. } => ("group", q(format!("{class:?}")), aff, null(), null()),
        GlyphKey::GroupGeometric { class, .. } => ("group-geo", q(format!("{class:?}")), aff, null(), null()),
        GlyphKey::Unknown { .. } => ("unknown", null(), aff, null(), null()),
        GlyphKey::UnknownGeometric { .. } => ("unknown-geo", null(), aff, null(), null()),
        GlyphKey::Heading(i) => ("heading", null(), null(), (f64::from(i) * 5.0).to_string(), null()),
        GlyphKey::Bda(b) => {
            let name = match b { BdaDecoration::Uncertain => "Uncertain", BdaDecoration::ProbablyDestroyed => "ProbablyDestroyed" };
            ("bda", null(), null(), null(), q(name.to_string()))
        }
    }
}
```

- [ ] **Step 2: the template** — `src/build/gallery/template.html`. A complete dark-tactical page (see "Template contents" appendix below for the full file). It MUST contain: `<style>` (dark-tactical CSS), `@font-face { font-family: "NTDS Icons"; src: url("__WOFF2_HREF__") format("woff2"); }`, a header, a **composition demo** section (selects for class/affiliation/heading/BDA + a live `.preview` element + a copy button), a **grid** container, and a `<script>` that: parses `const GLYPHS = __GLYPH_DATA__;`, renders category sections + cards, wires search + affiliation toggles + click-to-copy, and wires the demo (composes `base.ch + heading.ch + bda.ch`, mutually exclusive heading/BDA).

- [ ] **Step 3: wire** — `src/build/mod.rs`: add `mod gallery;` and `pub use gallery::render_gallery;`.

- [ ] **Step 4: tests** (`src/build/gallery/mod.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_is_self_contained_and_complete() {
        let html = render_gallery("ntds_icons.woff2");
        // placeholders all substituted
        assert!(!html.contains("__WOFF2_HREF__"));
        assert!(!html.contains("__GLYPH_DATA__"));
        // font wired
        assert!(html.contains("@font-face"));
        assert!(html.contains("url(\"ntds_icons.woff2\")"));
        // all 170 glyphs in the data
        assert_eq!(html.matches("\"cp\":\"").count(), 170);
        assert!(html.contains("\"cp\":\"E000\""));
        assert!(html.contains("\"cp\":\"E0A9\""));
        // composition demo present
        assert!(html.contains("id=\"demo\""));
    }
}
```

- [ ] **Step 5: verify + commit**

Run: `cargo test --features build gallery` → PASS.
```bash
git add -A && git commit -m "feat: build::gallery — dark-tactical gallery from all_glyphs()"
```

---

## Task 3: CLI emits `index.html`

**Files:** Modify `src/bin/ntds-font-gen.rs`.

- [ ] **Step 1:** import `render_gallery` and write `index.html` after the existing artifacts:

```rust
use ntds_icons::build::{
    FontLayout, build_font, codepoints_json, render_gallery, render_specimen, to_woff2,
};
// ... in main(), after the woff2 write:
let html = render_gallery("ntds_icons.woff2");
// ... add to the writes:
write("index.html", html.as_bytes());
```

- [ ] **Step 2: regenerate + verify**

Run: `cargo run --features build --bin ntds-font-gen`
Then: `cargo test --features build` (drift test still green).
Expected: `assets/index.html` written; head references `ntds_icons.woff2`.

```bash
grep -q 'url("ntds_icons.woff2")' assets/index.html && echo OK
```

- [ ] **Step 3: commit** (includes the committed `assets/index.html`)
```bash
git add -A && git commit -m "feat: ntds-font-gen emits index.html (gallery)"
```

---

## Task 4: Eyeball the gallery

**Files:** none (manual verification).

- [ ] **Step 1:** open `assets/index.html` in a browser (or `python3 -m http.server` in `assets/` and visit it). Confirm: glyphs render via the woff2; affiliation colors correct; search filters; affiliation toggles work; click-to-copy flashes; the composition demo composites base+heading+BDA live. Fix any CSS/JS issues in `template.html`, regenerate (`cargo run --features build --bin ntds-font-gen`), re-check.

- [ ] **Step 2: commit** any template fixes + regenerated `index.html`.

---

## Task 5: `pages.yml` — deploy gallery on push-to-main

**Files:** Create `.github/workflows/pages.yml`.

- [ ] **Step 1: write the workflow**

```yaml
name: Deploy gallery to Pages
on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Generate gallery
        run: cargo run --features build --bin ntds-font-gen -- --out-dir dist
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 2: commit**
```bash
git add .github/workflows/pages.yml && git commit -m "ci: deploy gallery to GitHub Pages on push to main"
```

---

## Task 6: `release.yml` — drift-check + release on tag

**Files:** Create `.github/workflows/release.yml`.

- [ ] **Step 1: write the workflow**

```yaml
name: Release
on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Regenerate committed artifacts
        run: cargo run --features build --bin ntds-font-gen -- --out-dir assets
      - name: Drift check (committed artifacts must be fresh)
        run: git diff --exit-code -- assets/ntds_icons.ttf assets/ntds_icons.woff2
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            assets/ntds_icons.ttf
            assets/ntds_icons.woff2
          generate_release_notes: true
```

- [ ] **Step 2: commit**
```bash
git add .github/workflows/release.yml && git commit -m "ci: publish ttf/woff2 release on v* tags (with drift check)"
```

---

## Task 7: README — link the gallery + Pages setup note

**Files:** Modify `README.md`.

- [ ] **Step 1:** add a "Gallery" line near the top (under the specimen) linking the Pages URL `https://peterellisjones.github.io/ntds-icons/`, and a short "Maintainer setup" note: enable Pages once (Settings → Pages → Source: **GitHub Actions**); the gallery then redeploys on every push to `main`, and tagging `vX.Y.Z` cuts a Release with `.ttf`/`.woff2`.

- [ ] **Step 2: commit**
```bash
git add README.md && git commit -m "docs: link the live gallery + Pages maintainer setup"
```

---

## Appendix: template contents (Task 2 Step 2)

The full `template.html` is authored during implementation. Required structure
(dark-tactical, self-contained):

- `<head>`: `<meta charset>`, title "NTDS Icons", `<style>` with the palette
  constants, `@font-face` using `url("__WOFF2_HREF__")`, `.glyph { font-family:
  "NTDS Icons" }`.
- **Header**: title, one-line tagline, links (crates.io, GitHub, OFL).
- **Composition demo** `<section id="demo">`: `<select id="demo-class">`
  (8 classes), `<select id="demo-aff">` (4 affiliations), `<input
  type="range" id="demo-heading" min="0" max="355" step="5">` + a readout,
  `<select id="demo-bda">` (None / Uncertain / ProbablyDestroyed — disables the
  heading slider when non-None, and vice-versa), a large `<span class="preview
  glyph">`, and a "copy codepoints" button showing e.g. `U+E008 U+E034`.
- **Controls**: `<input id="search">`, affiliation toggle buttons
  (`data-aff="Friend|Enemy|Unknown|Neutral"`).
- **Grid** `<div id="grid">`: filled by JS, grouped into `<section>`s by `cat`
  with a heading each; each card is the glyph (colored by `aff`, overlays in the
  neutral accent), the `U+XXXX` codepoint, and the name; clicking copies the
  codepoint.
- `<script>`: `const GLYPHS = __GLYPH_DATA__;`, then `AFF_COLOR = {Friend:'#4aa3ff',
  Enemy:'#ff5a5a', Unknown:'#ffd23f', Neutral:'#4ade80'}`, render-grid,
  search/toggle filters, click-to-copy (`navigator.clipboard.writeText`), and
  the demo composition (look up base by `cat:'symbol'`+class+aff, heading by
  `cat:'heading'`+deg, bda by `cat:'bda'`+name; concatenate their `ch`).

## Self-review

**Spec coverage:** `affiliation()` (T1), gallery generation + no-drift + no-placeholder + features (T2), live glyphs/color/copy/search/filter/demo (T2 template + T4 verify), CLI emits index.html (T3), pages.yml (T5), release.yml + drift check (T6), README + Pages note (T7). ✔

**Placeholder scan:** the template's full CSS/JS styling is authored at impl time (T2/T4) — its *structure, dynamic data, and behaviors* are fully specified here; the remaining freedom is visual styling, which is creative, not a hand-wave. No TODO/TBD requirements. ✔

**Type consistency:** `render_gallery(&str) -> String`, `GlyphKey::affiliation() -> Option<ShapeAffiliation>`, `classify`, `glyph_data_json`, the JSON fields (`cp/ch/name/cat/class/aff/deg/bda`) are consistent across tasks and the template's JS. ✔
