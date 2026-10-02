"""The industrial set's sheets, at the game's camera: `assemble.py` rebuilds and lights the set.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/industry_sheets.py -- <industry set dir> <homes set dir> <out dir> [sheet ...]

A warehouse is ten times a house's plan, so `assemble.py`'s house-sized sheets crop
it. These frame it (all of them unless named):
  each     one sheet a template: 30 m in front of its first door, then the whole of it
           from 80 m and from 250 m, with its part boxes
  row      all five side by side, from 250 m
  estate   a made-up industrial estate along a road, with houses for scale, from 250 m
  tiers    the shed at each of its four detail tiers from 80 m, and the works from 250 m
"""
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import assemble as A  # noqa: E402

FACADES = {"positive_x": ((1, 0), (0, 1)), "positive_y": ((0, 1), (-1, 0)), "negative_x": ((-1, 0), (0, -1)), "negative_y": ((0, -1), (1, 0))}
ASPHALT, YARD = (0.05, 0.05, 0.052), (0.2, 0.195, 0.18)


def short(template):
    return template["descriptor"]["id"].replace("china-", "")


def first_door(template):
    """Where the first entrance is, on its wall."""
    d = template["descriptor"]
    door = d["entrances"][0]
    edge = next(e for e in d["edges"] if e["id"] == door["edge"])
    part = next(p for p in d["parts"] if p["id"] == edge["part"])
    normal, along = FACADES[edge["facade"]]
    half = part["half_extents"][0] if normal[0] else part["half_extents"][1]
    return Vector((part["center"][0] + normal[0] * half + along[0] * door["offset_m"],
                   part["center"][1] + normal[1] * half + along[1] * door["offset_m"], 0.0))


def sheet_each(camera, sets, out, scratch):
    modules, templates = sets["industry"]
    names = templates["modules"]
    for template in templates["templates"]:
        x0, x1, y0, y1, height = A.extent(template)
        centre = ((x0 + x1) / 2, (y0 + y1) / 2, height * 0.4)
        panels = []
        for distance, target, crop in ((30, first_door(template) + Vector((0, 0, 2.5)), None), (80, centre, None),
                                       (250, centre, (A.WIDTH, 540))):
            made = A.build(modules, names, template, tier=A.tier_at(height, distance), wires=0.0012 * distance)
            panels.append(A.shoot(camera, os.path.join(scratch, "panel.png"), target, distance, math.radians(-118), crop))
            A.clear(made)
            A.save(os.path.join(scratch, f"{short(template)}-{distance}m.png"), panels[-1])
        A.save(os.path.join(out, f"{short(template)}.png"), np.concatenate(panels, 0))


def sheet_row(camera, sets, out, scratch):
    modules, templates = sets["industry"]
    made, x, gap = [], 0.0, 16.0
    made.append(A.slab("road", (-40, 400, -30, -22), 0.0, ASPHALT))
    for template in sorted(templates["templates"], key=lambda t: A.extent(t)[1] - A.extent(t)[0]):
        x0, x1, y0, y1, height = A.extent(template)
        frame = Matrix.Translation((x - x0, -y0, 0))  # every street front on one line
        made += A.build(modules, templates["modules"], template, frame, tier=A.tier_at(height, 250))
        made.append(A.label(short(template), (x + (x1 - x0) / 2, -12.0), size=5.0))
        x += x1 - x0 + gap
    image = A.shoot(camera, os.path.join(scratch, "panel.png"), ((x - gap) / 2, 14.0, 3.0), 250, math.radians(-100))
    A.clear(made)
    A.save(os.path.join(out, "row-250m.png"), image)


def sheet_estate(camera, sets, out, scratch):
    """A road with yards either side: the depot and the works north of it, the warehouses south, a shed on each side, and houses where the estate ends."""
    by = {short(t): (name, t) for name, (_, templates) in sets.items() for t in templates["templates"]}
    rows = [  # (which side of the road, the buildings along it from the west, the gap after each)
        (1, [("depot-90x39", 18), ("works-54x36", 16), ("shed-15x24", 14), ("home-12x9-2f", 8), ("home-10x8-1f", 8)]),
        (-1, [("warehouse-72x33", 18), ("warehouse-48x24", 16), ("shed-15x24", 14), ("home-9x9-2f", 8), ("terrace-3x2f", 8)]),
    ]
    target = Vector((118.0, 0.0, 0.0))
    azimuth = math.radians(-112)
    eye = target + 250 * Vector((math.cos(0.85) * math.cos(azimuth), math.cos(0.85) * math.sin(azimuth), math.sin(0.85)))
    made = [A.slab("road", (-60, 300, -4.5, 4.5), 0.0, ASPHALT)]
    count = 0
    for side, buildings in rows:
        x = 0.0
        for id_, gap in buildings:
            set_name, template = by[id_]
            modules, templates = sets[set_name]
            x0, x1, y0, y1, height = A.extent(template)
            setback = 16.0 if set_name == "industry" else 9.0
            if set_name == "industry":  # its yard, out to the road
                made.append(A.slab(f"yard_{count}", (x - 5, x + x1 - x0 + 5, *sorted((side * 4.5, side * (setback + y1 - y0 + 5)))), -0.01, YARD))
            centre = Vector((x + (x1 - x0) / 2, side * setback, 0.0))
            frame = (Matrix.Translation(centre) @ Matrix.Rotation(0.0 if side > 0 else math.pi, 4, "Z")
                     @ Matrix.Translation((-(x0 + x1) / 2, -y0, 0)))
            made += A.build(modules, templates["modules"], template, frame, tier=A.tier_at(height, (centre - eye).length))
            x += x1 - x0 + gap
            count += 1
    image = A.shoot(camera, os.path.join(scratch, "panel.png"), target, 250, azimuth)
    A.clear(made)
    A.save(os.path.join(out, "estate-250m.png"), image)
    print(f"estate: {count} buildings")


def sheet_tiers(camera, sets, out, scratch):
    modules, templates = sets["industry"]
    by = {short(t): t for t in templates["templates"]}
    strips = []
    for id_, pitch, distance, crop in (("shed-15x24", 24.0, 80, (A.WIDTH, 700, 20)), ("works-54x36", 68.0, 250, (A.WIDTH, 560, 20))):
        made = []
        for tier in range(4):
            made += A.build(modules, templates["modules"], by[id_], Matrix.Translation((tier * pitch, 0, 0)), tier=tier)
            made.append(A.label(f"tier {tier}", (tier * pitch, A.extent(by[id_])[2] - 7.0), size=1.6 * distance / 80))
        strips.append(A.shoot(camera, os.path.join(scratch, "panel.png"), (1.5 * pitch, 0.0, 3.0), distance, math.radians(-100), crop))
        A.clear(made)
    A.save(os.path.join(out, "tiers.png"), np.concatenate(strips, 0))


def main():
    args = [a for a in sys.argv[sys.argv.index("--") + 1:] if a != "--"] if "--" in sys.argv else []
    if len(args) < 3:
        raise SystemExit("industry_sheets.py <industry set dir> <homes set dir> <out dir> [each|row|estate|tiers ...]")
    industry, homes, out = (os.path.abspath(a) for a in args[:3])
    sheets = {"each": sheet_each, "row": sheet_row, "estate": sheet_estate, "tiers": sheet_tiers}
    scratch = os.path.join(out, "scratch")
    os.makedirs(scratch, exist_ok=True)
    sets = {"homes": A.load(homes)}
    for block in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.images):
        for item in block:  # out of the next kit's way: both have a `gutter`, a `wall_brick`, a `concrete_albedo`
            item.name = "homes:" + item.name
    sets["industry"] = A.load(industry, fresh=False)
    camera = A.stage()
    for name in args[3:] or list(sheets):
        sheets[name](camera, sets, out, scratch)


main()
