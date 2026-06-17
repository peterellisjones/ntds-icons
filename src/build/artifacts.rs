//! Emit the codepoint map (JSON) and the README cheat-sheet table (Markdown)
//! from the shared [`crate::codepoints::all_glyphs`] data, so neither drifts
//! from the font.

use crate::codepoints::all_glyphs;

/// The codepoint map as JSON: one `{ "codepoint", "name" }` object per glyph,
/// in codepoint order. Hand-rolled so the default build needs no serde.
#[must_use]
pub fn codepoints_json() -> String {
    let entries: Vec<_> = all_glyphs().collect();
    let mut s = String::from("[\n");
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

/// The README codepoint cheat-sheet as a Markdown table.
#[must_use]
pub fn readme_table() -> String {
    let mut s = String::from("| Codepoint | Glyph name |\n|---|---|\n");
    for e in all_glyphs() {
        s.push_str(&format!("| `U+{:04X}` | `{}` |\n", e.codepoint as u32, e.name));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_has_170_entries_and_anchors() {
        let j = codepoints_json();
        assert_eq!(j.matches("\"codepoint\"").count(), 170);
        assert!(j.contains("\"U+E000\""));
        assert!(j.contains("\"U+E0A9\""));
        assert!(j.contains("\"Air.Friend\""));
    }

    #[test]
    fn table_has_header_and_rows() {
        let t = readme_table();
        assert!(t.starts_with("| Codepoint |"));
        assert_eq!(t.matches("| `U+").count(), 170);
    }
}
