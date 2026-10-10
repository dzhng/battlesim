"""The test helicopter (test unit art, never game content): a small utility
airframe on skids with a door HMG (and, with --wreck, its burnt wreck).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/test_heli.py -- [out.glb] [--wreck]

Built to the simulation's test helicopter, `units.test_heli` in
`fixtures/units/test/aircraft.json`: hull half extents [7, 1.2, 1.6] (the
rotorcraft frame: nose to fin, the widest fixed part, the top of the rotor
head; the blades are left out) and the HMG's `mounts` row, pivot
[1.6, 1.12, 1.25] at the left door, muzzle [1.2, 0, 0]. The parts are the
aircraft library's (`aircraft_parts`), as a roster rotorcraft's are, and the
rotors are its `rotor` nodes, which the renderer turns:

    test_heli ─ hull ─┬ rotor_main (about its local Z) ─ blade_main_*
                      ├ rotor_tail (on its side, about its local Z) ─ blade_tail_*
                      └ hmg (yaw at the door) ─ hmg_gun (pitch) ─ hmg_muzzle

The default output is `assets/source/test/heli.glb`; with --wreck it writes
the wreck beside it (`heli_wreck.glb` and its thrown `heli_wreck_debris.glb`).
"""
import math
import os
import sys
import zlib
from pathlib import Path

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import aircraft_parts as A  # noqa: E402
import parts as P  # noqa: E402
from parts import box, cyl, finish, reset, script_args, triangles_by_tier  # noqa: E402
from vehicle_export import Vehicle, materials, rig  # noqa: E402
from vehicle_parts import FINE, MID  # noqa: E402
from wreckage import WRECK_ARG, burn, export_wreck, scatter  # noqa: E402

REPO = Path(__file__).resolve().parents[3]
ARGS = script_args()
WRECK = WRECK_ARG in ARGS
POS = [a for a in ARGS if not a.startswith("--")]
OUT = os.path.abspath(POS[0]) if POS else str(REPO / "assets/source/test/heli.glb")

# sim authority: units.test_heli.body.hull.half_extents_m [7, 1.2, 1.6] and its HMG row
HALF = (7.0, 1.2, 1.6)
HMG = dict(name="HMG", role="hmg", on=None, pivot_m=[1.6, 1.12, 1.25], muzzle_m=[1.2, 0.0, 0.0])
MAST_X = 1.6
HEAD_Z = 2 * HALF[2]  # the rotor head's top: the frame's height

SPEC = dict(
    # A blunt nose under the windscreen, a deep cabin, the boom tapering to the fin.
    fuselage=[(7.0, 0.0, 0.95, 0.95), (6.7, 0.5, 0.62, 1.4, 0.95, 2.2), (5.7, 0.85, 0.45, 1.85, 1.05, 3.0),
              (4.0, 1.0, 0.42, 2.05, 1.15, 3.5), (0.4, 1.0, 0.45, 2.05, 1.2, 3.5), (-1.0, 0.65, 0.9, 1.95, 1.4, 3.0),
              (-5.8, 0.24, 1.35, 1.8, 1.55), (-6.9, 0.2, 1.4, 2.4, 1.75)],
    # The engine deck over the cabin, carrying the mast.
    bodies=[("fuselage_roof", [(3.3, 0.0, 2.0, 2.0), (2.8, 0.62, 1.95, 2.58, 2.15, 3.0),
                               (-0.6, 0.62, 1.95, 2.58, 2.15, 3.0), (-1.4, 0.0, 2.05, 2.05)])],
    canopy=dict(x_front=6.75, x_back=4.6, sill=1.0, top=1.95, half_width=0.82, peak=0.85, bows=(6.1, 5.4), tail=0.9),
    stab=[(-5.55, 0.15, 1.62, 0.6, 0.06), (-5.65, 1.15, 1.62, 0.45, 0.05)],
    fins=[dict(root_x=-6.0, root_z=1.75, height=1.3, root_chord=0.95, tip_chord=0.65, sweep_m=0.4, thick=0.16)],
)


def build(v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The mast up to the rotor head; the head and blades turn on `rotor_main`.
    cyl("mast", 0.16, HEAD_Z - 2.55, (MAST_X, 0, (HEAD_Z + 2.55) / 2), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (MAST_X, 0, HEAD_Z + 0.05), 7.3, 4, 0.5, m, hull, hub=0.36, mast=0.0, droop=0.03, phase=0.3)
    # The tail rotor on the fin's left, turned on its side.
    A.rotor("tail", (-6.55, 0.32, 2.55), 1.15, 4, 0.22, m, hull, hub=0.13, mast=0.0, droop=0.0,
            rot=(-math.pi / 2, 0, 0), thick=0.12)
    A.skids(2.9, -1.6, 1.1, (2.0, -0.6), 0.5, m, hull)
    for side, k in ((1, "L"), (-1, "R")):
        # The sliding cabin door, its window, and the cockpit door's.
        box(f"cabin_door_{k}", (1.7, 0.03, 1.3), (1.6, side * 1.0, 1.25), m["dark"], hull, lods=MID)
        box(f"cabin_window_{k}", (0.6, 0.03, 0.45), (1.9, side * 1.01, 1.6), m["glass"], hull, lods=MID)
        box(f"cockpit_window_{k}", (0.9, 0.03, 0.5), (4.3, side * 0.99, 1.45), m["glass"], hull, lods=MID)
        cyl(f"engine_intake_{k}", 0.2, 0.06, (2.6, side * 0.45, 2.38), "X", m["black"], hull, seg=12, lods=MID)
        box(f"exhaust_{k}", (0.45, 0.18, 0.22), (-0.9, side * 0.5, 2.25), m["nozzle"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_belly", (-0.8, 0, 0.62), 0.25, m, hull, down=True)
    A.nav_lights(m, hull, left=(0.4, 1.02, 1.6), right=(0.4, -1.02, 1.6), tail=(-6.95, 0, 2.5))
    # The door HMG on its pintle: the rig's nodes where the mount row puts them.
    yaw, pitch, _muzzle, _pivot = rig(v.frame, hull)["HMG"]
    cyl("hmg_pintle", 0.04, 0.5, (0, 0, -0.25), "Z", m["dark"], yaw, seg=10)
    box("hmg_cradle", (0.12, 0.1, 0.12), (0, 0, -0.06), m["dark"], yaw, lods=MID)
    box("hmg_receiver", (0.42, 0.13, 0.15), (-0.08, 0, 0), m["dark"], pitch, bevel=0.01)
    cyl("hmg_jacket", 0.035, 0.4, (0.32, 0, 0), "X", m["dark"], pitch, seg=12)
    cyl("hmg_barrel", 0.018, 0.7, (0.85, 0, 0), "X", m["dark"], pitch, seg=10)
    cyl("hmg_flash_hider", 0.028, 0.08, (1.16, 0, 0), "X", m["dark"], pitch, seg=10, lods=MID)
    box("hmg_grips", (0.08, 0.12, 0.08), (-0.33, 0, 0), m["dark"], pitch, lods=MID)
    box("hmg_ammo", (0.24, 0.12, 0.18), (-0.02, -0.14, -0.08), m["store"], pitch, bevel=0.01, lods=FINE)
    A.markings(v, [
        ("text", dict(text="TEST", height=0.3, centre=(-3.0, 0.45, 1.62), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
    ])


def wreck(v):
    # It came down on its belly: the tail broke behind the cabin and the main
    # blades snapped off against the ground. The blades lie thrown clear, past
    # the box the simulation keeps as cover, so they are the wreck's debris.
    A.crash(v, tail_x=-1.4, tail_yaw=0.35, tail_drop=0.1, blades_broken=tuple(("main", k) for k in range(4)),
            seed=6)
    thrown = P.empty("debris_blades", parent=v.root)
    for o in list(bpy.data.objects):
        if o.name.startswith("litter_blade_") and o.parent == v.root:
            o.parent = thrown


reset()
P.SCORCH.clear()
variant = dict(id="test_heli", frame=dict(body_dimensions_m=[2 * h for h in HALF], mounts=[HMG]))
v = Vehicle(variant, materials("nato_helicopter_green"))
v.wreck = WRECK
build(v)
if WRECK:
    wreck(v)
    scatter((HALF[0], HALF[1]), v.mats, seed=zlib.crc32(b"test_heli"))
    burn()
finish(ao_distance=1.0)
counts = triangles_by_tier()
if not all(counts[k] > counts[k + 1] > 0 for k in range(3)):
    raise SystemExit(f"test_heli: tiers must reduce, got {counts}")
if WRECK:
    print("WRECK test_heli", counts, export_wreck(OUT), flush=True)
else:
    P.export(OUT)
    print("TEST_HELI", counts, flush=True)
