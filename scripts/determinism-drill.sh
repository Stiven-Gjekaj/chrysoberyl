#!/bin/sh
# A guard that nobody has seen fail is a belief, not a guard. This script
# plants one known defect per determinism guard this phase ships, runs
# the guard that should catch it, and checks two things, not one: that
# the guard went red, and that its message named the thing that was
# planted. A guard that goes red for the wrong reason is not better than
# a guard that stays green.
#
# Every mutation happens inside a temporary git worktree, created from
# HEAD and removed on every exit path, including failure. The real
# working tree is never edited.
#
# Exit 0 when all three drills behaved as expected. Exit non-zero when
# any drill's guard stayed green, or went red for a reason other than the
# defect this script planted.

set -u

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

if [ -n "$(git status --porcelain)" ]; then
    echo "determinism-drill: refusing to run: the working tree is not clean." >&2
    echo "determinism-drill: this script creates a temporary git worktree" >&2
    echo "determinism-drill: from HEAD, and would otherwise carry your" >&2
    echo "determinism-drill: uncommitted work into it. Commit or stash your" >&2
    echo "determinism-drill: changes, then re-run." >&2
    exit 1
fi

drill_worktree=$(mktemp -d)

cleanup() {
    cd "$repo_root"
    git worktree remove --force "$drill_worktree" >/dev/null 2>&1
    git worktree prune >/dev/null 2>&1
    rm -rf "$drill_worktree"
}
trap cleanup EXIT INT TERM

if ! git worktree add --detach --quiet "$drill_worktree" HEAD >/dev/null 2>&1; then
    echo "determinism-drill: could not create a temporary worktree from HEAD" >&2
    exit 1
fi

drill_pass_count=0
drill_fail_count=0

drill_ok() {
    echo "drill ok: $1"
    drill_pass_count=$((drill_pass_count + 1))
}

drill_failed() {
    echo "DRILL FAILED: $1"
    drill_fail_count=$((drill_fail_count + 1))
}

window_file="$drill_worktree/crates/chrys-core/src/register/window.rs"
manifest_file="$drill_worktree/crates/chrys-core/Cargo.toml"

# --- Drill one: the transcendental drill --------------------------------
# Replace the pure-Rust cosine call in the Hann window with the standard
# library's own method on the same value, and confirm the transcendental
# guard goes red and names window.rs.
sed 's/let cosine = libm::cos(angle);/let cosine = angle.cos();/' \
    "$window_file" >"$window_file.tmp" && mv "$window_file.tmp" "$window_file"

drill_one_output=$(cd "$drill_worktree" && cargo test -p chrys-core --test determinism \
    the_comparison_path_calls_no_forbidden_transcendental 2>&1)
drill_one_status=$?

if [ "$drill_one_status" -ne 0 ] && printf '%s' "$drill_one_output" | grep -q 'window\.rs'; then
    drill_ok "transcendental drill: the guard went red and named window.rs"
else
    drill_failed "transcendental drill: expected the guard to go red and name window.rs (exit=$drill_one_status)"
    printf '%s\n' "$drill_one_output"
fi

git -C "$drill_worktree" checkout -- crates/chrys-core/src/register/window.rs

# --- Drill two: the planner drill ----------------------------------------
# Replace the scalar transform planner with the auto-dispatching one, and
# confirm the cross-architecture digest script goes red and names a
# differing digest. This is the sharpest pitfall in the phase: two
# architectures on one machine take different arithmetic through the
# same call when the planner is left free to choose.
sed 's/use rustfft::{Fft, FftDirection, FftPlannerScalar};/use rustfft::{Fft, FftDirection, FftPlanner, FftPlannerScalar};/' \
    "$window_file" >"$window_file.tmp" && mv "$window_file.tmp" "$window_file"
sed 's/let mut planner = FftPlannerScalar::new();/let mut planner = FftPlanner::new();/' \
    "$window_file" >"$window_file.tmp" && mv "$window_file.tmp" "$window_file"

drill_two_output=$(cd "$drill_worktree" && sh scripts/cross-arch-hash.sh 2>&1)
drill_two_status=$?

if [ "$drill_two_status" -eq 2 ]; then
    drill_failed "planner drill: the cross-architecture drill is unavailable on this host, so it is not a passing drill"
    printf '%s\n' "$drill_two_output"
elif [ "$drill_two_status" -eq 1 ] && printf '%s' "$drill_two_output" | grep -q 'differ'; then
    drill_ok "planner drill: the cross-architecture digest script went red and named a differing digest"
else
    drill_failed "planner drill: expected the cross-architecture script to exit 1 and name a differing digest (exit=$drill_two_status)"
    printf '%s\n' "$drill_two_output"
fi

git -C "$drill_worktree" checkout -- crates/chrys-core/src/register/window.rs

# --- Drill three: the GPU drill -------------------------------------------
# Add a graphics crate (glow, an OpenGL binding, already on the
# dependency guard's own deny-list) to the engine manifest, and confirm
# the dependency guard goes red and names it.
sed 's/^\[dependencies\]$/[dependencies]\nglow = "0.17.0"/' \
    "$manifest_file" >"$manifest_file.tmp" && mv "$manifest_file.tmp" "$manifest_file"

drill_three_output=$(cd "$drill_worktree" && cargo test -p chrys-core --test determinism \
    no_gpu_or_format_crate_enters_chrys_cores_dependency_graph 2>&1)
drill_three_status=$?

if [ "$drill_three_status" -ne 0 ] && printf '%s' "$drill_three_output" | grep -q 'glow'; then
    drill_ok "GPU drill: the dependency guard went red and named glow"
else
    drill_failed "GPU drill: expected the dependency guard to go red and name glow (exit=$drill_three_status)"
    printf '%s\n' "$drill_three_output"
fi

git -C "$drill_worktree" checkout -- crates/chrys-core/Cargo.toml
git -C "$drill_worktree" checkout -- Cargo.lock

echo
echo "determinism-drill: $drill_pass_count of 3 drills behaved as expected"

if [ "$drill_fail_count" -gt 0 ]; then
    exit 1
fi

exit 0
