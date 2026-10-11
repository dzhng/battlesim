#!/bin/sh
# Exports the project as a macOS release build into throwaway/native-release/
# and runs the parity run from it, off screen, with the arguments given:
#   native/godot/release.sh --run <dir> --scene <name> [any other parity run argument]
# The exported app is not in the repository, so it takes the repository root
# from BATTLEGAME_REPO, which this sets to this checkout. It packs the assets
# staging/ holds at export time; an editor run of a scene stages them. It
# builds the Rust extension first (binding/build.sh).
# Take the GPU lock round it, as round an editor run.
set -eu
project=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$project/../.." && pwd)
app="$repo/throwaway/native-release/Battle Game Native.app"
"$project/binding/build.sh" >&2
mkdir -p "$(dirname "$app")"
if ! log=$(godot --headless --path "$project" --export-release "macOS release" "$app" 2>&1); then
	printf '%s\n' "$log" | grep -A1 "ERROR" >&2
	exit 1
fi
BATTLEGAME_REPO="$repo" exec "$app/Contents/MacOS/Battle Game Native" --window-position 10000,10000 -- "$@"
