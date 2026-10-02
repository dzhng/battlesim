"""The third-party packs the infantry scripts read, pinned in `packs.json`.

The packs are not redistributed in this repo. They live in a local cache
(`$BATTLEGAME_PACKS`, default `~/.cache/battlegame/packs`), and every file a
script reads is checked against its sha256 in `packs.json` first: a re-upload
changes the hash and stops the build instead of silently changing the art.

    python3 packages/scene-assets/blender/packs.py fetch    # download, unzip, verify

`packs.json` names each pack's zip, its itch.io page (fetched through the
"name your own price, $0" flow), its licence and the files read from it.
"""

import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.environ.get("BATTLEGAME_PACKS", os.path.expanduser("~/.cache/battlegame/packs"))

PACKS = json.load(open(os.path.join(HERE, "packs.json")))
# sha256 by path in the cache: each zip, and each file read from inside it.
PINS = {rel: sha for pack in PACKS for rel, sha in [(pack["zip"], pack["sha256"]), *pack["files"].items()]}
UAL = "Universal Animation Library[Standard]/Unreal-Godot/UAL1_Standard.glb"
UBC = "Universal Base Characters[Standard]/Base Characters/Godot - UE/Superhero_Male_FullBody.gltf"
UBC_BIN = "Universal Base Characters[Standard]/Base Characters/Godot - UE/Superhero_Male_FullBody.bin"


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def path(rel):
    """The cached file, verified against its pinned hash."""
    p = os.path.join(CACHE, rel)
    if not os.path.exists(p):
        raise SystemExit(f"{p} is missing; run: python3 packages/scene-assets/blender/packs.py fetch")
    got, want = sha256(p), PINS[rel]
    if got != want:
        raise SystemExit(f"{p}: sha256 {got}, packs.json pins {want}")
    return p


def fetch():
    import http.cookiejar
    import re
    import urllib.parse
    import urllib.request
    import zipfile

    os.makedirs(CACHE, exist_ok=True)
    for pack in PACKS:
        zip_name, page = pack["zip"], pack["source_url"]
        zpath = os.path.join(CACHE, zip_name)
        if not (os.path.exists(zpath) and sha256(zpath) == PINS[zip_name]):
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
        with zipfile.ZipFile(path(zip_name)) as z:
            z.extractall(CACHE)
        for rel in pack["files"]:
            path(rel)
    print(f"packs verified in {CACHE}")


if __name__ == "__main__" and sys.argv[1:] == ["fetch"]:
    fetch()
