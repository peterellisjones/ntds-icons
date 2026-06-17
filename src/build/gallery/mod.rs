//! Generate the dark-tactical GitHub Pages gallery from the shared glyph data.
//!
//! The page itself lives in `template.html` (maintainable HTML/CSS/JS);
//! [`render_gallery`] injects the glyph data and the woff2 href so the gallery
//! can never drift from the font.

use crate::codepoints::all_glyphs;
use crate::layout::GlyphKey;
use crate::shapes::BdaDecoration;

const TEMPLATE: &str = include_str!("template.html");

/// Render the self-contained gallery page.
///
/// `woff2_href` is the `@font-face` `src` URL (e.g. `"ntds_icons.woff2"`).
#[must_use]
pub fn render_gallery(woff2_href: &str) -> String {
    TEMPLATE
        .replace("__WOFF2_HREF__", woff2_href)
        .replace("__GLYPH_DATA__", &glyph_data_json())
}

/// A JSON array of every glyph: `{cp, ch, name, cat, class, aff, deg, bda}`.
/// `ch` is emitted as a `\uXXXX` escape so it embeds safely in the HTML/JS.
fn glyph_data_json() -> String {
    let mut s = String::from("[");
    for (i, e) in all_glyphs().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let cp = e.codepoint as u32;
        let (cat, class, aff, deg, bda) = classify(e.key);
        s.push_str(&format!(
            "{{\"cp\":\"{cp:04X}\",\"ch\":\"\\u{cp:04x}\",\"name\":\"{}\",\"cat\":\"{cat}\",\
             \"class\":{class},\"aff\":{aff},\"deg\":{deg},\"bda\":{bda}}}",
            e.name
        ));
    }
    s.push(']');
    s
}

/// `(category, class, aff, deg, bda)` — each already a JSON literal (quoted
/// string or `null`) ready to splice into the object.
fn classify(key: GlyphKey) -> (&'static str, String, String, String, String) {
    let q = |s: String| format!("\"{s}\"");
    let null = "null".to_string();
    let aff = key.affiliation().map_or_else(|| null.clone(), |a| q(format!("{a:?}")));
    match key {
        GlyphKey::Base { class, .. } => {
            ("symbol", q(format!("{class:?}")), aff, null.clone(), null)
        }
        GlyphKey::Geometric { class, .. } => {
            ("geometric", q(format!("{class:?}")), aff, null.clone(), null)
        }
        GlyphKey::Group { class, .. } => {
            ("group", q(format!("{class:?}")), aff, null.clone(), null)
        }
        GlyphKey::GroupGeometric { class, .. } => {
            ("group-geo", q(format!("{class:?}")), aff, null.clone(), null)
        }
        GlyphKey::Unknown { .. } => ("unknown", null.clone(), aff, null.clone(), null),
        GlyphKey::UnknownGeometric { .. } => {
            ("unknown-geo", null.clone(), aff, null.clone(), null)
        }
        GlyphKey::Heading(i) => {
            ("heading", null.clone(), null.clone(), (f64::from(i) * 5.0).to_string(), null)
        }
        GlyphKey::Bda(b) => {
            let name = match b {
                BdaDecoration::Uncertain => "Uncertain",
                BdaDecoration::ProbablyDestroyed => "ProbablyDestroyed",
            };
            ("bda", null.clone(), null.clone(), null, q(name.to_string()))
        }
    }
}

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
