"""The export run every rebuilt roster vehicle family shares.

A family script (`roster/<family>.py`) supplies only what its vehicle looks
like: `build(variant, v)` places its parts, and `wreck(variant, v)`, given,
works the built vehicle over into its own wreck before it burns. This module
does the rest, the same way for every family:

- each variant's frame from the resolved catalogs (`catalog_frames`), one
  appearance with `--variant=<appearance id>` or all of them; a disabled
  card family (`run_disabled`) takes each card's frame from the dimensions
  its script states from its references, and the built model must measure it;
- the materials, one per role (`materials`), in the family's real scheme;
- each mount's articulation nodes at its frame's pivot and muzzle (`rig`);
- `--wreck`: the family's damage, then `wreckage.burn()`, written beside the
  live vehicle with its hull and turret pieces where it has a turret;
- visible crew after the paint is baked (they keep the soldier's own), and
  their materials carried through the export;
- tiers checked to reduce, the GLB exported, and the family's receipt.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/<family>.py -- [--variant=<id>] [--wreck]
"""
import hashlib
import json
import math
import os
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import parts as P  # noqa: E402
from catalog_frames import family_variants  # noqa: E402
from parts import bare_steel, empty, finish, flat_paint, glass, reset, script_args, textured, track_steel, tyre, triangles_by_tier  # noqa: E402
from wreckage import WRECK_ARG, burn, export_wreck  # noqa: E402

REPO = Path(__file__).resolve().parents[3]
# Each articulation's nodes (MOUNT_NODES in packages/scene-assets/src/validate.ts).
RIG_NODES = {"gun": ("turret", "gun", "muzzle"), "hmg": ("hmg", "hmg_gun", "hmg_muzzle")}


def materials(scheme, fittings=(0.11, 0.095, 0.068), canvas=(0.105, 0.098, 0.062), chip=0.35):
    """One material per role `vehicle_parts` reads, in `scheme`. `fittings` is
    the darker painted tone of hubs, brackets and running gear fittings;
    `canvas` the stowage's cloth; `chip` how much of the paint's convex edges
    wear to its lighter tone (the look's painted edge highlights)."""
    return {
        "paint": P.paint(scheme, "armor_paint", chip=chip, dirt=0.5, rise=1.1),
        "dark": textured("fittings", "olive_paint", colour=fittings, chip=0.3, dirt=0.55, role="paint"),
        "steel": bare_steel("steel", chip=0.3, dirt=0.3),
        "black": flat_paint("recesses", (0.014, 0.014, 0.013), rough=0.85, grime=0.15),
        "rubber": tyre("rubber"),
        "glass": glass("optics"),
        "lamp": flat_paint("lamps", (0.62, 0.56, 0.42), rough=0.25, grime=0.05),
        "tail": flat_paint("tail_lamps", (0.35, 0.03, 0.02), rough=0.25, grime=0.05),
        "canvas": textured("stowage_canvas", "canvas", colour=canvas, chip=0.1, dirt=0.35, streak=0.0, role="fabric"),
        "track": track_steel("track"),
        "marking": textured("marking", "marking_paint", colour=(0.016, 0.016, 0.015), chip=0.8, dirt=0.4, streak=0.0,
                            role="marking"),
    }


def rig(frame, root, trunnion=None):
    """Each mount's yaw, pitch and muzzle nodes, by its role, at its frame's
    pivot (in the hull frame) and muzzle (from the pivot along the bore). A
    mount carried by another (`on`) yaws on that one's yaw node. `trunnion`
    (by mount name) puts the pitch node that far ahead of the pivot, along the
    bore, as a tank gun's trunnions are; else it is at the pivot. Returns, by
    mount name, (yaw, pitch, muzzle, pivot)."""
    made = {}
    for mount in frame["mounts"]:
        yaw_name, pitch_name, muzzle_name = RIG_NODES[mount["role"]]
        pivot = Vector(mount["pivot_m"])
        carrier, at = (root, Vector((0, 0, 0))) if mount["on"] is None else (made[mount["on"]][0], made[mount["on"]][3])
        yaw = empty(yaw_name, tuple(pivot - at), carrier)
        muzzle = Vector(mount["muzzle_m"])
        ahead = (trunnion or {}).get(mount["name"], 0.0)
        pitch = empty(pitch_name, (ahead, 0, muzzle.z), yaw)
        tip = empty(muzzle_name, (muzzle.x - ahead, muzzle.y, 0), pitch)
        made[mount["name"]] = (yaw, pitch, tip, pivot)
    return made


class Vehicle:
    """What a family's build places its parts on: `root`, `hull` (the static
    body), `mats`, the frame and its sizes, and `crew`, the crewmen to pose
    (`vehicle_crew.crew` arguments after the source)."""

    def __init__(self, variant, mats):
        self.variant = variant
        self.frame = variant["frame"]
        self.length, self.width, self.height = self.frame["body_dimensions_m"]
        self.mats = mats
        self.root = empty(variant["id"])
        self.root["unit_id"] = variant["id"]
        self.hull = empty("hull", parent=self.root)
        self.crew = []
        self.wreck = False

    def head_out(self, name, parent, x, y, roof, facing=0.0):
        """A crewman head and shoulders out of the hatch at (x, y) on a roof
        `roof` metres up (world, at rest), facing `facing` radians from +X, his
        hands on the hatch rim, posed under a `dressing_<name>` node of
        `parent`. None on a wreck."""
        if self.wreck:
            return
        seat = empty(f"dressing_{name}", parent=parent)
        c, s = math.cos(facing), math.sin(facing)

        def at(fx, fy, z):
            return (x + fx * c - fy * s, y + fx * s + fy * c, z)

        self.crew.append((name, seat, at(0.0, 0.0, roof - 0.43),
                          (at(0.22, 0.22, roof + 0.06), at(0.22, -0.22, roof + 0.06)),
                          (at(0.14, 0.11, roof - 1.25), at(0.14, -0.11, roof - 1.25))))


def _export(variants, scheme, build, wreck, ao_distance, ao_rays, check=None, looks=None):
    """Build and export each of `variants` (or the one `--variant=<id>` names),
    live or with `--wreck`; returns the receipt rows, or None for a partial run.
    `scheme` is one scheme or a scheme per variant id; `looks` are `materials`
    keywords (a family's fittings, canvas or chip)."""
    args = script_args()
    selected = next((a.split("=", 1)[1] for a in args if a.startswith("--variant=")), None)
    wrecking = WRECK_ARG in args
    receipt = []
    if selected and selected not in {v["id"] for v in variants}:
        raise SystemExit(f"{selected}: not one of {', '.join(v['id'] for v in variants)}")
    for variant in variants:
        if selected and variant["id"] != selected:
            continue
        reset()
        P.SCORCH.clear()
        own = scheme[variant["id"]] if isinstance(scheme, dict) else scheme
        v = Vehicle(variant, materials(own, **(looks or {})))
        v.wreck = wrecking
        build(variant, v)
        if check is not None and not wrecking:
            check(variant)
        if wrecking:
            if wreck is not None:
                wreck(variant, v)
            burn()
        finish(ao_distance=ao_distance, ao_rays=ao_rays)
        source = None
        if v.crew and not wrecking:
            from vehicle_crew import crew, soldier_source
            source = soldier_source(variant["faction"])
            for args_ in v.crew:
                crew(*args_, source)
        counts = triangles_by_tier()
        if not all(counts[k] > counts[k + 1] > 0 for k in range(3)):
            raise SystemExit(f"{variant['id']}: tiers must reduce, got {counts}")
        out = str(REPO / variant["export"])
        if wrecking:
            print("WRECK", variant["id"], counts, export_wreck(out), flush=True)
            continue
        P.export(out)
        if source is not None:
            from vehicle_crew import preserve_materials
            preserve_materials(out, source)
        row = {"id": variant["id"], "sha256": hashlib.sha256(Path(out).read_bytes()).hexdigest(),
               "triangles_by_tier": counts}
        if "frame_source" in variant:
            row.update(frame_source=variant["frame_source"], body_dimensions_m=variant["frame"]["body_dimensions_m"])
        receipt.append(row)
        print("VARIANT", variant["id"], counts, flush=True)
    return None if wrecking or selected else receipt


def _receipt(path, script, variants, frames, references=None):
    here = Path(__file__).resolve().parent
    sources = [here / "roster" / script, here / "vehicle_export.py", here / "vehicle_parts.py",
               here / "vehicle_crew.py", here / "parts.py", here / "textures.py", here / "wreckage.py",
               here / "catalog_frames.py"]
    if "aircraft_parts" in (here / "roster" / script).read_text():
        sources.append(here / "aircraft_parts.py")
    family = script.removesuffix(".py")
    (REPO / path).write_text(json.dumps({
        "blender_version": bpy.app.version_string,
        "source_sha256": {str(p.relative_to(REPO)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
        "references": references or f"assets/references/{family}/references.json",
        "authoring": "Original procedural geometry built from the committed references; photos are visual reference only.",
        "frames": frames,
        "variants": variants,
    }, indent=2) + "\n")


def run(family, scheme, build, wreck=None, ao_distance=1.0, ao_rays=8, **looks):
    """Export every variant of `family` (or `--variant=<id>`), live or with
    `--wreck` its wreck, through `build` and `wreck` (see the module doc).
    `looks` are `materials` keywords (a family's fittings, canvas or chip)."""
    receipt = _export(family_variants(family), scheme, build, wreck, ao_distance, ao_rays, looks=looks)
    if receipt is not None:
        _receipt(f"assets/source/roster/{family}/source-receipt.json", f"{family}.py", receipt, "fixtures/catalog.json")


# How far a disabled card's built model may stray from the dimensions its
# script states from the references (its frame, `frame_source: references`).
FRAME_TOLERANCE = 0.05


def _measured(skip):
    """The tier-0 extents (length, width, height) of what isn't under a node
    named by `skip`, and its lowest point."""
    bpy.context.view_layer.update()
    lo, hi = Vector((1e9,) * 3), Vector((-1e9,) * 3)
    for o in bpy.data.objects:
        if o.type != "MESH" or P.tier_of(o) not in (0, None):
            continue
        chain, p = [o], o.parent
        while p is not None:
            chain.append(p)
            p = p.parent
        if any(c.name.startswith(skip) for c in chain):
            continue
        for vert in o.data.vertices:
            w = o.matrix_world @ vert.co
            lo = Vector(map(min, lo, w))
            hi = Vector(map(max, hi, w))
    return hi - lo, lo


def run_disabled(family, cards, scheme, build, wreck=None, skip=("dressing_",), ao_distance=1.0, ao_rays=8):
    """Export a disabled card family: `cards` maps each card id to the
    (length, width, height) its script states from its references, its frame
    (`catalog_frames.disabled_variant`); `scheme` is the family's, or one
    per card id. The built model must measure that
    frame within `FRAME_TOLERANCE`, leaving out the nodes `skip` names (a
    rotor disc, dressing). Each card writes its `source_path`; the family's
    receipt, beside them, records each card's frame source."""
    from catalog_frames import disabled_variant

    def check(variant):
        size, low = _measured(skip)
        want = variant["frame"]["body_dimensions_m"]
        off = [abs(s - w) / w for s, w in zip(size, want)]
        print("FRAME", variant["id"], [round(s, 2) for s in size], "stated", want, flush=True)
        if max(off) > FRAME_TOLERANCE or abs(low.z) > 0.02:
            raise SystemExit(f"{variant['id']}: built {[round(s, 2) for s in size]} m, lowest {low.z:.2f} m; "
                             f"its references state {want} m on the ground")

    variants = [disabled_variant(card, dims) for card, dims in cards.items()]
    receipt = _export(variants, scheme, build, wreck, ao_distance, ao_rays, check)
    if receipt is not None:
        entries = json.loads((REPO / "fixtures/units/model-manifest.json").read_text())["entries"]
        families = sorted({e["source_family"] for e in entries if e["id"] in cards})
        _receipt(f"assets/source/roster/disabled/{family}.source-receipt.json", f"{family}.py", receipt,
                 "catalog_frames.disabled_variant", [f"assets/references/{f}/references.json" for f in families])
