//! DET-05's compile-time half: `resvg` builds with only its `text`
//! feature, so the host-font-database reader it can otherwise compile in
//! is absent from this binary rather than merely uncalled.
//!
//! **Why this crate, and not `chrys-core`.** `chrys-cli` already depends
//! on every other crate this workspace ships, and already hosts
//! `unsafe_guard.rs` and `decode_limits_guard.rs` for the same reason: it
//! is the one crate that can read every other crate's own boundary
//! without editing the engine itself. `crates/chrys-core`'s own tree
//! object id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`, read at commit
//! `199477d`, and covering `crates/chrys-core/tests/` as well as its
//! `src/`) must not move this phase, so a guard cannot be added to
//! `crates/chrys-core/tests/determinism.rs` even where it would
//! otherwise belong.
//!
//! **Two independent layers.** A manifest line says what was asked for;
//! the resolved `cargo tree` graph says what was actually built. Feature
//! unification means a second crate elsewhere in this workspace could
//! turn a feature back on without the manifest line this test also reads
//! ever changing, so this test checks both, and neither layer skips when
//! its own command cannot run: a guard that skips when its own command
//! is missing is a guard that always passes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Run `cargo` (through the `CARGO` environment variable when it is set,
/// the same convention `crates/chrys-core/tests/determinism.rs` already
/// uses) with `args`, from the workspace root, and return its stdout as a
/// `String`. Panics, rather than returning an empty or partial result,
/// when the command cannot be run or exits non-zero: a guard whose own
/// data source failed silently is a guard that always passes.
fn run_cargo(args: &[&str]) -> String {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let invocation = Command::new(&cargo)
        .args(args)
        .current_dir(workspace_root())
        .output();

    let output = match invocation {
        Ok(output) => output,
        Err(err) => panic!(
            "this guard needs `cargo {}` to read the dependency graph, and it could not be run \
             ({err}). This guard fails rather than skips: a guard that skips when its own \
             command is missing is a guard that always passes.",
            args.join(" ")
        ),
    };

    assert!(
        output.status.success(),
        "`cargo {}` exited with {}, so this guard cannot read the graph it is meant to check. \
         Failing, not skipping: stderr was:\n{}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn only_the_text_feature_of_resvg_is_enabled() {
    // Layer 1, checked first because it is the cheaper of the two: the
    // resolved feature graph. Feature unification means a second crate
    // elsewhere in this workspace could turn `system-fonts` or
    // `memmap-fonts` back on without any manifest line ever changing;
    // `02-LEARNINGS.md` already records the rule this test follows:
    // verify with `cargo tree`, not with the declared list. Checked
    // first, and its own assertion messages name the exact feature that
    // came back, so a drill that restores resvg's default features fails
    // here, naming the feature, rather than on the manifest-text check
    // below, whose own message cannot name a feature at all.
    let tree = run_cargo(&["tree", "-e", "features", "-p", "chrys-source-svg"]);

    assert!(
        !tree.contains("system-fonts"),
        "the resolved feature graph of chrys-source-svg names the `system-fonts` feature, which \
         compiles the host-font-database reader into the binary (DET-05):\n{tree}"
    );
    assert!(
        !tree.contains("memmap-fonts"),
        "the resolved feature graph of chrys-source-svg names the `memmap-fonts` feature:\n{tree}"
    );
    assert!(
        tree.contains("\"text\""),
        "the resolved feature graph of chrys-source-svg does not name the `text` feature at \
         all, so this test read a graph this crate does not use rather than an absence of the \
         forbidden features:\n{tree}"
    );

    // Layer 2: the workspace manifest line itself asks for
    // `default-features = false` and the `text` feature. This mirrors,
    // line for line in shape, `crates/chrys-core/tests/determinism.rs`'s
    // own
    // `the_colour_crate_declares_no_default_features_and_the_pure_rust_maths_feature`,
    // which checks the same two properties for `palette`. A manifest
    // line says what was asked for; Layer 1 above says what was
    // actually built, which is why Layer 1 runs first.
    let manifest_path = workspace_root().join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", manifest_path.display()));

    let resvg_line = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("resvg "))
        .unwrap_or_else(|| {
            panic!(
                "{} has no `resvg` dependency entry to check",
                manifest_path.display()
            )
        });

    assert!(
        resvg_line.contains("default-features = false"),
        "{}: `{}` must declare `default-features = false`, so a future default feature cannot \
         reach the host font database behind this project's back",
        manifest_path.display(),
        resvg_line.trim()
    );
    assert!(
        resvg_line.contains("\"text\""),
        "{}: `{}` must enable the `text` feature, which this crate needs to render a `<text>` \
         element at all",
        manifest_path.display(),
        resvg_line.trim()
    );
}
