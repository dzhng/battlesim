"""Run only publication consumers at S0's admitted empty-Battle extents."""
import json
import os
import pathlib
import subprocess

root = pathlib.Path.cwd()
out = root / "specs/city-maps/assets/fog-delivery/native"
out.mkdir(exist_ok=True)
target = pathlib.Path(os.environ["CARGO_TARGET_DIR"])
ceiling = 4 * 1024**3
for side, baseline in [(12000, 257971283), (15000, 403209279), (18000, 579706255)]:
    cells = ((side + 7) // 8)**2
    words = (cells + 31) // 32
    # Prior words, changed indices, record capacity slack, two credited snapshots.
    additional = words * (4 + 8 + 8) + 2 * words * 8
    assert baseline + additional < ceiling
    result = subprocess.run(
        ["/usr/bin/time", "-l", str(target / "release/examples/sa4_allocation"), str(side), "battle"],
        capture_output=True, text=True, check=True,
    )
    (out / f"{side}.jsonl").write_text(result.stdout)
    (out / f"{side}.time.txt").write_text(result.stderr)
    rows = [json.loads(line) for line in result.stdout.splitlines()]
    assert max(row.get("stage_peak_bytes", 0) for row in rows) < ceiling
    print(json.dumps({"side_m": side, "admitted_peak_upper_bytes": baseline + additional, **rows[-1]}))
