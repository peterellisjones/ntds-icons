//! NTDS tactical-symbol geometry and outline icon font.
//!
//! `ntds-icons` packages the Naval Tactical Data System symbol set as a
//! standalone, bevy-free, font-first crate:
//!
//! - [`shapes`] — pure-geometry [`shapes::ShapeCmd`] descriptions of every NTDS
//!   symbol (zero dependencies), for direct drawing (map gizmos, SVG export).
//!
//! The codepoint lookups, the pre-built font, and the font generator are added
//! in subsequent tasks.

pub mod shapes;
