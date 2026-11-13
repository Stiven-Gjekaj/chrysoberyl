//! Write the eight views of a mesh as PNG files, so a person can look at
//! them.
//!
//!     cargo run -p chrys-source-mesh --example render-views -- MESH OUT_DIR

use std::path::PathBuf;

use chrys_source_mesh::{MeshSource, rig};

fn main() {
    let mut args = std::env::args().skip(1);
    let mesh = PathBuf::from(args.next().expect("the mesh path"));
    let out = PathBuf::from(args.next().expect("the output directory"));
    std::fs::create_dir_all(&out).expect("make the output directory");
    let (frames, blind) = MeshSource::new().render(&mesh).expect("render");
    for (frame, view) in frames.iter().zip(rig::VIEWS.iter()) {
        let path = out.join(format!("{}.png", view.name.replace(' ', "-")));
        let file = std::fs::File::create(&path).expect("create the PNG");
        let mut encoder = png::Encoder::new(file, frame.width, frame.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(&frame.pixels).expect("PNG data");
        println!("{}", path.display());
    }
    println!("{blind}");
}
