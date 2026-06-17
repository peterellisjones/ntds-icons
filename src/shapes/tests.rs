//! Unit tests for NTDS symbol shape generation.
//!
//! Covers all public API functions: `describe_symbol`, `describe_bda_decoration`,
//! `describe_heading_vector`, `describe_group_symbol`, `base_shape_center_y`, and
//! `perceived_center_y`.

use std::f32::consts::{FRAC_PI_2, PI};

use super::{
    BdaDecoration, GROUP_OUTER_RADIUS_RATIO, GroupNtdsShapeClass, HEADING_VECTOR_LENGTH_RATIO,
    NtdsShapeClass, ShapeAffiliation, ShapeCmd, base_shape_center_y, describe_bda_decoration,
    describe_group_symbol, describe_heading_vector, describe_symbol, perceived_center_y,
};

#[test]
fn all_combinations_produce_shapes() {
    for class in NtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            let cmds = describe_symbol(class, aff, 0.0, 0.0, 100.0);
            assert!(!cmds.is_empty(), "no shapes for {class:?} / {aff:?}");
        }
    }
}

#[test]
fn friendly_surface_has_circle_and_dot() {
    let cmds = describe_symbol(NtdsShapeClass::Surface, ShapeAffiliation::Friend, 0.0, 0.0, 100.0);
    assert!(cmds.iter().any(|c| matches!(c, ShapeCmd::Circle { .. })), "expected a Circle command");
    assert!(
        cmds.iter().any(|c| matches!(c, ShapeCmd::FilledCircle { .. })),
        "expected a FilledCircle (dot) command"
    );
}

#[test]
fn enemy_air_has_half_diamond() {
    let cmds = describe_symbol(NtdsShapeClass::Air, ShapeAffiliation::Enemy, 0.0, 0.0, 100.0);
    // Half-diamond is a 3-point LineStrip
    let has_half_diamond =
        cmds.iter().any(|c| matches!(c, ShapeCmd::LineStrip(pts) if pts.len() == 3));
    assert!(has_half_diamond, "expected a 3-point LineStrip (half-diamond)");
}

#[test]
fn friendly_air_has_arc() {
    let cmds = describe_symbol(NtdsShapeClass::Air, ShapeAffiliation::Friend, 0.0, 0.0, 100.0);
    let has_arc = cmds.iter().any(|c| {
        matches!(c, ShapeCmd::Arc { direction_angle, arc_angle, .. }
            if (*direction_angle - (-FRAC_PI_2)).abs() < f32::EPSILON
            && (*arc_angle - PI).abs() < f32::EPSILON)
    });
    assert!(has_arc, "expected an upper semicircle Arc");
}

#[test]
fn missile_has_no_dot_but_has_lines() {
    let cmds = describe_symbol(NtdsShapeClass::Missile, ShapeAffiliation::Friend, 0.0, 0.0, 100.0);
    assert!(
        !cmds.iter().any(|c| matches!(c, ShapeCmd::FilledCircle { .. })),
        "missiles should not have a center dot"
    );
    // Letter M is a 5-point LineStrip
    let has_letter_m = cmds.iter().any(|c| matches!(c, ShapeCmd::LineStrip(pts) if pts.len() == 5));
    assert!(has_letter_m, "expected a 5-point LineStrip (letter M)");
}

#[test]
fn command_ship_has_carrier_lines() {
    for aff in ShapeAffiliation::ALL {
        let cmds = describe_symbol(NtdsShapeClass::CommandShip, aff, 0.0, 0.0, 100.0);
        let line_count = cmds.iter().filter(|c| matches!(c, ShapeCmd::Line { .. })).count();
        assert!(
            line_count >= 2,
            "expected at least 2 Line commands (carrier lines) for CommandShip/{aff:?}, got \
             {line_count}"
        );
    }
}

#[test]
fn helicopter_has_ear_lines() {
    for aff in ShapeAffiliation::ALL {
        let cmds = describe_symbol(NtdsShapeClass::Helicopter, aff, 0.0, 0.0, 100.0);
        // 4 ear lines: left ear, right ear, left rotor, right rotor
        let line_count = cmds.iter().filter(|c| matches!(c, ShapeCmd::Line { .. })).count();
        assert!(
            line_count >= 4,
            "expected at least 4 Line commands (ears + rotors) for Helicopter/{aff:?}, got \
             {line_count}"
        );
    }
}

#[test]
fn probably_destroyed_produces_two_lines() {
    let cmds = describe_bda_decoration(BdaDecoration::ProbablyDestroyed, 0.0, 0.0, 100.0);
    let line_count = cmds.iter().filter(|c| matches!(c, ShapeCmd::Line { .. })).count();
    assert_eq!(line_count, 2, "ProbablyDestroyed should produce exactly 2 lines (X)");
}

#[test]
fn uncertain_produces_one_line() {
    let cmds = describe_bda_decoration(BdaDecoration::Uncertain, 0.0, 0.0, 100.0);
    let line_count = cmds.iter().filter(|c| matches!(c, ShapeCmd::Line { .. })).count();
    assert_eq!(line_count, 1, "Uncertain should produce exactly 1 line (slash)");
}

#[test]
fn half_square_centers_are_symmetric() {
    let cy = 0.0;
    let r = 100.0;
    let upper = base_shape_center_y(NtdsShapeClass::Air, ShapeAffiliation::Unknown, cy, r);
    let lower = base_shape_center_y(NtdsShapeClass::Subsurface, ShapeAffiliation::Unknown, cy, r);
    // Both should be equidistant from cy, on opposite sides
    assert!(
        (upper + lower).abs() < f32::EPSILON,
        "half-square centers not symmetric: {upper} vs {lower}"
    );
    // And offset from cy by half_extent / 2
    let expected_offset = r / 2.0;
    assert!((upper - expected_offset).abs() < f32::EPSILON);
}

#[test]
fn base_shape_center_y_all_combinations() {
    for class in NtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            // Should not panic for any combination
            let _ = base_shape_center_y(class, aff, 512.0, 400.0);
        }
    }
}

#[test]
fn full_shape_centers_at_cy() {
    let cy = 512.0;
    let r = 400.0;
    for class in [NtdsShapeClass::Surface, NtdsShapeClass::Land, NtdsShapeClass::CommandShip] {
        for aff in ShapeAffiliation::ALL {
            let center = base_shape_center_y(class, aff, cy, r);
            assert!(
                (center - cy).abs() < f32::EPSILON,
                "full shape center {center} != cy {cy} for {class:?}/{aff:?}"
            );
        }
    }
}

#[test]
fn unknown_air_and_sub_share_center_offset() {
    let cy = 512.0;
    let r = 400.0;
    let air = base_shape_center_y(NtdsShapeClass::Air, ShapeAffiliation::Unknown, cy, r);
    let sub = base_shape_center_y(NtdsShapeClass::Subsurface, ShapeAffiliation::Unknown, cy, r);
    let surface = base_shape_center_y(NtdsShapeClass::Surface, ShapeAffiliation::Unknown, cy, r);
    // Air and sub centers equidistant from surface center.
    // Use a relative tolerance — f32 cancellation at magnitude 512 loses ~1 ULP.
    let air_offset = air - surface;
    let sub_offset = surface - sub;
    let tol = r * f32::EPSILON * 4.0;
    assert!(
        (air_offset - sub_offset).abs() < tol,
        "air offset {air_offset} != sub offset {sub_offset} (tol {tol})"
    );
}

#[test]
fn friendly_semicircle_center_offset() {
    let cy = 0.0;
    let r = 100.0;
    let upper = base_shape_center_y(NtdsShapeClass::Air, ShapeAffiliation::Friend, cy, r);
    assert!((upper - r / 2.0).abs() < f32::EPSILON);
}

#[test]
fn heading_vector_returns_single_line() {
    let cmds = describe_heading_vector(0.0, 0.0, 100.0, 0.0);
    assert_eq!(cmds.len(), 1, "expected exactly 1 ShapeCmd");
    assert!(matches!(cmds[0], ShapeCmd::Line { .. }), "expected a Line command");
}

#[test]
fn heading_vector_endpoint_distance() {
    let cx = 50.0;
    let cy = 50.0;
    let r = 100.0;
    for angle in [0.0_f32, FRAC_PI_2, PI, -FRAC_PI_2, PI / 4.0] {
        let cmds = describe_heading_vector(cx, cy, r, angle);
        let ShapeCmd::Line { x1, y1, x2, y2 } = cmds[0] else {
            panic!("expected Line");
        };
        assert!(
            (x1 - cx).abs() < f32::EPSILON && (y1 - cy).abs() < f32::EPSILON,
            "line start should be at center ({cx}, {cy}), got ({x1}, {y1})"
        );
        let dx = x2 - cx;
        let dy = y2 - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let expected = HEADING_VECTOR_LENGTH_RATIO * r;
        assert!(
            (dist - expected).abs() < 0.01,
            "endpoint distance {dist} != expected {expected} for angle {angle}"
        );
    }
}

#[test]
fn heading_vector_angle_zero_points_right() {
    let cmds = describe_heading_vector(0.0, 0.0, 100.0, 0.0);
    let ShapeCmd::Line { x2, y2, .. } = cmds[0] else {
        panic!("expected Line");
    };
    // angle=0 → +X direction → x2 > 0, y2 ≈ 0
    assert!(x2 > 0.0, "expected positive x2 for angle=0");
    assert!(y2.abs() < 0.01, "expected y2 ≈ 0 for angle=0");
}

#[test]
fn perceived_center_full_shapes_at_cy() {
    let cy = 512.0;
    let r = 400.0;
    for class in [NtdsShapeClass::Surface, NtdsShapeClass::Land, NtdsShapeClass::CommandShip] {
        for aff in ShapeAffiliation::ALL {
            let center = perceived_center_y(class, aff, cy, r);
            assert!(
                (center - cy).abs() < f32::EPSILON,
                "full shape perceived center {center} != cy {cy} for {class:?}/{aff:?}"
            );
        }
    }
}

#[test]
fn perceived_center_upper_lower_symmetric() {
    let cy = 512.0;
    let r = 400.0;
    for aff in ShapeAffiliation::ALL {
        let air = perceived_center_y(NtdsShapeClass::Air, aff, cy, r);
        let sub = perceived_center_y(NtdsShapeClass::Subsurface, aff, cy, r);
        let air_offset = air - cy;
        let sub_offset = cy - sub;
        let tol = r * f32::EPSILON * 4.0;
        assert!(
            (air_offset - sub_offset).abs() < tol,
            "air offset {air_offset} != sub offset {sub_offset} for {aff:?}"
        );
    }
}

#[test]
fn perceived_center_smaller_than_bounding_box_center() {
    let cy = 0.0;
    let r = 400.0;
    for aff in ShapeAffiliation::ALL {
        let perceived = perceived_center_y(NtdsShapeClass::Air, aff, cy, r);
        let bbox = base_shape_center_y(NtdsShapeClass::Air, aff, cy, r);
        assert!(
            perceived < bbox,
            "perceived {perceived} should be less than bbox {bbox} for Air/{aff:?}"
        );
        assert!(perceived > cy, "perceived {perceived} should be above cy {cy} for Air/{aff:?}");
    }
}

#[test]
fn group_all_combinations_produce_shapes() {
    for class in GroupNtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            let cmds = describe_group_symbol(class, aff, 0.0, 0.0, 100.0);
            assert!(!cmds.is_empty(), "no shapes for group {class:?} / {aff:?}");
        }
    }
}

#[test]
fn group_symbol_has_doubled_outline() {
    // A friendly surface group should have two Circle commands (inner + outer)
    let cmds = describe_group_symbol(
        GroupNtdsShapeClass::Surface,
        ShapeAffiliation::Friend,
        0.0,
        0.0,
        100.0,
    );
    let circle_count = cmds.iter().filter(|c| matches!(c, ShapeCmd::Circle { .. })).count();
    assert_eq!(
        circle_count, 2,
        "expected 2 circles (inner + outer) for friendly surface group, got {circle_count}"
    );
}

#[test]
fn group_symbol_outer_radius_is_larger() {
    let cmds = describe_group_symbol(
        GroupNtdsShapeClass::Surface,
        ShapeAffiliation::Friend,
        0.0,
        0.0,
        100.0,
    );
    let radii: Vec<f32> = cmds
        .iter()
        .filter_map(|c| match c {
            ShapeCmd::Circle { r, .. } => Some(*r),
            ShapeCmd::Arc { .. }
            | ShapeCmd::Line { .. }
            | ShapeCmd::ClosedLineStrip(_)
            | ShapeCmd::LineStrip(_)
            | ShapeCmd::FilledCircle { .. } => None,
        })
        .collect();
    assert_eq!(radii.len(), 2);
    assert!(radii[1] > radii[0], "outer radius {} should be > inner radius {}", radii[1], radii[0]);
    let expected_outer = 100.0 * GROUP_OUTER_RADIUS_RATIO;
    assert!(
        (radii[1] - expected_outer).abs() < 0.01,
        "outer radius {} != expected {expected_outer}",
        radii[1]
    );
}

#[test]
fn group_symbol_has_dot() {
    for class in GroupNtdsShapeClass::ALL {
        for aff in ShapeAffiliation::ALL {
            let cmds = describe_group_symbol(class, aff, 0.0, 0.0, 100.0);
            assert!(
                cmds.iter().any(|c| matches!(c, ShapeCmd::FilledCircle { .. })),
                "expected a center dot for group {class:?} / {aff:?}"
            );
        }
    }
}

#[test]
fn group_to_shape_class_mapping() {
    assert_eq!(GroupNtdsShapeClass::Air.to_shape_class(), NtdsShapeClass::Air);
    assert_eq!(GroupNtdsShapeClass::Surface.to_shape_class(), NtdsShapeClass::Surface);
    assert_eq!(GroupNtdsShapeClass::Submarine.to_shape_class(), NtdsShapeClass::Subsurface);
}

#[test]
fn neutral_surface_has_cross_and_dot() {
    let cmds = describe_symbol(NtdsShapeClass::Surface, ShapeAffiliation::Neutral, 0.0, 0.0, 100.0);
    let has_cross =
        cmds.iter().any(|c| matches!(c, ShapeCmd::ClosedLineStrip(pts) if pts.len() == 13));
    assert!(has_cross, "expected a 13-point ClosedLineStrip (cross)");
    assert!(
        cmds.iter().any(|c| matches!(c, ShapeCmd::FilledCircle { .. })),
        "expected a FilledCircle (dot) command"
    );
}

#[test]
fn neutral_air_has_half_cross() {
    let cmds = describe_symbol(NtdsShapeClass::Air, ShapeAffiliation::Neutral, 0.0, 0.0, 100.0);
    let has_half_cross =
        cmds.iter().any(|c| matches!(c, ShapeCmd::LineStrip(pts) if pts.len() == 8));
    assert!(has_half_cross, "expected an 8-point LineStrip (half-cross)");
}
