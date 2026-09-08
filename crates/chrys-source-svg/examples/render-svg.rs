//! Render an SVG file through `SvgSource`, and write the exact bytes that
//! decode produced as a PNG.
//!
//! A hash proves six runners agree on a pixel buffer; it cannot prove
//! they agree on a picture that shows the text a person authored, or
//! that a curve and a gradient actually rendered rather than silently
//! coming out blank. Both checks are needed and neither replaces the
//! other, so this example reaches for no second rendering path: it writes
//! the very buffer `SvgSource::load` returned, through the `png` crate,
//! taken here as a dev-dependency only. Any second path to produce the
//! picture would make it unfalsifiable evidence about the first.
//!
//! Usage: `cargo run -p chrys-source-svg --example render-svg -- <input.svg> <output.png>`

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_svg::SvgSource;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let input = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("usage: render-svg <input.svg> <output.png>"));
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("usage: render-svg <input.svg> <output.png>"));

    let source = SvgSource::new();
    let frame = source
        .load(&input)
        .unwrap_or_else(|error| panic!("loading {}: {error}", input.display()))
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{} decoded to zero frames", input.display()));

    let file = std::fs::File::create(&output)
        .unwrap_or_else(|error| panic!("creating {}: {error}", output.display()));
    let mut encoder = png::Encoder::new(file, frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .unwrap_or_else(|error| panic!("writing header for {}: {error}", output.display()));
    writer
        .write_image_data(&frame.pixels)
        .unwrap_or_else(|error| panic!("writing image data for {}: {error}", output.display()));

    println!(
        "wrote {} ({}x{}) from {}",
        output.display(),
        frame.width,
        frame.height,
        input.display()
    );
}
