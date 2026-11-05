#!/usr/bin/env bash
#
# Regenerates the screenshots of `nf aerocloud v7 batch` used in the docs
# (`docs/content/aerocloud/cli/batch`), by driving the TUI with vhs in a 134x44 terminal.
#
# The examples in `examples/aerocloud/v7/batch` get submitted for real, unless `--dry-run`
# is passed: the ids of the simulations created are printed at the end, so they can be
# deleted afterwards.
#
# Run it from the dev shell (`nix develop`, or direnv), with a valid AeroCloud token
# (e.g. `NF_AEROCLOUD_AUTH_TOKEN`).
#
# Usage: scripts/batch-screenshots.sh --project NAME [--org ORG_ID] [--out DIR] [--dry-run]

set -euo pipefail

usage() {
    grep '^# Usage:' "$0" | sed 's/^# //'
    exit "${1:-0}"
}

repo=$(cd "$(dirname "$0")/.." && pwd)
project=""
org=""
out="$repo"
dry_run=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --project) project="$2"; shift 2 ;;
        --org) org="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --dry-run) dry_run=true; shift ;;
        -h | --help) usage ;;
        *) echo "unknown argument: $1" >&2; usage 1 ;;
    esac
done

[[ -n "$project" ]] || { echo "--project is required" >&2; usage 1; }

mkdir -p "$out"
out=$(cd "$out" && pwd)

# Terminal geometry: JetBrains Mono at this size fits exactly 134x44 in this window.
readonly cols=134 rows=44 font_size=20 width=1642 height=1158

examples="$repo/examples/aerocloud/v7"
work=$(mktemp -d)

log() { echo "==> $*" >&2; }

log "building nf"
cargo build --quiet --manifest-path "$repo/Cargo.toml"
nf="$repo/target/debug/nf"
nf_args=(-s aerocloud v7)
[[ -n "$org" ]] && nf_args+=(--org "$org")

# --- Submission state -------------------------------------------------------------------

# Simulations already marked as sent would show up as such, so set their state aside and
# put it back on exit.
restore_state() {
    local dir name
    for dir in "$examples"/batch/*/; do
        name=$(basename "$dir")
        rm -f "$dir/submission_state.json"
        if [[ -f "$work/state/$name" ]]; then
            mv "$work/state/$name" "$dir/submission_state.json"
        fi
    done
    rm -rf "$work"
}

mkdir -p "$work/state"
for dir in "$examples"/batch/*/; do
    if [[ -f "$dir/submission_state.json" ]]; then
        mv "$dir/submission_state.json" "$work/state/$(basename "$dir")"
    fi
done
trap restore_state EXIT

# --- Tapes ------------------------------------------------------------------------------

run_vhs() {
    (cd "$work" && FONTCONFIG_FILE="$BATCH_SCREENSHOTS_FONTCONFIG" vhs "$1" >/dev/null 2>&1)
}

tape_header() {
    cat <<EOF
Output out.gif
Set Shell bash
Set FontFamily "JetBrains Mono"
Set FontSize $font_size
Set Width $width
Set Height $height
Set Padding 0
Set TypingSpeed 0
Set WaitTimeout 180s
EOF
}

log "checking terminal size"
{
    tape_header
    echo 'Type "stty size > size.txt"'
    echo 'Enter'
    echo 'Sleep 1s'
} >"$work/probe.tape"
run_vhs probe.tape
size=$(cat "$work/size.txt")
if [[ "$size" != "$rows $cols" ]]; then
    echo "terminal is ${size/ /x} (rows x cols) instead of ${rows}x${cols}, adjust width/height" >&2
    exit 1
fi

log "finding project \`$project\`"
index=$("$nf" -j "${nf_args[@]}" list-projects --status active |
    python3 -c '
import json, sys
names = [p["name"] for p in json.load(sys.stdin)]
print(names.index(sys.argv[1]) if sys.argv[1] in names else -1)
' "$project")
if [[ "$index" -lt 0 ]]; then
    echo "project \`$project\` not found among active projects" >&2
    exit 1
fi

# Go regexps matching names literally.
quote_re() { python3 -c 'import re, sys; print(re.escape(sys.argv[1]))' "$1"; }
project_re=$(quote_re "$project")
last_sim_re=$(quote_re "$(basename "$(find "$examples/batch" -mindepth 1 -maxdepth 1 -type d | sort | tail -1)")")

# vhs rejects file names starting with digits, so screenshots get renamed afterwards.
# NOTE: each screenshot is followed by a pause, as vhs saves it on a later frame and the
# next key press would otherwise end up in it.
{
    tape_header
    echo 'Hide'
    printf 'Type "cd %s && %s %s batch batch 2>/dev/null"\n' "$examples" "$nf" "${nf_args[*]}"
    echo 'Enter'
    echo 'Wait+Screen /Created at/'
    echo 'Show'

    for ((i = 0; i < index; i++)); do echo 'Down'; done
    echo "Wait+Screen />> $project_re/"
    echo 'Sleep 500ms'
    echo 'Screenshot shot1.png'
    echo 'Sleep 1s'

    echo 'Enter'
    echo 'Wait+Screen /Simulations \(/'
    # NOTE: give it time to redraw everything after the project picker closes.
    echo 'Sleep 2s'
    echo 'Screenshot shot2.png'
    echo 'Sleep 1s'

    echo 'Ctrl+O'
    echo 'Wait+Screen /Launching batch/'
    echo 'Sleep 500ms'
    echo 'Screenshot shot3.png'
    echo 'Sleep 1s'

    if ! $dry_run; then
        echo 'Type "y"'
        echo 'Wait+Screen /Uploading files/'
        echo 'Screenshot shot4.png'
        echo 'Sleep 1s'

        echo "Wait+Screen /$last_sim_re \\(sent\\)/"
        echo 'Sleep 2s'
        echo 'Screenshot shot5.png'
        echo 'Sleep 1s'
    fi
} >"$work/batch.tape"

if $dry_run; then
    log "capturing screenshots (dry run, nothing gets submitted)"
else
    log "capturing screenshots, submitting simulations to \`$project\`"
fi
run_vhs batch.tape

# --- Output -----------------------------------------------------------------------------

names=(
    01-batch-select-project
    02-batch-review-sims
    03-batch-submit-sims
    04-batch-submit-sims-progress
    05-batch-end
)

for i in "${!names[@]}"; do
    shot="$work/shot$((i + 1)).png"
    [[ -f "$shot" ]] || continue
    cwebp -quiet -lossless -z 9 "$shot" -o "$out/${names[$i]}.webp"
    log "wrote $out/${names[$i]}.webp"
done

if ! $dry_run; then
    log "simulations created, delete them with \`nf aerocloud v7${org:+ --org $org} delete-simulations <ids>\`:"
    for dir in "$examples"/batch/*/; do
        [[ -f "$dir/submission_state.json" ]] || continue
        python3 -c '
import json, sys
sent = json.load(open(sys.argv[1])).get("Sent")
if sent:
    print("  " + sent["id"] + "  " + sent["browser_url"])
' "$dir/submission_state.json"
    done
fi
