"""The towers' review sheets, photographed by `assemble.py`'s camera from the written sets.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/tower_sheets.py <city sets dir> <out dir> [sheet ...]

`<city sets dir>` holds the sets (`assets/source/city`): the towers, and the
houses and apartment blocks they are judged beside. Sheets (all unless named):
  each    one sheet a tower: 30 m at its door, 80 m and 250 m, with its part box
  row     the four side by side from 250 m
  tiers   a tower at each detail tier, at a distance that tier is drawn at (and
          enlarged four times under it), then all four tiers from 250 m
  group   the towers among apartment blocks and houses from 250 m, and one tower
          with two houses at its foot from 80 m

A tower is over 150 px tall, so at tier 0, out to 250 to 500 m: the far tiers are
what a zoomed-out camera sees.
"""
import math
import os
import sys

import numpy as np
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import assemble as A  # noqa: E402

VIEW = math.radians(-118)  # from the south-west: the street front and one side


def by_id(templates):
    return {t["descriptor"]["id"]: t for t in templates["templates"]}


def sheet_each(camera, sets, out, scratch):
    modules, templates = sets["towers"]
    names = templates["modules"]
    for template in templates["templates"]:
        x0, x1, y0, y1, height = A.extent(template)
        door = template["descriptor"]["entrances"][0]["offset_m"]
        panels = []
        views = ((30, (door, y0, 5.0), None), (80, (0, 0, height * 0.42), None), (250, (0, 0, height * 0.45), (1920, 540)))
        for distance, target, crop in views:
            made = A.build(modules, names, template, tier=A.tier_at(height, distance), wires=0.0012 * distance)
            panels.append(A.shoot(camera, os.path.join(scratch, "panel.png"), target, distance, VIEW, crop))
            A.clear(made)
        A.save(os.path.join(out, f"{template['descriptor']['id']}.png"), np.concatenate(panels, 0))


def sheet_row(camera, sets, out, scratch):
    modules, templates = sets["towers"]
    made, x = [], 0.0
    for template in sorted(templates["templates"], key=lambda t: A.extent(t)[4]):
        x0, x1, y0, y1, height = A.extent(template)
        frame = Matrix.Translation((x - x0, -y0, 0))
        made += A.build(modules, templates["modules"], template, frame, tier=A.tier_at(height, 250))
        x += x1 - x0 + 22.0
    image = A.shoot(camera, os.path.join(scratch, "panel.png"), ((x - 22.0) / 2, 10.0, 22.0), 250, math.radians(-104))
    A.clear(made)
    A.save(os.path.join(out, "row-250m.png"), image)


def sheet_tiers(camera, sets, out, scratch):
    modules, templates = sets["towers"]
    for id_ in ("china-tower-20f", "china-tower-slab-10f"):
        template = by_id(templates)[id_]
        x0, x1, y0, y1, height = A.extent(template)
        # where each tier is drawn: 250 m, then the projected heights 100, 40 and 18 px
        far, near = [], []
        for tier, px in enumerate((None, 100, 40, 18)):
            distance = 250 if px is None else height * A.FOCAL_PX / px
            assert A.tier_at(height, distance) == tier, (id_, tier, distance)
            made = A.build(modules, templates["modules"], template, tier=tier)
            far.append(A.shoot(camera, os.path.join(scratch, "panel.png"), (0, 0, height * 0.45), distance, VIEW, (480, 540)))
            A.clear(made)
            made = A.build(modules, templates["modules"], template, tier=tier)
            near.append(A.shoot(camera, os.path.join(scratch, "panel.png"), (0, 0, height * 0.45), 250, VIEW, (480, 540)))
            A.clear(made)
        zoom = [np.repeat(np.repeat(p[202:337, 180:300], 4, 0), 4, 1) for p in far]  # the middle, four times the size
        A.save(os.path.join(out, f"tiers-{id_}.png"), np.concatenate([np.concatenate(r, 1) for r in (far, zoom, near)], 0))


def sheet_group(camera, sets, out, scratch):
    """A made-up district: the towers behind a street of apartment blocks, houses across it."""
    plan = [  # (set, template, x of its west end, y of its street front, the way it faces: 1 south, -1 north)
        ("towers", "china-tower-slab-10f", 0.0, 78.0, 1), ("towers", "china-tower-20f", 76.0, 70.0, 1),
        ("towers", "china-tower-12f", 126.0, 76.0, 1), ("towers", "china-tower-16f", 172.0, 70.0, 1),
        ("china_apartments", "china-apartment-slab-47x11-5f", 2.0, 12.0, 1),
        ("china_apartments", "china-apartment-point-20x20-7f", 62.0, 12.0, 1),
        ("china_apartments", "china-apartment-slab-53x14-8f", 96.0, 12.0, 1),
        ("china_apartments", "china-apartment-slab-35x11-4f", 164.0, 12.0, 1),
        ("homes", "china-home-12x9-2f", 6.0, -12.0, -1), ("homes", "china-home-10x8-1f", 28.0, -12.0, -1),
        ("homes", "china-terrace-5x3f", 48.0, -12.0, -1), ("homes", "china-home-9x9-2f", 90.0, -12.0, -1),
        ("homes", "china-shops-4x3f", 110.0, -12.0, -1), ("homes", "china-home-ell-2f", 150.0, -12.0, -1),
        ("homes", "china-townhouse-2f", 172.0, -12.0, -1), ("homes", "china-home-8x11-1f", 190.0, -12.0, -1),
    ]
    foot = [("towers", "china-tower-12f", 0.0, 0.0, 1), ("homes", "china-home-12x9-2f", -20.0, 0.0, 1),
            ("homes", "china-terrace-3x2f", 34.0, 0.0, 1)]
    for name, plan, distance, target, azimuth in (("group-250m.png", plan, 250, (100.0, 30.0, 10.0), -108),
                                                  ("scale-80m.png", foot, 80, (14.0, 6.0, 9.0), -100)):
        pitch = A.pitch_at(distance)
        eye = Vector(target) + distance * Vector((math.cos(pitch) * math.cos(math.radians(azimuth)),
                                                  math.cos(pitch) * math.sin(math.radians(azimuth)), math.sin(pitch)))
        made = [A.slab("street", (-40, 260, -8, 8), 0.0, (0.05, 0.05, 0.052))]
        for set_id, id_, x, y, facing in plan:
            if set_id not in sets:
                continue
            modules, templates = sets[set_id]
            template = by_id(templates)[id_]
            x0, x1, y0, y1, height = A.extent(template)
            centre = Vector((x + (x1 - x0) / 2, y, 0.0))
            frame = (Matrix.Translation(centre) @ Matrix.Rotation(0.0 if facing > 0 else math.pi, 4, "Z")
                     @ Matrix.Translation((-(x0 + x1) / 2, -y0, 0)))
            made += A.build(modules, templates["modules"], template, frame, tier=A.tier_at(height, (centre - eye).length))
        image = A.shoot(camera, os.path.join(scratch, "panel.png"), target, distance, math.radians(azimuth))
        A.clear(made)
        A.save(os.path.join(out, name), image)


def main():
    args = [a for a in sys.argv[sys.argv.index("--") + 1:] if a != "--"] if "--" in sys.argv else []
    if len(args) < 2:
        raise SystemExit("tower_sheets.py <city sets dir> <out dir> [each|row|tiers|group ...]")
    root, out = os.path.abspath(args[0]), os.path.abspath(args[1])
    sheets = {"each": sheet_each, "row": sheet_row, "tiers": sheet_tiers, "group": sheet_group}
    wanted = args[2:] or list(sheets)
    scratch = os.path.join(out, "scratch")
    os.makedirs(scratch, exist_ok=True)
    sets = {}
    for set_id in ["towers"] + (["homes", "china_apartments"] if "group" in wanted else []):
        if os.path.exists(os.path.join(root, set_id, "kit.glb")):
            sets[set_id] = A.load(os.path.join(root, set_id), fresh=not sets)
    camera = A.stage()
    camera.data.clip_end = 12000.0
    for name in wanted:
        sheets[name](camera, sets, out, scratch)


main()
