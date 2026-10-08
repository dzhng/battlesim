"""Each roster variant's physical frame, read from the resolved catalogs.

The simulation is the authority on fit, so a roster exporter builds to the
frame of the unit type that draws its appearance: hull extents, eye height
and mounts from `fixtures/catalog.json`, each mount's articulation role from
the appearance's `mounts` in `assets/catalog.json`. A family is every
appearance whose source lies in `assets/source/roster/<family>/`.

Plain Python, no Blender: exporters import it, and it is tested on its own.
An appearance no unit draws is refused by name. A disabled card has no unit
type, so `disabled_variant` takes the frame its exporter states from the
card's references: its length, width and height, and the mounts a turreted
card rigs for its art alone.
"""
import json
import os
from pathlib import Path

REPO = Path(os.path.dirname(os.path.abspath(__file__))).parents[2]


def _catalogs(repo):
    units = json.loads((Path(repo) / "fixtures/catalog.json").read_text())["units"]
    appearances = json.loads((Path(repo) / "assets/catalog.json").read_text())["appearances"]
    return units, appearances


def _variant(appearance_id, appearance, units):
    drawn_by = [u for u in units if u.get("appearance") == appearance_id]
    if len(drawn_by) != 1:
        raise LookupError(
            f"{appearance_id}: drawn by {len(drawn_by)} unit types in fixtures/catalog.json, "
            "need exactly one to take its frame from"
        )
    unit = drawn_by[0]
    hull = unit["body"]["hull"]
    roles = appearance.get("mounts", {})
    mounts = [
        dict(
            name=mount["name"],
            role=roles.get(mount["name"]),
            on=mount["on"],
            pivot_m=mount["pivot_m"],
            muzzle_m=mount["muzzle_m"],
        )
        for mount in unit["mounts"]
    ]
    frame = dict(
        half_extents_m=hull["half_extents_m"],
        body_dimensions_m=[2 * h for h in hull["half_extents_m"]],
        eye_m=hull["eye_m"],
        mounts=mounts,
    )
    return dict(
        id=unit["id"],
        name=unit["name"],
        variant=unit["roster"]["variant"],
        faction=unit["faction"],
        export=appearance["source"],
        frame=frame,
    )


def roster_variant(appearance_id, repo=REPO):
    """The variant drawn by one appearance: unit id, names, faction, export path and frame."""
    units, appearances = _catalogs(repo)
    if appearance_id not in appearances:
        raise LookupError(f"{appearance_id}: no appearance in assets/catalog.json")
    return _variant(appearance_id, appearances[appearance_id], units)


def requested_variant(family, args, repo=REPO):
    """A per-variant exporter's request: (variant, output path).

    The first positional argument is an appearance id, written to its catalog
    source, or an output `.glb` named after the appearance it draws. The
    appearance must be one of the family's; a family of one may omit it.
    """
    target = next((a for a in args if not a.startswith("--")), None)
    if target is None:
        variants = family_variants(family, repo)
        if len(variants) != 1:
            raise LookupError(f"{family}: name one of its {len(variants)} appearances")
        return variants[0], str(Path(repo) / variants[0]["export"])
    appearance_id = os.path.basename(target).removesuffix(".glb")
    variant = roster_variant(appearance_id, repo)
    folder = f"assets/source/roster/{family}/"
    if not variant["export"].startswith(folder):
        raise LookupError(f"{appearance_id}: its source is not in {folder}")
    if target.endswith(".glb"):
        return variant, os.path.abspath(target)
    return variant, str(Path(repo) / variant["export"])


def family_variants(family, repo=REPO):
    """Every variant whose source is in the family's folder, by appearance id."""
    units, appearances = _catalogs(repo)
    folder = f"assets/source/roster/{family}/"
    ids = sorted(k for k, a in appearances.items() if a.get("source", "").startswith(folder))
    if not ids:
        raise LookupError(f"{family}: no appearance in assets/catalog.json has a source in {folder}")
    return [_variant(i, appearances[i], units) for i in ids]


def disabled_variant(card_id, dimensions, mounts=(), repo=REPO):
    """A disabled card's export path, names, faction and frame: `dimensions`
    (length, width and height in metres) and `mounts` (each as a catalog
    mount, with its articulation `role`) as its exporter states them from its
    references (`frame_source`)."""
    entries = json.loads((Path(repo) / "fixtures/units/model-manifest.json").read_text())["entries"]
    entry = next((e for e in entries if e["id"] == card_id), None)
    if entry is None:
        raise LookupError(f"{card_id}: no card in fixtures/units/model-manifest.json")
    card = next(
        doc["units"][card_id]
        for path in sorted((Path(repo) / "fixtures/units/roster").glob("*.json"))
        for doc in [json.loads(path.read_text())]
        if card_id in doc.get("units", {})
    )
    frame = dict(
        half_extents_m=[d / 2 for d in dimensions],
        body_dimensions_m=list(dimensions),
        eye_m=None,
        mounts=[dict(m) for m in mounts],
    )
    return dict(
        id=card_id,
        name=card["name"],
        faction=card["faction"],
        category=entry["category"],
        export=entry["source_path"],
        frame=frame,
        frame_source="references",
    )
