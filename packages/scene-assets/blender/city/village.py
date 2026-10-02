"""The authored maps' buildings: the village farm, built on every box their catalogue has.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/village.py

writes `assets/source/city/village/kit.glb` and `templates.json` (the format is in
this folder's readme). The village and the labs place the solid boxes of
`fixtures/building-templates.json`, which no set derives: this one dresses each of
them as it is written. Every box is the courtyard farm of `house.py`, built at the
box's own size, never stretched to it: one module for the farm standing and one for
its ruin, each the whole building in the template's frame.

The farm has three looks. Each belongs to the village house it was first drawn for,
and any other box (a lab's) takes the look of the house nearest it in size, a tier
coarser: its finest tier is the farm's second, painted as that tier is. Twelve farms
at the houses' own detail are more than a kit's bundle may weigh.
"""
import json
import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403
from house import BAKE, farmstead  # noqa: E402

LOOKS = {(15.0, 12.0, 4.0): 0, (17.0, 14.0, 4.0): 1, (13.0, 11.0, 4.0): 2}
# A box that borrows its look: its meshes are a tier coarser (`coarsen`), and so is its paint, whose
# edge about doubles from a tier to the next (`parts.PAINT_EDGE_M`).
BORROWED = BAKE | dict(paint_scale=2 * BAKE["paint_scale"])

# (The farm's ruin is older than the rule that a ruin draws no more than its building: its heaps outdraw the farm at two tiers.)
kit = Kit("village", "village.py", fit_side_m=0.5, fit_top_m=0.5, fit_ruin_top_m=0.6, damage_budget=False)

with open(os.path.join(REPO, "fixtures", "building-templates.json")) as f:
    ROWS = json.load(f)["templates"]


def look(half):
    return min(LOOKS.items(), key=lambda at: sum(abs(math.log(a / b)) for a, b in zip(half, at[0])))[1]


def coarsen(root):
    """Drop a farm's finest tier: every tier takes the next one's meshes, and the last keeps its own."""
    for o in sorted(root.children_recursive, key=lambda o: o.name):
        tier = tier_of(o)
        if tier == 0:
            bpy.data.objects.remove(o, do_unlink=True)
        elif tier is not None:
            if tier == TIERS[-1]:
                last = o.copy()
                last.data = o.data.copy()
                bpy.context.scene.collection.objects.link(last)
            o.name = o.data.name = f"{o.name[:-1]}{tier - 1}"
            if tier == TIERS[-1]:
                last.name = last.data.name = f"{o.name[:-1]}{tier}"


for row in ROWS:
    (part,) = row["parts"]
    hx, hy, hz = part["half_extents"]
    if part["center"] != [0.0, 0.0] or part["base_z"]:
        raise SystemExit(f"{row['id']}: the farm stands on one box, centred on the ground")
    variant = look((hx, hy, hz))
    borrowed = (hx, hy, hz) not in LOOKS
    t = kit.dress(row, recipe=dict(Box=[2 * hx, 2 * hy, 2 * hz], Look=variant))
    for state, ruin in (("intact", None), ("ruin", t.ruin_height())):
        m = kit.module(f"farm_{hx:g}x{hy:g}x{hz:g}_{state}", ground=True, **(BORROWED if borrowed else BAKE))
        farmstead(m.root, hx, hy, hz, variant, ruin, tag=f"_{variant}" + ("_burnt" if ruin else ""))
        if borrowed:
            coarsen(m.root)
        for o in m.root.children_recursive:  # Blender's names are one namespace: a part carries its module's
            o.name = o.data.name = m.n(o.name)
        t.place(m.name, state=state)

rest_on_ground()
kit.write(next(iter(script_args()), None))  # an argument writes the two files somewhere else
