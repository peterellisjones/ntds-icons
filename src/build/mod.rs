//! Font generation — only compiled with the `build` feature.
//!
//! Pulls the heavy deps (`kurbo` + `write-fonts`, plus `ttf2woff2` / `resvg`
//! in later modules) that the default, font-consuming build does not need.

mod font;

pub use font::{FontLayout, FontNames, build_font};
