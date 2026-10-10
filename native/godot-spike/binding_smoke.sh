#!/bin/sh
set -eu
exec python3 "$(dirname "$0")/binding_smoke.py" "$@"
