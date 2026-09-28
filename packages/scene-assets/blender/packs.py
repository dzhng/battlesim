"""The third-party packs the infantry scripts read, pinned by the reuse manifest.

The packs are not redistributed in this repo. They live in a local cache
(`$BATTLEGAME_PACKS`, default `~/.cache/battlegame/packs`), and every file a
script reads is checked against its `third_party` entry in
`reuse-manifest.json` first: a re-upload changes the
hash and stops the build instead of silently changing the art.

    python3 packages/scene-assets/blender/packs.py fetch    # download, unzip, verify

Both packs are Quaternius's free Standard tier, CC0 1.0 (accepted by the user,
2026-09-25), fetched through itch.io's "name your own price, $0" flow.
"""

import hashlib
import json
import os
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../.."))
MANIFEST = os.path.join(REPO, "reuse-manifest.json")
CACHE = os.environ.get("BATTLEGAME_PACKS", os.path.expanduser("~/.cache/battlegame/packs"))

# Pack zips: itch page and upload name. Their hashes, and those of the files
# inside them, are the manifest's `third_party` entries (path `packs/...`).
PACKS = {
    "Universal Animation Library[Standard].zip": "https://quaternius.itch.io/universal-animation-library",
    "Universal Base Characters[Standard].zip": "https://quaternius.itch.io/universal-base-characters",
}
UAL = "Universal Animation Library[Standard]/Unreal-Godot/UAL1_Standard.glb"
UBC = "Universal Base Characters[Standard]/Base Characters/Godot - UE/Superhero_Male_FullBody.gltf"
UBC_BIN = "Universal Base Characters[Standard]/Base Characters/Godot - UE/Superhero_Male_FullBody.bin"


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def pinned(rel):
    for e in json.load(open(MANIFEST))["third_party"]:
        if e["path"] == f"packs/{rel}":
            return e["sha256"]
    raise SystemExit(f"packs/{rel} has no third_party entry in {MANIFEST}")


def path(rel):
    """The cached file, verified against its manifest hash."""
    p = os.path.join(CACHE, rel)
    if not os.path.exists(p):
        raise SystemExit(f"{p} is missing; run: python3 packages/scene-assets/blender/packs.py fetch")
    got, want = sha256(p), pinned(rel)
    if got != want:
        raise SystemExit(f"{p}: sha256 {got}, the manifest pins {want}")
    return p


def _upsert(rel_path, make):
    """Rewrite the manifest row for `rel_path` as `make(old row or None)`, under a
    lock (sources export in parallel)."""
    import fcntl
    import tempfile

    lock = open(os.path.join(tempfile.gettempdir(), "battlegame-manifest.lock"), "w")
    fcntl.flock(lock, fcntl.LOCK_EX)
    manifest = json.load(open(MANIFEST))
    rows = manifest["third_party"]
    at = next((i for i, e in enumerate(rows) if e["path"] == rel_path), None)
    entry = make(None if at is None else rows[at])
    if at is None:
        rows.append(entry)
    else:
        rows[at] = entry
    with open(MANIFEST, "w") as f:
        f.write(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    fcntl.flock(lock, fcntl.LOCK_UN)
    lock.close()
    print("MANIFEST", rel_path, entry["sha256"])


def record_source(rel_path, script, inputs):
    """Upsert the manifest's `project-owned` entry for an exported appearance source:
    its hash, the script that wrote it and the pinned packs it was built from."""
    _upsert(rel_path, lambda _old: {
        "path": rel_path,
        "sha256": sha256(os.path.join(REPO, rel_path)),
        "licence": "project-owned",
        "covers": f"exported by packages/scene-assets/blender/{script}",
        "derived_from": [f"packs/{rel}" for rel in inputs],
        "accepted_by": "project: authored in this repo",
    })


def record_hash(path, covers):
    """Upsert a from-scratch source's `project-owned` entry with its new hash; an
    existing row keeps its acceptance."""
    rel_path = os.path.relpath(os.path.abspath(path), REPO)
    if rel_path.startswith(".."):
        return  # a scratch export outside the repo has no manifest row
    _upsert(rel_path, lambda old: {
        "path": rel_path,
        "sha256": sha256(os.path.join(REPO, rel_path)),
        "licence": "project-owned",
        "covers": (old or {}).get("covers", covers),
        "accepted_by": (old or {}).get("accepted_by", "project: authored in this repository"),
    })


def fetch():
    import http.cookiejar
    import re
    import urllib.parse
    import urllib.request
    import zipfile

    os.makedirs(CACHE, exist_ok=True)
    for zip_name, page in PACKS.items():
        zpath = os.path.join(CACHE, zip_name)
        if not (os.path.exists(zpath) and sha256(zpath) == pinned(zip_name)):
            jar = urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar())
            op = urllib.request.build_opener(jar)
            op.addheaders = [("User-Agent", "Mozilla/5.0"), ("X-Requested-With", "XMLHttpRequest")]
            csrf = lambda html: re.search(r'name="csrf_token" value="([^"]+)"', html).group(1)
            token = csrf(op.open(page).read().decode())
            form = urllib.parse.urlencode({"csrf_token": token}).encode()
            listing = op.open(json.loads(op.open(page + "/download_url", form).read())["url"]).read().decode()
            uploads = dict(zip(re.findall(r'title="([^"]+)" class="name"', listing),
                               re.findall(r'data-upload_id="(\d+)"', listing)))
            if zip_name not in uploads:
                raise SystemExit(f"{page} no longer offers {zip_name} (it offers {sorted(uploads)})")
            query = urllib.parse.urlencode({"source": "game_download"})
            url = json.loads(op.open(f"{page}/file/{uploads[zip_name]}?{query}", form).read())["url"]
            urllib.request.urlretrieve(url, zpath)
        if sha256(zpath) != pinned(zip_name):
            raise SystemExit(f"{zpath}: sha256 {sha256(zpath)}, the manifest pins {pinned(zip_name)}")
        with zipfile.ZipFile(zpath) as z:
            z.extractall(CACHE)
    for rel in (UAL, UBC, UBC_BIN):
        path(rel)
    print(f"packs verified in {CACHE}")


if __name__ == "__main__" and sys.argv[1:] == ["fetch"]:
    fetch()
