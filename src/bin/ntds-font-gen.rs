//! Generate every NTDS font artifact from the shared codepoint data, in one
//! pass, so nothing drifts: `ntds_icons.ttf`, `ntds_icons.woff2`,
//! `specimen.png`, `codepoints.json`, and the Pages gallery `index.html`.

use std::{fs, path::PathBuf};

use clap::Parser;
use ntds_icons::build::{
    FontLayout, build_font, codepoints_json, render_gallery, render_specimen, to_woff2,
};

#[derive(Parser)]
#[command(
    about = "Generate ntds_icons.ttf / .woff2 + specimen.png + codepoints.json from shared data"
)]
struct Args {
    /// Output directory for the generated artifacts.
    #[arg(long, default_value = "assets")]
    out_dir: PathBuf,
}

fn main() {
    let args = Args::parse();
    fs::create_dir_all(&args.out_dir).expect("create out dir");

    let ttf = build_font(&FontLayout::default());
    let woff2 = to_woff2(&ttf);
    let png = render_specimen(&ttf);
    let json = codepoints_json();
    let gallery = render_gallery("ntds_icons.woff2");

    let write = |name: &str, bytes: &[u8]| {
        let path = args.out_dir.join(name);
        fs::write(&path, bytes).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
    };

    write("ntds_icons.ttf", &ttf);
    write("ntds_icons.woff2", &woff2);
    write("specimen.png", &png);
    write("codepoints.json", json.as_bytes());
    write("index.html", gallery.as_bytes());
}
