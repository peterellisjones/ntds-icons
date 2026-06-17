//! Font generation — only compiled with the `build` feature.
//!
//! Pulls the heavy deps (`kurbo` + `write-fonts`, plus `ttf2woff2` / `resvg`
//! in later modules) that the default, font-consuming build does not need.

mod artifacts;
mod font;
mod gallery;
mod specimen;
mod woff2;

pub use artifacts::{codepoints_json, readme_table};
pub use font::{FontLayout, FontNames, build_font};
pub use gallery::render_gallery;
pub use specimen::render_specimen;
pub use woff2::to_woff2;
