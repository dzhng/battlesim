#!/bin/sh
set -eu
GODOT_BIN=${1:?usage: binding_smoke.sh GODOT_EXECUTABLE SCENARIO_JSON}
SCENARIO=${2:?usage: binding_smoke.sh GODOT_EXECUTABLE SCENARIO_JSON}
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$GODOT_BIN" --headless --path "$ROOT" --scene res://binding_smoke.tscn --rendering-method gl_compatibility --audio-driver Dummy --quit-after 60 --no-header -- "$SCENARIO"
