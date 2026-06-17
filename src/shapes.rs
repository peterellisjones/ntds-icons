//! Abstract NTDS symbol shape descriptions.
//!
//! Pure-geometry representation of NTDS tactical symbols as sequences of
//! [`ShapeCmd`] drawing commands. Zero dependencies — only `std::f32::consts`
//! — so the font generator and direct-drawing consumers (map gizmos, SVG
//! export) share one geometry definition.

use std::f32::consts::{FRAC_PI_2, PI};

// ---------------------------------------------------------------------------
// Public enums
// ---------------------------------------------------------------------------

/// NTDS classification of a contact or entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NtdsShapeClass {
    Air,
    Helicopter,
    Surface,
    Subsurface,
    CommandShip,
    Land,
    Missile,
    Torpedo,
}

impl NtdsShapeClass {
    /// All variants in enum-declaration order.
    pub const ALL: [Self; 8] = [
        Self::Air,
        Self::Helicopter,
        Self::Surface,
        Self::Subsurface,
        Self::CommandShip,
        Self::Land,
        Self::Missile,
        Self::Torpedo,
    ];
}

/// NTDS classification for a group entity.
///
/// Groups use only 3 classifications (no Helicopter, Missile, Torpedo,
/// `CommandShip`, or Land). Rendered as a doubled concentric outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupNtdsShapeClass {
    Air,
    Surface,
    Submarine,
}

impl GroupNtdsShapeClass {
    /// All variants in enum-declaration order.
    pub const ALL: [Self; 3] = [Self::Air, Self::Surface, Self::Submarine];

    /// Map to the underlying individual shape class.
    #[must_use]
    pub fn to_shape_class(self) -> NtdsShapeClass {
        match self {
            Self::Air => NtdsShapeClass::Air,
            Self::Surface => NtdsShapeClass::Surface,
            Self::Submarine => NtdsShapeClass::Subsurface,
        }
    }
}

/// Affiliation of a contact relative to the observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeAffiliation {
    Friend,
    Enemy,
    Unknown,
    Neutral,
}

impl ShapeAffiliation {
    /// All variants in enum-declaration order.
    pub const ALL: [Self; 4] = [Self::Friend, Self::Enemy, Self::Unknown, Self::Neutral];
}

/// An abstract drawing command for one piece of an NTDS symbol.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeCmd {
    /// Stroke a full circle.
    Circle { cx: f32, cy: f32, r: f32 },
    /// Stroke a circular arc.
    ///
    /// `direction_angle` is the angle (radians) from center to the arc's
    /// midpoint, measured counter-clockwise from +X. `arc_angle` is the
    /// total angular extent of the arc (always positive).
    ///
    /// This matches Bevy's `Gizmos::arc_2d` parameterization where the
    /// isometry rotation equals `direction_angle` and the arc sweeps
    /// `arc_angle` symmetrically around that direction.
    Arc { cx: f32, cy: f32, r: f32, direction_angle: f32, arc_angle: f32 },
    /// Stroke a single line segment.
    Line { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// Stroke a closed polygon (first point == last point in the output).
    ClosedLineStrip(Vec<[f32; 2]>),
    /// Stroke an open polyline.
    LineStrip(Vec<[f32; 2]>),
    /// Fill a circle (used for the center dot).
    FilledCircle { cx: f32, cy: f32, r: f32 },
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the abstract shape commands that describe an NTDS symbol.
///
/// All coordinates are in the caller's coordinate system — pass whatever
/// center and radius are appropriate for your use case (screen pixels for
/// gizmos, font units for the font generator, etc.).
#[must_use]
pub fn describe_symbol(
    class: NtdsShapeClass,
    aff: ShapeAffiliation,
    cx: f32,
    cy: f32,
    radius: f32,
) -> Vec<ShapeCmd> {
    let mut cmds = Vec::new();

    // Base shape
    emit_base_shape(&mut cmds, class, aff, cx, cy, radius);

    // Interior decoration
    match class {
        NtdsShapeClass::Missile => emit_letter_m(&mut cmds, cx, cy, radius),
        NtdsShapeClass::Torpedo => emit_letter_t(&mut cmds, cx, cy, radius),
        NtdsShapeClass::Surface
        | NtdsShapeClass::Land
        | NtdsShapeClass::Air
        | NtdsShapeClass::Helicopter
        | NtdsShapeClass::Subsurface
        | NtdsShapeClass::CommandShip => {
            emit_stationary_dot(&mut cmds, cx, cy, radius);
        }
    }

    cmds
}

/// A node in the coarse-to-fine NTDS classification tree, for rendering graded
/// identity (DC-36.5). Mirrors `dc_types::NtdsNode` but stays game-agnostic so
/// this crate keeps zero game dependencies. Intermediate nodes render the coarse
/// environment shape; the specific `Air` leaf adds a fixed-wing glyph so it reads
/// distinctly from generic air.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NtdsNodeShape {
    /// Environment unresolved — a bare affiliation outline ("pending").
    Unknown,
    /// Generic air frame (fixed-wing / helicopter / missile not yet separated).
    AirGeneric,
    /// Generic surface vessel (surface combatant / command ship not separated).
    SurfaceGeneric,
    /// Generic subsurface contact (submarine / torpedo not separated).
    SubsurfaceGeneric,
    /// A fully-resolved class.
    Specific(NtdsShapeClass),
}

/// Return the shape commands for a coarse-to-fine NTDS tree node (DC-36.5).
///
/// Intermediate nodes render the environment base shape with no class-specific
/// interior; `Unknown` is a bare affiliation outline. The specific `Air` leaf
/// gains a fixed-wing glyph (a small upward chevron) so it is distinguishable
/// from generic air, whose icon is the upper-half base alone.
#[must_use]
pub fn describe_node_symbol(
    node: NtdsNodeShape,
    aff: ShapeAffiliation,
    cx: f32,
    cy: f32,
    radius: f32,
) -> Vec<ShapeCmd> {
    match node {
        NtdsNodeShape::Specific(class) => {
            let mut cmds = describe_symbol(class, aff, cx, cy, radius);
            if class == NtdsShapeClass::Air {
                emit_fixed_wing_glyph(&mut cmds, cx, cy, radius);
            }
            cmds
        }
        // Generic air: the upper-half base + stationary dot, no fixed-wing glyph.
        NtdsNodeShape::AirGeneric => describe_symbol(NtdsShapeClass::Air, aff, cx, cy, radius),
        // Generic surface / subsurface render as the plain environment shape; the
        // CommandShip cross / specific decorations only appear at a resolved leaf.
        NtdsNodeShape::SurfaceGeneric => {
            describe_symbol(NtdsShapeClass::Surface, aff, cx, cy, radius)
        }
        NtdsNodeShape::SubsurfaceGeneric => {
            describe_symbol(NtdsShapeClass::Subsurface, aff, cx, cy, radius)
        }
        // Unknown environment: a bare full affiliation outline, no interior dot.
        NtdsNodeShape::Unknown => {
            let mut cmds = Vec::new();
            emit_base_shape(&mut cmds, NtdsShapeClass::Surface, aff, cx, cy, radius);
            cmds
        }
    }
}

/// A small upward chevron in the upper half of an air symbol, marking a
/// fixed-wing (vs generic-air / helicopter / missile) airframe (DC-36.5).
fn emit_fixed_wing_glyph(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    cmds.push(ShapeCmd::LineStrip(vec![
        [cx - 0.35 * r, cy + 0.30 * r],
        [cx, cy + 0.60 * r],
        [cx + 0.35 * r, cy + 0.30 * r],
    ]));
}

/// BDA (Battle Damage Assessment) decoration overlay for stale contacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BdaDecoration {
    /// X through symbol — two diagonal lines, corner to corner. High-confidence kill.
    ProbablyDestroyed,
    /// Single diagonal slash — bottom-left to top-right. Engaged but unverified.
    Uncertain,
}

/// Return shape commands for a BDA decoration overlay.
///
/// The decoration covers the full bounding box defined by `(cx, cy, radius)`,
/// even for half-symbols — this keeps the overlay visually clear without
/// needing per-class adjustments.
#[must_use]
pub fn describe_bda_decoration(
    decoration: BdaDecoration,
    cx: f32,
    cy: f32,
    radius: f32,
) -> Vec<ShapeCmd> {
    let mut cmds = Vec::new();
    match decoration {
        BdaDecoration::ProbablyDestroyed => {
            // X: two diagonal lines corner to corner
            cmds.push(ShapeCmd::Line {
                x1: cx - radius,
                y1: cy - radius,
                x2: cx + radius,
                y2: cy + radius,
            });
            cmds.push(ShapeCmd::Line {
                x1: cx - radius,
                y1: cy + radius,
                x2: cx + radius,
                y2: cy - radius,
            });
        }
        BdaDecoration::Uncertain => {
            // Single diagonal slash: bottom-left to top-right
            cmds.push(ShapeCmd::Line {
                x1: cx - radius,
                y1: cy - radius,
                x2: cx + radius,
                y2: cy + radius,
            });
        }
    }
    cmds
}

/// Length of heading vector line as a multiple of the symbol radius.
pub const HEADING_VECTOR_LENGTH_RATIO: f32 = 2.0;

/// Return shape commands for a heading vector line from symbol center
/// outward at the given angle.
///
/// The line starts at `(cx, cy)` and extends to distance
/// `HEADING_VECTOR_LENGTH_RATIO * radius` from center.
///
/// `angle` is in radians, measured counter-clockwise from +X
/// (matching Bevy's 2D convention and the existing `direction_angle`
/// parameterization used by [`ShapeCmd::Arc`]).
#[must_use]
pub fn describe_heading_vector(cx: f32, cy: f32, radius: f32, angle: f32) -> Vec<ShapeCmd> {
    let length = HEADING_VECTOR_LENGTH_RATIO * radius;
    vec![ShapeCmd::Line {
        x1: cx,
        y1: cy,
        x2: cx + length * angle.cos(),
        y2: cy + length * angle.sin(),
    }]
}

/// Ratio of outer to inner radius for group doubled-outline symbols.
pub const GROUP_OUTER_RADIUS_RATIO: f32 = 1.3;

/// Return the abstract shape commands for a group NTDS symbol.
///
/// Group symbols render as a doubled concentric outline: the standard base
/// shape at `radius` plus a second copy at `radius * GROUP_OUTER_RADIUS_RATIO`.
/// Heading vector overlays are compatible; BDA overlays are not supported.
#[must_use]
pub fn describe_group_symbol(
    class: GroupNtdsShapeClass,
    aff: ShapeAffiliation,
    cx: f32,
    cy: f32,
    radius: f32,
) -> Vec<ShapeCmd> {
    let mut cmds = Vec::new();
    let shape_class = class.to_shape_class();

    // Inner outline at standard radius
    emit_base_shape(&mut cmds, shape_class, aff, cx, cy, radius);

    // Outer concentric outline at GROUP_OUTER_RADIUS_RATIO × radius
    emit_base_shape(&mut cmds, shape_class, aff, cx, cy, radius * GROUP_OUTER_RADIUS_RATIO);

    // Center dot
    emit_stationary_dot(&mut cmds, cx, cy, radius);

    cmds
}

/// Compute the Y-coordinate of a symbol's geometric center.
///
/// Returns the vertical midpoint of the base shape (circle, diamond, square,
/// or their half variants) — ignoring decorations (dot, letter M/T, carrier
/// lines, helicopter ears) and stroke expansion. Used by the font generator
/// to align glyphs consistently.
#[must_use]
pub fn base_shape_center_y(
    class: NtdsShapeClass,
    aff: ShapeAffiliation,
    cy: f32,
    radius: f32,
) -> f32 {
    #[allow(clippy::match_same_arms)] // Friend (semicircle) and Enemy (half-diamond) both
    // yield `radius` but for different geometric reasons — keep separate arms to document each.
    let half_extent = match (class, aff) {
        // Full shapes — symmetric about cy
        (NtdsShapeClass::Surface | NtdsShapeClass::Land | NtdsShapeClass::CommandShip, _) => {
            return cy;
        }

        // Half shapes — extent depends on affiliation's base shape
        (_, ShapeAffiliation::Friend) => radius, // semicircle
        (_, ShapeAffiliation::Enemy) => radius,  // half-diamond tip at cy ± r
        (_, ShapeAffiliation::Unknown) => radius, // half-square (half-side = r)
        (_, ShapeAffiliation::Neutral) => radius, // half-cross (half-side = r)
    };

    let offset = half_extent / 2.0;

    match class {
        // Upper half
        NtdsShapeClass::Air | NtdsShapeClass::Missile | NtdsShapeClass::Helicopter => cy + offset,
        // Lower half
        NtdsShapeClass::Subsurface | NtdsShapeClass::Torpedo => cy - offset,
        // Full shapes handled above
        NtdsShapeClass::Surface | NtdsShapeClass::CommandShip | NtdsShapeClass::Land => {
            unreachable!()
        }
    }
}

/// Tunable perceived-center factors (fraction of radius) per affiliation shape.
///
/// Full shapes are symmetric and need no correction (factor = 0).
/// Half shapes use these factors instead of the geometric 0.5 used by
/// [`base_shape_center_y`]. The eye perceives the center of shapes with
/// varying visual weight distributions differently from the bounding-box
/// midpoint.
const PERCEIVED_SEMICIRCLE: f32 = 0.33;
const PERCEIVED_HALF_DIAMOND: f32 = 0.42;
const PERCEIVED_HALF_SQUARE: f32 = 0.38;
const PERCEIVED_HALF_CROSS: f32 = 0.38;

/// Compute the Y-coordinate of a symbol's perceived visual center.
///
/// Like [`base_shape_center_y`] but uses per-shape tuning factors that
/// account for visual weight distribution. Used by the font generator
/// for the geometrically-centered glyph variant.
#[must_use]
pub fn perceived_center_y(
    class: NtdsShapeClass,
    aff: ShapeAffiliation,
    cy: f32,
    radius: f32,
) -> f32 {
    let factor = match (class, aff) {
        // Full shapes — symmetric about cy
        (NtdsShapeClass::Surface | NtdsShapeClass::Land | NtdsShapeClass::CommandShip, _) => {
            return cy;
        }
        (_, ShapeAffiliation::Friend) => PERCEIVED_SEMICIRCLE,
        (_, ShapeAffiliation::Enemy) => PERCEIVED_HALF_DIAMOND,
        (_, ShapeAffiliation::Unknown) => PERCEIVED_HALF_SQUARE,
        (_, ShapeAffiliation::Neutral) => PERCEIVED_HALF_CROSS,
    };

    let offset = radius * factor;

    match class {
        NtdsShapeClass::Air | NtdsShapeClass::Missile | NtdsShapeClass::Helicopter => cy + offset,
        NtdsShapeClass::Subsurface | NtdsShapeClass::Torpedo => cy - offset,
        NtdsShapeClass::Surface | NtdsShapeClass::CommandShip | NtdsShapeClass::Land => {
            unreachable!()
        }
    }
}

// ---------------------------------------------------------------------------
// Base shape dispatch
// ---------------------------------------------------------------------------

fn emit_base_shape(
    cmds: &mut Vec<ShapeCmd>,
    class: NtdsShapeClass,
    aff: ShapeAffiliation,
    cx: f32,
    cy: f32,
    r: f32,
) {
    match (class, aff) {
        // CommandShip — base shape + carrier lines
        (NtdsShapeClass::CommandShip, ShapeAffiliation::Friend) => {
            emit_circle(cmds, cx, cy, r);
            emit_carrier_lines(cmds, aff, cx, cy, r);
        }
        (NtdsShapeClass::CommandShip, ShapeAffiliation::Enemy) => {
            emit_diamond(cmds, cx, cy, r);
            emit_carrier_lines(cmds, aff, cx, cy, r);
        }
        (NtdsShapeClass::CommandShip, ShapeAffiliation::Unknown) => {
            emit_square(cmds, cx, cy, r);
            emit_carrier_lines(cmds, aff, cx, cy, r);
        }
        (NtdsShapeClass::CommandShip, ShapeAffiliation::Neutral) => {
            emit_cross(cmds, cx, cy, r);
            emit_carrier_lines(cmds, aff, cx, cy, r);
        }

        // Surface / Land
        (NtdsShapeClass::Surface | NtdsShapeClass::Land, ShapeAffiliation::Friend) => {
            emit_circle(cmds, cx, cy, r);
        }
        (NtdsShapeClass::Surface | NtdsShapeClass::Land, ShapeAffiliation::Enemy) => {
            emit_diamond(cmds, cx, cy, r);
        }
        (NtdsShapeClass::Surface | NtdsShapeClass::Land, ShapeAffiliation::Unknown) => {
            emit_square(cmds, cx, cy, r);
        }
        (NtdsShapeClass::Surface | NtdsShapeClass::Land, ShapeAffiliation::Neutral) => {
            emit_cross(cmds, cx, cy, r);
        }

        // Air / Missile — upper half shapes
        (NtdsShapeClass::Air | NtdsShapeClass::Missile, ShapeAffiliation::Friend) => {
            emit_semicircle(cmds, cx, cy, r, true);
        }
        (NtdsShapeClass::Air | NtdsShapeClass::Missile, ShapeAffiliation::Enemy) => {
            emit_half_diamond(cmds, cx, cy, r, true);
        }
        (NtdsShapeClass::Air | NtdsShapeClass::Missile, ShapeAffiliation::Unknown) => {
            emit_half_square(cmds, cx, cy, r, true);
        }
        (NtdsShapeClass::Air | NtdsShapeClass::Missile, ShapeAffiliation::Neutral) => {
            emit_half_cross(cmds, cx, cy, r, true);
        }

        // Helicopter — upper half + ears
        (NtdsShapeClass::Helicopter, ShapeAffiliation::Friend) => {
            emit_semicircle(cmds, cx, cy, r, true);
            emit_helicopter_ears(cmds, aff, cx, cy, r);
        }
        (NtdsShapeClass::Helicopter, ShapeAffiliation::Enemy) => {
            emit_half_diamond(cmds, cx, cy, r, true);
            emit_helicopter_ears(cmds, aff, cx, cy, r);
        }
        (NtdsShapeClass::Helicopter, ShapeAffiliation::Unknown) => {
            emit_half_square(cmds, cx, cy, r, true);
            emit_helicopter_ears(cmds, aff, cx, cy, r);
        }
        (NtdsShapeClass::Helicopter, ShapeAffiliation::Neutral) => {
            emit_half_cross(cmds, cx, cy, r, true);
            emit_helicopter_ears(cmds, aff, cx, cy, r);
        }

        // Subsurface / Torpedo — lower half shapes
        (NtdsShapeClass::Subsurface | NtdsShapeClass::Torpedo, ShapeAffiliation::Friend) => {
            emit_semicircle(cmds, cx, cy, r, false);
        }
        (NtdsShapeClass::Subsurface | NtdsShapeClass::Torpedo, ShapeAffiliation::Enemy) => {
            emit_half_diamond(cmds, cx, cy, r, false);
        }
        (NtdsShapeClass::Subsurface | NtdsShapeClass::Torpedo, ShapeAffiliation::Unknown) => {
            emit_half_square(cmds, cx, cy, r, false);
        }
        (NtdsShapeClass::Subsurface | NtdsShapeClass::Torpedo, ShapeAffiliation::Neutral) => {
            emit_half_cross(cmds, cx, cy, r, false);
        }
    }
}

// ---------------------------------------------------------------------------
// Primitive shapes
// ---------------------------------------------------------------------------

fn emit_circle(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    cmds.push(ShapeCmd::Circle { cx, cy, r });
}

/// Half circle arc.
///
/// `open_at_bottom=true` → upper semicircle (friendly air): `direction_angle = -PI/2`, `arc_angle = PI`
/// `open_at_bottom=false` → lower semicircle (friendly sub): `direction_angle = PI/2`, `arc_angle = PI`
fn emit_semicircle(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32, open_at_bottom: bool) {
    let direction_angle = if open_at_bottom { -FRAC_PI_2 } else { FRAC_PI_2 };
    cmds.push(ShapeCmd::Arc { cx, cy, r, direction_angle, arc_angle: PI });
}

/// Diamond (4-point rhombus) with vertices at distance `r` along each axis.
fn emit_diamond(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    cmds.push(ShapeCmd::ClosedLineStrip(vec![
        [cx - r, cy],
        [cx, cy + r],
        [cx + r, cy],
        [cx, cy - r],
        [cx - r, cy],
    ]));
}

/// Axis-aligned square with half-side equal to `r`, matching the visual extent
/// of circles and diamonds that use the same radius.
fn emit_square(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    cmds.push(ShapeCmd::ClosedLineStrip(vec![
        [cx - r, cy + r],
        [cx + r, cy + r],
        [cx + r, cy - r],
        [cx - r, cy - r],
        [cx - r, cy + r],
    ]));
}

/// Upper or lower half of a diamond. `open_at_bottom=true` gives the upper half (^).
fn emit_half_diamond(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32, open_at_bottom: bool) {
    let tip_y = if open_at_bottom { cy + r } else { cy - r };
    cmds.push(ShapeCmd::LineStrip(vec![[cx - r, cy], [cx, tip_y], [cx + r, cy]]));
}

/// Upper or lower half of a square. `open_at_bottom=true` gives a ⊓ shape (open at bottom).
fn emit_half_square(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32, open_at_bottom: bool) {
    let bar_y = if open_at_bottom { cy + r } else { cy - r };
    cmds.push(ShapeCmd::LineStrip(vec![[cx - r, cy], [cx - r, bar_y], [cx + r, bar_y], [
        cx + r,
        cy,
    ]]));
}

/// Inset fraction for cross shape — determines the width of each arm relative
/// to the overall radius. A cross is a 12-sided polygon (square with notches).
const CROSS_INSET: f32 = 0.60;

/// Cross shape (neutral affiliation) — a 12-sided polygon formed by cutting
/// notches from each corner of a square.
fn emit_cross(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    let i = r * CROSS_INSET;
    cmds.push(ShapeCmd::ClosedLineStrip(vec![
        [cx - i, cy + r],
        [cx + i, cy + r],
        [cx + i, cy + i],
        [cx + r, cy + i],
        [cx + r, cy - i],
        [cx + i, cy - i],
        [cx + i, cy - r],
        [cx - i, cy - r],
        [cx - i, cy - i],
        [cx - r, cy - i],
        [cx - r, cy + i],
        [cx - i, cy + i],
        [cx - i, cy + r],
    ]));
}

/// Upper or lower half of a cross. `open_at_bottom=true` gives the upper half
/// (for air/helicopter), `false` gives the lower half (for subsurface/torpedo).
fn emit_half_cross(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32, open_at_bottom: bool) {
    let i = r * CROSS_INSET;
    let arm_y = if open_at_bottom { cy + r } else { cy - r };
    let notch_y = if open_at_bottom { cy + i } else { cy - i };
    cmds.push(ShapeCmd::LineStrip(vec![
        [cx - r, cy],
        [cx - r, notch_y],
        [cx - i, notch_y],
        [cx - i, arm_y],
        [cx + i, arm_y],
        [cx + i, notch_y],
        [cx + r, notch_y],
        [cx + r, cy],
    ]));
}

// ---------------------------------------------------------------------------
// Decorations
// ---------------------------------------------------------------------------

/// Two vertical lines inside the symbol, clipped to the containing shape boundary.
fn emit_carrier_lines(cmds: &mut Vec<ShapeCmd>, aff: ShapeAffiliation, cx: f32, cy: f32, r: f32) {
    let dx = r * 0.3;
    let h = match aff {
        ShapeAffiliation::Friend => (r * r - dx * dx).sqrt(),
        ShapeAffiliation::Enemy => r - dx,
        ShapeAffiliation::Unknown | ShapeAffiliation::Neutral => r,
    };
    // Left line
    cmds.push(ShapeCmd::Line { x1: cx - dx, y1: cy - h, x2: cx - dx, y2: cy + h });
    // Right line
    cmds.push(ShapeCmd::Line { x1: cx + dx, y1: cy - h, x2: cx + dx, y2: cy + h });
}

/// Helicopter "ears" — angled lines from bounding-box corners to the symbol body,
/// plus short horizontal "rotor" lines.
fn emit_helicopter_ears(cmds: &mut Vec<ShapeCmd>, aff: ShapeAffiliation, cx: f32, cy: f32, r: f32) {
    let left_tip = [cx - r, cy + r];
    let right_tip = [cx + r, cy + r];

    let (left_base, right_base) = match aff {
        ShapeAffiliation::Friend | ShapeAffiliation::Unknown | ShapeAffiliation::Neutral => {
            ([cx - r, cy + r], [cx + r, cy + r])
        }
        ShapeAffiliation::Enemy => ([cx - r * 0.5, cy + r * 0.5], [cx + r * 0.5, cy + r * 0.5]),
    };

    let rotor = r * 0.3;

    // Left ear line
    cmds.push(ShapeCmd::Line {
        x1: left_base[0],
        y1: left_base[1],
        x2: left_tip[0],
        y2: left_tip[1],
    });
    // Right ear line
    cmds.push(ShapeCmd::Line {
        x1: right_base[0],
        y1: right_base[1],
        x2: right_tip[0],
        y2: right_tip[1],
    });
    // Left rotor
    cmds.push(ShapeCmd::Line {
        x1: left_tip[0],
        y1: left_tip[1],
        x2: left_tip[0] - rotor,
        y2: left_tip[1],
    });
    // Right rotor
    cmds.push(ShapeCmd::Line {
        x1: right_tip[0],
        y1: right_tip[1],
        x2: right_tip[0] + rotor,
        y2: right_tip[1],
    });
}

/// Letter 'M' drawn with line segments.
fn emit_letter_m(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    let s = r * 0.4;
    let h = s;
    let w = s * 0.6;
    cmds.push(ShapeCmd::LineStrip(vec![
        [cx - w, cy - h],   // bottom-left
        [cx - w, cy + h],   // top-left
        [cx, cy - h * 0.3], // center-bottom
        [cx + w, cy + h],   // top-right
        [cx + w, cy - h],   // bottom-right
    ]));
}

/// Letter 'T' drawn with two line segments.
fn emit_letter_t(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    let s = r * 0.4;
    // Horizontal bar
    cmds.push(ShapeCmd::Line { x1: cx - s, y1: cy, x2: cx + s, y2: cy });
    // Vertical stem
    cmds.push(ShapeCmd::Line { x1: cx, y1: cy, x2: cx, y2: cy - s });
}

/// Small filled dot at center.
fn emit_stationary_dot(cmds: &mut Vec<ShapeCmd>, cx: f32, cy: f32, r: f32) {
    cmds.push(ShapeCmd::FilledCircle { cx, cy, r: r * 0.1 });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
