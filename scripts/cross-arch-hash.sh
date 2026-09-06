#!/bin/sh
# What this measures, and why it exists.
#
# DET-02 claims the same input gives the same digest on two CPU
# architectures. The six-runner matrix in
# .github/workflows/determinism.yml is the measured proof of that claim,
# but it cannot run without a remote, and this repository is private
# until asked otherwise. This script is the evidence that can be produced
# on one machine today: it builds the command line binary for the host's
# own architecture and for a second architecture, runs both against the
# same committed pair, and asserts the two digest reports agree byte for
# byte.
#
# Exit codes:
#   0  the two architectures agree on all four digests
#   1  the two architectures disagree; both reports are printed
#   2  this host cannot run the drill at all (not a pass; the caller must
#      treat this the same as a failure, never as success)

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

pair_base="tests/golden/pair-01/base.png"
pair_candidate="tests/golden/pair-01/candidate.png"

host_os=$(uname -s)
host_arch=$(uname -m)

if [ "$host_os" != "Darwin" ] || [ "$host_arch" != "arm64" ]; then
    echo "cross-arch-hash: this host is $host_os/$host_arch, not Apple" >&2
    echo "cross-arch-hash: Silicon macOS; the local cross-architecture drill" >&2
    echo "cross-arch-hash: is not available here. This is not a pass." >&2
    exit 2
fi

native_target="aarch64-apple-darwin"
second_target="x86_64-apple-darwin"

echo "cross-arch-hash: ensuring $second_target is installed"
rustup target add "$second_target" >/dev/null

report_dir=$(mktemp -d)
trap 'rm -rf "$report_dir"' EXIT INT TERM

native_report="$report_dir/$native_target.txt"
second_report="$report_dir/$second_target.txt"

echo "cross-arch-hash: building for $native_target"
cargo build --release --target "$native_target" -p chrys-cli >&2

echo "cross-arch-hash: building for $second_target"
cargo build --release --target "$second_target" -p chrys-cli >&2

echo "cross-arch-hash: running the $native_target binary"
"target/$native_target/release/chrys" compare "$pair_base" "$pair_candidate" \
    --hash-only >"$native_report"

echo "cross-arch-hash: running the $second_target binary"
"target/$second_target/release/chrys" compare "$pair_base" "$pair_candidate" \
    --hash-only >"$second_report"

if diff -q "$native_report" "$second_report" >/dev/null; then
    echo "cross-arch-hash: $native_target and $second_target agree on all four digests"
    cat "$native_report"
    exit 0
fi

echo "cross-arch-hash: $native_target and $second_target DISAGREE" >&2
echo >&2
echo "== $native_target ==" >&2
cat "$native_report" >&2
echo >&2
echo "== $second_target ==" >&2
cat "$second_report" >&2
echo >&2

paste "$native_report" "$second_report" | while IFS="$(printf '\t')" read -r native_line second_line; do
    native_name=${native_line%% *}
    native_value=${native_line#* }
    second_value=${second_line#* }
    if [ "$native_value" != "$second_value" ]; then
        echo "cross-arch-hash: digest '$native_name' differs between $native_target and $second_target" >&2
    fi
done

exit 1
