//! The rasterizer calls no platform transcendental function.
//!
//! The standard library says that the precision of `sin`, `cos`, `powf`,
//! `exp`, `ln` and the others changes between platforms. A view matrix or
//! a shade that used one of them could change between machines. `sqrt`
//! and `round` are not on the list: IEEE 754 makes each of them exact.

use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "exp2", "ln", "log", "log2",
    "log10", "powf", "powi", "cbrt", "hypot", "sinh", "cosh", "tanh", "sin_cos", "mul_add",
];

/// Remove each line comment, then return the lines that call `.name(`.
fn calls(source: &str, name: &str) -> Vec<usize> {
    let pattern = format!(".{name}(");
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.split("//").next().unwrap_or("").contains(&pattern))
        .map(|(index, _)| index + 1)
        .collect()
}

#[test]
fn the_matcher_finds_a_call_and_skips_a_comment() {
    // Without this test, a matcher with a fault passes the guard below for
    // the wrong reason.
    let source = "let a = x.sin();\n// y.cos() in a comment\nlet b = z.sqrt();\n";
    assert_eq!(calls(source, "sin"), vec![1]);
    assert!(calls(source, "cos").is_empty());
}

#[test]
fn the_mesh_source_calls_no_transcendental() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<_> = std::fs::read_dir(&src)
        .expect("read src")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    assert!(files.len() >= 4, "{files:?}");
    let mut hits = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file).expect("read");
        for name in FORBIDDEN {
            for line in calls(&source, name) {
                hits.push(format!("{}:{line} calls {name}", file.display()));
            }
        }
    }
    assert!(hits.is_empty(), "{hits:#?}");
}
