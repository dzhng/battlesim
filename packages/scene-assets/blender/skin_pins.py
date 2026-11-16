"""Generated skins, paid for once and pinned: the one reader of `skins/*.json`.

A pin records a generated mesh's file, its sha256, the request that made it,
its inputs and terms (`skins/<family>.json`; the generation tool writes them).
An export wears a skin only through `verified`, which refuses anything but the
pinned bytes and records what was worn (`WORN`) for the export's receipt.
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
PINS = HERE / "skins"
# Every skin an export wore, repo path to sha256, for its receipt.
WORN = {}


def pins(family):
    """The pins of `family` (`skins/<family>.json`), or none."""
    path = PINS / f"{family}.json"
    return json.loads(path.read_text()) if path.exists() else []


def required(family, **key):
    """The pin of `family` whose fields equal `key`. A missing one is an error
    naming the key (its values joined, as the generation tool writes it) and
    the commands that make it."""
    pin = next((p for p in pins(family) if all(p.get(k) == v for k, v in key.items())), None)
    if pin is None:
        name = "_".join(str(v) for v in key.values())
        tool = "node packages/scene-assets/tools/skin-generate.mjs"
        raise ValueError(f"no {family} skin is pinned for {name}: make one with `{tool} image {family} {name}`, "
                         f"then `{tool} generate {family} {name}` and `{tool} pin {family} {name} <request_id>`")
    return pin


def verified(pin):
    """The absolute path of `pin`'s skin, its bytes checked against the pin and
    recorded in `WORN`."""
    path = pin["glb"]
    data = (REPO / path).read_bytes()
    if data.startswith(b"version https://git-lfs"):
        raise ValueError(f"{path} is an LFS pointer: pull it first (git lfs pull -I {path})")
    got = hashlib.sha256(data).hexdigest()
    if got != pin["sha256"]:
        raise ValueError(f"{path} has sha256 {got}, pinned {pin['sha256']}")
    WORN[path] = got
    return str(REPO / path)


def pinned(path):
    """The pin of the skin at `path` (repo-relative), from any family, verified."""
    pin = next((p for f in sorted(PINS.glob("*.json")) for p in json.loads(f.read_text()) if p["glb"] == path), None)
    if pin is None:
        raise ValueError(f"{path} is not pinned in skins/")
    verified(pin)
    return pin
