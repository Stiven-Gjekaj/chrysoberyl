#!/bin/sh
# The animation wave (02-03) claims it changed no file under
# crates/chrys-core/. That claim is checked with one command:
#
#   git diff --name-only <first>^..<last> -- crates/chrys-core/
#
# Empty output is the pass condition. But an empty result is also what a
# wrong commit range, a typo in the path filter, or a silently failing
# command produces. A check whose only visible state is silence has not
# been proven able to fail, so its silence is a belief, not evidence.
#
# This script drills that check the same way scripts/determinism-drill.sh
# drills the phase 1 guards: plant a real defect inside a disposable git
# worktree, run the check over the planted commit's own range, and assert
# two things, not one: that the check produced output, and that the
# output names the exact file that was planted. Then run the same check
# over a real, clean range and assert the output is empty.
#
# Every mutation happens inside a temporary git worktree, created from
# HEAD and removed on every exit path, including failure. The real
# working tree is never edited.
#
# Usage: engine-boundary-drill.sh <first-commit> <last-commit>
#   <first-commit>  the first commit of the wave whose boundary is checked
#   <last-commit>   the last commit of the wave whose boundary is checked
#
# Exit 0 when both drills behaved as expected. Exit non-zero when either
# drill's check stayed silent when it should have spoken, spoke without
# naming the planted file, or spoke when it should have stayed silent.

set -u

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

if [ "$#" -ne 2 ]; then
    echo "usage: engine-boundary-drill.sh <first-commit> <last-commit>" >&2
    echo "  <first-commit>  the first commit of the wave whose boundary is checked" >&2
    echo "  <last-commit>   the last commit of the wave whose boundary is checked" >&2
    exit 1
fi

wave_first_commit="$1"
wave_last_commit="$2"

if [ -n "$(git status --porcelain)" ]; then
    echo "engine-boundary-drill: refusing to run: the working tree is not clean." >&2
    echo "engine-boundary-drill: this script creates a temporary git worktree" >&2
    echo "engine-boundary-drill: from HEAD, and would otherwise carry your" >&2
    echo "engine-boundary-drill: uncommitted work into it. Commit or stash your" >&2
    echo "engine-boundary-drill: changes, then re-run." >&2
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
    echo "engine-boundary-drill: could not create a temporary worktree from HEAD" >&2
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

boundary_check() {
    # boundary_check WORKTREE RANGE prints the path of every file the range
    # changed under crates/chrys-core/. Empty output is the pass condition.
    git -C "$1" diff --name-only "$2" -- crates/chrys-core/
}

# --- Drill one: the planted defect ---------------------------------------
# Append a harmless comment line to an engine source file and, in the same
# commit, a comment line to a file in the animation adapter crate, so the
# planted commit looks like a real wave commit that happens to have
# reached across the boundary. Run the check over that commit's own range
# and assert it names the planted engine file.
engine_file="crates/chrys-core/src/lib.rs"
adapter_file="crates/chrys-source-animation/src/lib.rs"

echo "// engine-boundary-drill: planted defect, reverted after this drill" \
    >>"$drill_worktree/$engine_file"
echo "// engine-boundary-drill: planted defect, reverted after this drill" \
    >>"$drill_worktree/$adapter_file"

git -C "$drill_worktree" add "$engine_file" "$adapter_file"
git -C "$drill_worktree" -c user.name="engine-boundary-drill" \
    -c user.email="engine-boundary-drill@localhost" \
    commit --quiet -m "engine-boundary-drill: planted cross-boundary defect"

planted_commit=$(git -C "$drill_worktree" rev-parse HEAD)
planted_parent=$(git -C "$drill_worktree" rev-parse HEAD^)

drill_one_output=$(boundary_check "$drill_worktree" "${planted_parent}..${planted_commit}")

echo "engine-boundary-drill: drill one: checked range ${planted_parent}..${planted_commit}"
echo "engine-boundary-drill: drill one: check output:"
if [ -n "$drill_one_output" ]; then
    printf '%s\n' "$drill_one_output"
else
    echo "(empty)"
fi

if [ -n "$drill_one_output" ] && printf '%s' "$drill_one_output" | grep -qx "$engine_file"; then
    drill_ok "planted-defect drill: the check went red and named $engine_file"
else
    drill_failed "planted-defect drill: expected the check to name $engine_file"
fi

# --- Drill two: the real, clean range -------------------------------------
# Run the same check over the real commit range of plan 02-03 and assert
# the output is empty, which is the pass condition this phase's headline
# claim rests on.
drill_two_output=$(boundary_check "$drill_worktree" "${wave_first_commit}^..${wave_last_commit}")

echo "engine-boundary-drill: drill two: checked range ${wave_first_commit}^..${wave_last_commit}"
echo "engine-boundary-drill: drill two: check output:"
if [ -n "$drill_two_output" ]; then
    printf '%s\n' "$drill_two_output"
else
    echo "(empty)"
fi

if [ -z "$drill_two_output" ]; then
    drill_ok "clean-range drill: the check stayed silent over plan 02-03's own range"
else
    drill_failed "clean-range drill: expected no output over plan 02-03's own range"
fi

echo
echo "engine-boundary-drill: $drill_pass_count of 2 drills behaved as expected"

if [ "$drill_fail_count" -gt 0 ]; then
    exit 1
fi

exit 0
