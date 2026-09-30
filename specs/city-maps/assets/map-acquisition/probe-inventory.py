"""Verify the immutable original source and registry receipt without building."""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
inventory = json.loads((HERE / "inventory.json").read_text())
revision = json.loads((HERE / "receipt.json").read_text())["baseline_revision"]


def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args])


for source in inventory["sources"]:
    raw = git("show", f'{revision}:{source["path"]}')
    assert len(raw) == source["bytes"], source["path"]
    assert hashlib.sha256(raw).hexdigest() == source["sha256"], source["path"]

registered = json.loads(git("show", f"{revision}:apps/battle-lab/src/fixtures.json"))
assert registered == inventory["registered"]
scenes = {
    Path(p).stem
    for p in git("ls-tree", "-r", "--name-only", revision, "web/scenes").decode().splitlines()
    if p.endswith(".mjs") and not Path(p).name.startswith("_")
}
ids = {r["id"] for r in registered}
assert ids == scenes, {"missing": sorted(ids - scenes), "orphan": sorted(scenes - ids)}
print(json.dumps({"revision": revision, "sources": len(inventory["sources"]),
                  "routes": len(ids), "scenes": len(scenes), "missing": [], "orphan": []}))
