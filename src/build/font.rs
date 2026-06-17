//! TrueType font generation: `ShapeCmd` → kurbo stroke-expansion → write-fonts.
//!
//! This is a faithful port of dronecom's `crates/dc_ntds_icons/build.rs`, lifted
//! out of a build script into a public, parameterized [`build_font`] API. The
//! eight inline glyph-generation loops of the legacy `generate_all_glyphs` are
//! replaced by one [`glyph_geometry`] switch driven by [`crate::layout`].
//!
//! ## Parity map (legacy `build.rs` → here)
//!
//! | Legacy (`build.rs`)                          | Here                                  |
//! |----------------------------------------------|---------------------------------------|
//! | `translate_cmds` (L94–202)                   | [`translate_cmds`] (verbatim)         |
//! | `stroke_and_flatten` (L209–222)              | [`stroke_and_flatten`] (params from `FontLayout`) |
//! | `flatten_into` (L226–230)                    | [`flatten_into`] (verbatim)           |
//! | `build_glyph` (L237–252)                     | [`build_glyph`] (params from `FontLayout`; name split out) |
//! | `generate_all_glyphs` 8 loops (L271–409)     | [`glyph_geometry`] 8 arms + the `keys_in_order` loop in [`build_font`] |
//! | glyf/loca + bbox/maxp scan (L433–456)        | [`build_font`] glyf section          |
//! | cmap index arithmetic (L459–535)             | [`build_font`] sequential cmap        |
//! | head/hhea/maxp/os2/post/name (L543–677)      | [`build_font`] table section          |
//! | hmtx advance ranges (L581–610)               | [`build_font`] `key.advance()`        |
//! | `$OUT_DIR` / assets writes (L694–706)        | moved to the `ntds-font-gen` CLI      |
//!
//! Named behavior change: the `UNIQUE_ID` name-table string drops the legacy
//! `DroneCarrier-` prefix — the standalone font is not dronecom-branded. This is
//! why equivalence with dronecom's font is structural (cmap + outlines), not a
//! byte match against the legacy file.

use std::f64::consts::{PI, TAU};

use kurbo::{Affine, Arc, BezPath, Cap, Join, Point, Stroke, StrokeOpts, Vec2};
use write_fonts::{
    FontBuilder,
    tables::{
        cmap::Cmap,
        glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph},
        head::Head,
        hhea::Hhea,
        hmtx::{Hmtx, LongMetric},
        maxp::Maxp,
        name::{Name, NameRecord},
        os2::Os2,
        post::Post,
    },
    types::{Fixed, GlyphId, NameId},
};

use crate::layout::{self, Advance, GlyphKey};
use crate::shapes::{
    NtdsNodeShape, NtdsShapeClass, ShapeCmd, describe_bda_decoration, describe_group_symbol,
    describe_heading_vector, describe_node_symbol, describe_symbol, perceived_center_y,
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Name-table strings for the generated font.
pub struct FontNames {
    pub family: String,
    pub subfamily: String,
    pub unique_id: String,
    pub full_name: String,
    pub version: String,
    pub postscript: String,
}

/// Tunable parameters for [`build_font`]. [`FontLayout::default`] reproduces the
/// committed [`crate::FONT_TTF`] byte-for-byte.
pub struct FontLayout {
    pub units_per_em: u16,
    pub symbol_radius: f64,
    pub stroke_width: f64,
    pub advance_width: u16,
    /// Origin both base symbols and overlays are built around.
    pub center: f64,
    /// Vertical position the construction center is shifted to.
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

// ---------------------------------------------------------------------------
// ShapeCmd → kurbo path translation (port of build.rs L94–202)
// ---------------------------------------------------------------------------

/// Convert `ShapeCmd`s into separate stroked and filled `BezPath` collections.
#[allow(clippy::too_many_lines)]
fn translate_cmds(cmds: &[ShapeCmd], tol: f64) -> (Vec<BezPath>, Vec<BezPath>) {
    let mut stroked = Vec::new();
    let mut filled = Vec::new();

    for cmd in cmds {
        match cmd {
            ShapeCmd::Circle { cx, cy, r } => {
                let (cx, cy, r) = (f64::from(*cx), f64::from(*cy), f64::from(*r));
                let arc = Arc::new(Point::new(cx, cy), Vec2::new(r, r), 0.0, TAU, 0.0);
                let mut path = BezPath::new();
                path.move_to(Point::new(cx + r, cy));
                for el in arc.append_iter(tol) {
                    path.push(el);
                }
                path.close_path();
                stroked.push(path);
            }
            ShapeCmd::Arc { cx, cy, r, direction_angle, arc_angle } => {
                let (cx, cy, r) = (f64::from(*cx), f64::from(*cy), f64::from(*r));
                let direction = f64::from(*direction_angle);
                let sweep = f64::from(*arc_angle);
                // Bevy-native parameterization → kurbo: start = direction + PI/2.
                let kurbo_start = direction + PI / 2.0;
                let kurbo_sweep = sweep;

                let arc =
                    Arc::new(Point::new(cx, cy), Vec2::new(r, r), kurbo_start, kurbo_sweep, 0.0);
                let start_pt = Point::new(cx + r * kurbo_start.cos(), cy + r * kurbo_start.sin());
                let mut path = BezPath::new();
                path.move_to(start_pt);
                for el in arc.append_iter(tol) {
                    path.push(el);
                }
                // Open arc — do NOT close, so the stroke gets end caps.
                stroked.push(path);
            }
            ShapeCmd::Line { x1, y1, x2, y2 } => {
                let mut path = BezPath::new();
                path.move_to(Point::new(f64::from(*x1), f64::from(*y1)));
                path.line_to(Point::new(f64::from(*x2), f64::from(*y2)));
                stroked.push(path);
            }
            ShapeCmd::ClosedLineStrip(pts) => {
                let mut path = BezPath::new();
                for (i, pt) in pts.iter().enumerate() {
                    let p = Point::new(f64::from(pt[0]), f64::from(pt[1]));
                    if i == 0 {
                        path.move_to(p);
                    } else {
                        path.line_to(p);
                    }
                }
                path.close_path();
                stroked.push(path);
            }
            ShapeCmd::LineStrip(pts) => {
                let mut path = BezPath::new();
                for (i, pt) in pts.iter().enumerate() {
                    let p = Point::new(f64::from(pt[0]), f64::from(pt[1]));
                    if i == 0 {
                        path.move_to(p);
                    } else {
                        path.line_to(p);
                    }
                }
                stroked.push(path);
            }
            ShapeCmd::FilledCircle { cx, cy, r } => {
                let (cx, cy, r) = (f64::from(*cx), f64::from(*cy), f64::from(*r));
                let arc = Arc::new(Point::new(cx, cy), Vec2::new(r, r), 0.0, TAU, 0.0);
                let mut path = BezPath::new();
                path.move_to(Point::new(cx + r, cy));
                for el in arc.append_iter(tol) {
                    path.push(el);
                }
                path.close_path();
                filled.push(path);
            }
        }
    }

    (stroked, filled)
}

// ---------------------------------------------------------------------------
// Stroke expansion + flattening (port of build.rs L209–230)
// ---------------------------------------------------------------------------

/// Expand stroked paths into filled outlines, then flatten to line segments.
fn stroke_and_flatten(paths: &[BezPath], l: &FontLayout) -> BezPath {
    let style = Stroke::new(l.stroke_width).with_join(l.join).with_caps(l.cap);
    let opts = StrokeOpts::default();
    let mut result = BezPath::new();
    for path in paths {
        let expanded = kurbo::stroke(path.iter(), &style, &opts, l.flatten_tolerance);
        flatten_into(&mut result, &expanded, l.flatten_tolerance);
    }
    result
}

/// Flatten a `BezPath` (cubics/quadratics) into pure line segments into `out`.
fn flatten_into(out: &mut BezPath, path: &BezPath, tol: f64) {
    kurbo::flatten(path.iter(), tol, |el| out.push(el));
}

// ---------------------------------------------------------------------------
// Glyph generation
// ---------------------------------------------------------------------------

/// Build a `SimpleGlyph` from shape commands with the given affine shift
/// (port of build.rs `build_glyph`, L237–252; name generation split out).
fn build_glyph(cmds: &[ShapeCmd], dx: f64, dy: f64, l: &FontLayout) -> SimpleGlyph {
    let (stroked_paths, filled_paths) = translate_cmds(cmds, l.flatten_tolerance);

    let mut combined = stroke_and_flatten(&stroked_paths, l);
    for fp in &filled_paths {
        flatten_into(&mut combined, fp, l.flatten_tolerance);
    }

    if dx.abs() > 0.5 || dy.abs() > 0.5 {
        combined.apply_affine(Affine::translate(Vec2::new(dx, dy)));
    }

    SimpleGlyph::from_bezpath(&combined)
        .unwrap_or_else(|e| panic!("failed to convert glyph to SimpleGlyph: {e:?}"))
}

/// Geometry + `(dx, dy)` affine shift for one glyph. Each arm reproduces the
/// `describe_*` call and the shift of its legacy loop in `generate_all_glyphs`
/// (build.rs L271–409).
fn glyph_geometry(key: GlyphKey, l: &FontLayout) -> (Vec<ShapeCmd>, f64, f64) {
    #[allow(clippy::cast_possible_truncation)]
    let c = l.center as f32;
    #[allow(clippy::cast_possible_truncation)]
    let r = l.symbol_radius as f32;
    let construction_dy = l.glyph_center_y - l.center; // GLYPH_CENTER_Y - CENTER
    let overlay_dx = -f64::from(l.advance_width); // overlay glyphs shift left one advance

    match key {
        GlyphKey::Base { class, aff } => {
            (describe_symbol(class, aff, c, c, r), 0.0, construction_dy)
        }
        GlyphKey::Bda(dec) => {
            (describe_bda_decoration(dec, c, c, r), overlay_dx, construction_dy)
        }
        GlyphKey::Heading(i) => {
            let degrees = f64::from(i) * 5.0;
            // Nautical degrees (CW from north/+Y) → math radians (CCW from +X).
            #[allow(clippy::cast_possible_truncation)]
            let math_angle = (90.0 - degrees).to_radians() as f32;
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
            let dy =
                l.glyph_center_y - f64::from(perceived_center_y(NtdsShapeClass::Surface, aff, c, r));
            (describe_node_symbol(NtdsNodeShape::Unknown, aff, c, c, r), 0.0, dy)
        }
    }
}

// ---------------------------------------------------------------------------
// Font assembly (port of build.rs main(), L419–692, minus file writes)
// ---------------------------------------------------------------------------

/// Generate the NTDS icon font as TrueType bytes.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_font(layout: &FontLayout) -> Vec<u8> {
    let keys = layout::keys_in_order();

    // Glyph 0 = .notdef; glyphs 1.. = keys_in_order (glyph id = position + 1).
    let mut glyphs: Vec<SimpleGlyph> = vec![SimpleGlyph::default()];
    let mut glyph_names: Vec<String> = vec![".notdef".to_string()];
    for &key in &keys {
        let (cmds, dx, dy) = glyph_geometry(key, layout);
        glyphs.push(build_glyph(&cmds, dx, dy, layout));
        glyph_names.push(layout::name(key));
    }

    let num_glyphs = u16::try_from(glyphs.len()).expect("glyph count fits u16");

    // glyf + loca, tracking global bbox + maxp limits.
    let mut glyf_builder = GlyfLocaBuilder::new();
    let mut global_bbox = write_fonts::tables::glyf::Bbox {
        x_min: i16::MAX,
        y_min: i16::MAX,
        x_max: i16::MIN,
        y_max: i16::MIN,
    };
    let mut max_points: u16 = 0;
    let mut max_contours: u16 = 0;

    for (i, glyph) in glyphs.iter().enumerate() {
        let g: Glyph = glyph.clone().into();
        glyf_builder.add_glyph(&g).unwrap_or_else(|e| panic!("failed to add glyph {i}: {e}"));

        if let Some(bbox) = g.bbox() {
            global_bbox = global_bbox.union(bbox);
        }
        let n_points: u16 = glyph.contours.iter().map(|c| c.iter().count() as u16).sum();
        let n_contours = u16::try_from(glyph.contours.len()).expect("contour count fits u16");
        max_points = max_points.max(n_points);
        max_contours = max_contours.max(n_contours);
    }

    let (glyf, loca, loca_format) = glyf_builder.build();

    // cmap — contiguous PUA codepoints, glyph id = position + 1.
    let mappings: Vec<(char, GlyphId)> = keys
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let cp = layout::PUA_START + u32::try_from(i).expect("index fits u32");
            let gid = u32::try_from(i + 1).expect("glyph id fits u32");
            (char::from_u32(cp).expect("valid PUA codepoint"), GlyphId::new(gid))
        })
        .collect();

    let first_char = layout::PUA_START as u16;
    let last_char = mappings.last().expect("at least one mapping").0 as u16;
    let cmap = Cmap::from_mappings(mappings).expect("cmap construction failed");

    // head
    let head = Head {
        units_per_em: layout.units_per_em,
        x_min: global_bbox.x_min,
        y_min: global_bbox.y_min,
        x_max: global_bbox.x_max,
        y_max: global_bbox.y_max,
        flags: write_fonts::tables::head::Flags::BASELINE_AT_Y_0
            | write_fonts::tables::head::Flags::LSB_AT_X_0,
        index_to_loc_format: match loca_format {
            write_fonts::tables::loca::LocaFormat::Short => 0,
            write_fonts::tables::loca::LocaFormat::Long => 1,
        },
        lowest_rec_ppem: 8,
        font_revision: Fixed::from_f64(1.0),
        ..Default::default()
    };

    // hhea
    let ascender: i16 = 800;
    let descender: i16 = -200;
    let hhea = Hhea {
        ascender: ascender.into(),
        descender: descender.into(),
        line_gap: 0i16.into(),
        advance_width_max: layout.advance_width.into(),
        min_left_side_bearing: global_bbox.x_min.into(),
        min_right_side_bearing: (layout.advance_width as i16 - global_bbox.x_max).into(),
        x_max_extent: global_bbox.x_max.into(),
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        number_of_h_metrics: num_glyphs,
    };

    // hmtx — base symbols advance the pen; overlays get zero advance.
    // Glyph 0 (.notdef) gets full advance, lsb 0 (matching legacy build.rs).
    let mut h_metrics: Vec<LongMetric> = vec![LongMetric::new(layout.advance_width, 0)];
    for (&key, g) in keys.iter().zip(glyphs.iter().skip(1)) {
        let advance = match key.advance() {
            Advance::Full => layout.advance_width,
            Advance::Zero => 0,
        };
        let lsb = if g.contours.is_empty() { 0 } else { g.bbox.x_min };
        h_metrics.push(LongMetric::new(advance, lsb));
    }
    let hmtx = Hmtx::new(h_metrics, vec![]);

    // maxp
    let maxp = Maxp {
        num_glyphs,
        max_points: Some(max_points),
        max_contours: Some(max_contours),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(1),
        max_twilight_points: Some(0),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(0),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };

    // post (v2 with glyph names)
    let glyph_name_refs: Vec<&str> = glyph_names.iter().map(String::as_str).collect();
    let post = Post::new_v2(glyph_name_refs);

    // name
    let n = &layout.names;
    let mut name_records = vec![
        NameRecord::new(3, 1, 0x0409, NameId::FAMILY_NAME, n.family.clone().into()),
        NameRecord::new(3, 1, 0x0409, NameId::SUBFAMILY_NAME, n.subfamily.clone().into()),
        NameRecord::new(3, 1, 0x0409, NameId::UNIQUE_ID, n.unique_id.clone().into()),
        NameRecord::new(3, 1, 0x0409, NameId::FULL_NAME, n.full_name.clone().into()),
        NameRecord::new(3, 1, 0x0409, NameId::VERSION_STRING, n.version.clone().into()),
        NameRecord::new(3, 1, 0x0409, NameId::POSTSCRIPT_NAME, n.postscript.clone().into()),
    ];
    name_records.sort();
    let name = Name::new(name_records);

    // OS/2
    let os2 = Os2 {
        us_weight_class: 400,
        us_width_class: 5,
        fs_type: 0,
        s_typo_ascender: ascender,
        s_typo_descender: descender,
        s_typo_line_gap: 0,
        us_win_ascent: ascender as u16,
        us_win_descent: descender.unsigned_abs(),
        us_first_char_index: first_char,
        us_last_char_index: last_char,
        fs_selection: write_fonts::tables::os2::SelectionFlags::REGULAR
            | write_fonts::tables::os2::SelectionFlags::USE_TYPO_METRICS,
        // Unicode range bit 57 (Private Use Area): word 2, bit 57-32 = 25.
        ul_unicode_range_2: 1 << 25,
        ..Default::default()
    };

    let mut builder = FontBuilder::new();
    builder.add_table(&head).expect("head table");
    builder.add_table(&hhea).expect("hhea table");
    builder.add_table(&maxp).expect("maxp table");
    builder.add_table(&os2).expect("OS/2 table");
    builder.add_table(&hmtx).expect("hmtx table");
    builder.add_table(&cmap).expect("cmap table");
    builder.add_table(&loca).expect("loca table");
    builder.add_table(&glyf).expect("glyf table");
    builder.add_table(&name).expect("name table");
    builder.add_table(&post).expect("post table");

    builder.build()
}

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
