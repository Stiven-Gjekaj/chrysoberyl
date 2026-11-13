# Contributing to Chrysoberyl

Thank you for your interest in Chrysoberyl, a structural diff for everything
that is not text. Each kind of contribution is welcome: a bug report, a fix
to the documentation, a new fixture, or a new input format.

## Ways to contribute

- Open an issue to report a bug or to ask for a feature.
- Improve the documentation in `docs/` or the README.
- Run the GPU equivalence test on an Intel, an Nvidia or an AMD GPU, and
  report the result. This settles an open claim. See "Open claims" in
  [docs/DESIGN.md](docs/DESIGN.md).
- Add an input format as a new adapter crate.

Before you start a large change, open an issue. Agree on the approach before
you write the pull request.

## Development setup

You need `rustup`. The file `rust-toolchain.toml` pins the toolchain, and
`rustup` installs it on the first build.

    git clone https://github.com/stiven-gjekaj/chrysoberyl
    cd chrysoberyl
    git config core.hooksPath .githooks
    cargo build -p chrys-cli

Compare a pair:

    cargo run -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png

## The rules that a change must keep

Read [docs/DESIGN.md](docs/DESIGN.md) first. These five rules do not change:

1. No pixel that enters a comparison comes from a GPU.
2. The engine compares near-identical pairs only.
3. One engine, one meaning of "changed". A format never changes the verdict.
4. The mesh rig is fixed.
5. A rule file is data. No rule is computed.

## Where a change lives

| Change | Files |
| ------ | ----- |
| A new input format | A new crate `crates/chrys-source-<name>`, a sniff and a branch in `crates/chrys-cli/src/main.rs`, a Cargo feature that is off by default, a fixture in `tests/golden/formats/<name>`, a line in `.github/workflows/determinism.yml`, a boundary guard in `crates/chrys-cli/tests` |
| The engine | `crates/chrys-core`. A format never needs a change here |
| A rule kind | `crates/chrys-rule`, `examples/rules/example.toml` |
| The baseline store | `crates/chrys-baseline` |
| The window | `crates/chrys-view` |
| The GPU mirror | `crates/chrys-gpu`. It never writes a verdict |

### A new format must prove its determinism

A new format is not done when its tests pass on one machine. Add its
fixture to the digest report in `.github/workflows/determinism.yml`. The six
runners must agree byte for byte. If they do not agree, write the finding in
`docs/DESIGN.md` and keep the format out. Do not weaken what is compared.

## Before you open a pull request

Run these commands, as CI does:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets --features chrys-cli/pdf,chrys-cli/view,chrys-cli/video,chrys-cli/mesh -- -D warnings
    cargo test --workspace --features chrys-cli/pdf,chrys-cli/view,chrys-cli/video,chrys-cli/mesh

On a machine with a GPU, also run:

    cargo test -p chrys-gpu -- --ignored

- Build the state that a test needs inside the test. Do not read it from a
  file that an author edits.
- A guard must be able to fail. Prove a new guard by planting its defect.
- A fixture comes from a script in `scripts/`, so a person can make it again.

## Writing

Write each text in ASD-STE100 Simplified Technical English: the code, the
comments, the documentation, the commit messages and the pull requests.
Use short sentences, the active voice and the present tense. Do not use an
em-dash or an emoji. [AGENTS.md](AGENTS.md) has the full rules.

## Commits and pull requests

- One commit does one change.
- The code and its tests go in one commit. The documentation goes in its own.
- The subject line is in the present tense and tells what the change does.
- Add your change to `CHANGELOG.md` under `## Unreleased`.
- In the pull request, say what changed, why, and how you tested it.

## Security issues

Do not open a public issue for a security problem. See
[SECURITY.md](SECURITY.md).

## Code of conduct

When you take part in this project, you agree to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).
