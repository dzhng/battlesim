"""The geometry lint over the roster: every appearance a roster script exports.

Each roster script that exports through `vehicle_export` runs as its own
export would, with `vehicle_export.CAPTURE` set: its `run` and `run_disabled`
calls record the variants they would build (a shared script's
`appearances`, or its family folder's) and build nothing. Each appearance is
then built alone and linted (`vehicle_lint`).

A script that fails or exports nothing, or an appearance that fails to
build, fails the walk. `--update-baseline` then writes nothing: a baseline
rebuilt from a partial walk would forget the known issues of whatever
failed. The command line is `vehicle_lint.py`'s.
"""
import re
import runpy
import sys
import time
from collections import Counter
from pathlib import Path

import vehicle_export
from vehicle_lint import ALLOWLIST_PATH, BASELINE_PATH, lint_current_scene
from vehicle_lint_core import (
    filter_by_allowlist,
    filter_by_baseline,
    format_issues,
    load_allowlist,
    load_baseline,
    save_baseline,
)

ROSTER = Path(__file__).resolve().parent / "roster"
EXPORTER = re.compile(r"^from vehicle_export import .*\brun(_disabled)?\b", re.M)


def exporters():
    """Every roster script that exports vehicles through `vehicle_export`."""
    return [p for p in sorted(ROSTER.glob("*.py")) if EXPORTER.search(p.read_text())]


def captured(script):
    """What `script` exports: (variant, scheme, build, looks) for each variant
    its `run` and `run_disabled` calls would build."""
    vehicle_export.CAPTURE = runs = []
    try:
        runpy.run_path(str(script), run_name="__main__")
    finally:
        vehicle_export.CAPTURE = None
    return [(variant, r["scheme"], r["build"], r["looks"]) for r in runs for variant in r["variants"]]


def appearances(failures, only=None):
    """Each (script name, variant, scheme, build, looks) the roster exports,
    once per variant id, or just the one `only` names. A script that fails
    or exports nothing is added to `failures`."""
    seen = set()
    for script in exporters():
        try:
            exports = captured(script)
            why = None if exports else "exports nothing"
        except (Exception, SystemExit) as e:
            exports, why = [], f"script failed: {e!r}"
        if why:
            failures.append((script.stem, why))
            print(f"FAIL {script.stem}: {why}", flush=True)
        for variant, scheme, build, looks in exports:
            vid = variant["id"]
            if vid in seen or only not in (None, vid):
                continue
            seen.add(vid)
            yield script.stem, variant, scheme, build, looks
            if only is not None:
                return


def walk(only=None):
    """Build and lint each appearance; returns (rows, failures), a row being
    (script name, variant, its `LintResult`)."""
    rows, failures = [], []
    for script, variant, scheme, build, looks in appearances(failures, only):
        start = time.monotonic()
        try:
            vehicle_export.build_variant(variant, scheme, build, looks)
        except (Exception, SystemExit) as e:
            failures.append((variant["id"], f"build failed: {e!r}"))
            print(f"FAIL {variant['id']} ({script}): build failed: {e!r}", flush=True)
            continue
        result = lint_current_scene(variant["id"])
        rows.append((script, variant, result))
        print(f"LINT {variant['id']} ({script}): {len(result.issues)} issue(s), "
              f"{time.monotonic() - start:.0f}s", flush=True)
    return rows, failures


def report(rows, failures, baseline, allowlist):
    """Print each appearance's issues by check and what is new against the
    baseline and allowlist; returns the number of new issues."""
    total_new, by_check = 0, Counter()
    print("\n=== Roster lint summary ===")
    for script, variant, result in rows:
        new = filter_by_allowlist(filter_by_baseline(result, baseline), allowlist).issues
        total_new += len(new)
        checks = Counter(i.check for i in result.issues)
        by_check.update(checks)
        counts = ", ".join(f"{c} {n}" for c, n in sorted(checks.items())) or "clean"
        print(f"{variant['name']} [{variant['id']}, {script}]: {counts}; {len(new)} new")
        if new:
            print(format_issues(new))
    print(f"\nAppearances linted: {len(rows)}")
    print("Issues by check: " + (", ".join(f"{c} {n}" for c, n in sorted(by_check.items())) or "none"))
    print(f"New issues: {total_new}")
    for what, why in failures:
        print(f"FAILED {what}: {why}")
    return total_new


def main(args):
    """`--roster`, `--update-baseline` or `--variant=<id>`; exits non-zero on
    any new issue or failure, and `--update-baseline` writes only after a
    complete walk."""
    only = next((a.split("=", 1)[1] for a in args if a.startswith("--variant=")), None)
    update = "--update-baseline" in args
    if not (only or update or "--roster" in args):
        print(__doc__)
        sys.exit(2)
    rows, failures = walk(only)
    if only is not None:
        # One appearance's verdict is its own: scripts that failed before it
        # was found were printed, and are the roster walk's to fail.
        failures = [f for f in failures if f[0] == only]
        if not rows and not failures:
            failures.append((only, "no roster script exports it"))
    allowlist = load_allowlist(ALLOWLIST_PATH)
    baseline = {} if update else load_baseline(BASELINE_PATH)
    new = report(rows, failures, baseline, allowlist)
    if update:
        if failures:
            print(f"\nREFUSED: {len(failures)} failure(s); the baseline is unchanged")
            sys.exit(1)
        remaining = (filter_by_allowlist(r, allowlist) for _, _, r in rows)
        kept = {r.vehicle_id: [i.to_dict() for i in r.issues] for r in remaining if r.issues}
        save_baseline(BASELINE_PATH, kept)
        print(f"\nWrote {BASELINE_PATH.name}: {sum(map(len, kept.values()))} issue(s) "
              f"on {len(kept)} appearance(s)")
        sys.exit(0)
    if failures or new:
        print(f"\nFAIL: {new} new issue(s), {len(failures)} failure(s)")
        sys.exit(1)
    print("\nPASS: no new issues")
