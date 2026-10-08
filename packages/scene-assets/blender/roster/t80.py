"""T-80BVM, from assets/references/t80/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t80.py -- [--variant=<id>] [--wreck]

What the photos settle (Murmansk rehearsal, Army-2020 and 2022): a longer
hull than the T-72's, its six rubber-tyred road wheels smaller and evenly
spaced, five return rollers behind the side armour, the sprocket at the rear
and the idler at the front; the hull sides in two rows of Relikt panels over
the front two-thirds, the lower row's edge scalloped, slat panels over the
rear and a slat cage on the hull rear with fuel drums over it; the turbine
deck's broad grille and the exhaust louvre in the rear plate. The cast turret
carries the broad Relikt cheek kit, a band of flat panels round its front arc
continuing as modules down its flanks, Relikt tiles on the roof front, smoke
tubes, the gunner's sight on the left, the commander's cupola with its
machine gun on the right, and stowage boxes and a slat cage on the rear. The
commander rides head out.

Gun, machine gun and Soviet skirts are the T-72's (`roster/t72.py`); the hull
and running gear are the T-80's own.

Built to the catalog frame (hull 7.4 x 3.603 x 2.202 m, turret pivot 1.321 m,
cannon muzzle 6.2 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from t72 import TRUNNION, cupola_mg, dome_rings, gun_2a46, side_modules, soviet_skirts, soviet_wreck  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TRACK_Y = 1.42
TRACK_W = 0.58
ROAD_R = 0.335
ROAD_Z = 0.415
ROAD_X = [2.35 - 0.92 * k for k in range(6)]
SPROCKET = (-3.20, 0.52, 0.32)
IDLER = (3.22, 0.50, 0.29)
RETURNS = [(1.90, 0.82, 0.09), (1.00, 0.83, 0.09), (0.10, 0.83, 0.09), (-0.80, 0.83, 0.09), (-1.70, 0.82, 0.09)]
DECK = 1.30


def glacis_z(x):
    if x >= 2.30:
        return DECK - (x - 2.30) / 1.40 * 0.40
    return DECK


def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.92), (3.55, 0.92), (half, 0.94), (2.30, DECK), (-3.62, DECK), (-half, 1.22)],
          3.30, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.45, 0.44), (3.15, 0.44), (3.68, 0.94), (-half, 0.94), (-half, 0.78)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan(0.40 / 1.40)
    VP.armour_tiles("glacis_era", (3.02, 0, glacis_z(3.02) + 0.005), (1.10, 2.70), (2, 6), 0.09, m, hull,
                    rot=(0, slope, 0))
    prism("splash_board", [(2.40, DECK - 0.02), (2.50, DECK - 0.02), (2.44, DECK + 0.12), (2.34, DECK + 0.12)], 2.60,
          mat=m["paint"], parent=hull, bevel=0.015)
    VP.hatch("driver_hatch", (2.05, 0, DECK), m, hull, radius=0.30)
    VP.periscope("driver_periscope", (2.31, 0, DECK), m, hull, size=(0.12, 0.26, 0.09))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (2.25, side * 1.30, DECK + 0.10), 0.07, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.56, side * 0.62, 0.80), m, hull, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-3.56, side * 0.80, 0.82), m, hull, size=0.13, rot=(0, 0, math.pi))
        box(f"front_fender_{s}", (0.45, 0.60, 0.04), (3.45, side * TRACK_Y, 1.02), m["paint"], hull, bevel=0.012,
            rot=(0, 0.35, 0), lods=MID)
        VP.cable(f"tow_cable_{s}", [(-3.2, side * 1.52, DECK + 0.02), (-1.8, side * 1.54, DECK + 0.02),
                                    (0.0, side * 1.54, DECK + 0.02), (1.6, side * 1.52, DECK + 0.02)], m, hull,
                 radius=0.022)
        VP.stowage_box(f"fender_box_{s}", (0.55, side * 1.50, DECK), (0.80, 0.36, 0.30), m, hull,
                       rot=(0, 0, 0 if side > 0 else math.pi))
    # The turbine deck: one broad grille, the exhaust louvre in the rear plate.
    VP.grille("engine_grille", (-2.55, 0, DECK), (1.60, 2.10), m, hull, slats=14)
    VP.bolted_panel("engine_door", (-1.35, 0.0, DECK), (0.70, 2.00, 0.03), m, hull, bolts=(2, 3), bevel=0.01,
                    lods=MID)
    VP.grille("exhaust_louvre", (-half, 0, 1.06), (0.26, 1.90), m, hull, slats=6, rot=(0, -math.pi / 2, 0))
    VP.slat_armour("rear_cage", (-half - 0.05, 0, 0.62), (3.20, 0.46), m, hull, rot=(0, 0, math.pi / 2))
    drums = empty("dressing_fuel_drums", parent=hull)
    for side in (-1, 1):
        cyl(f"fuel_drum_{side}", 0.26, 1.10, (-3.52, side * 0.80, 1.28), "Y", m["paint"], drums, seg=24, bevel=0.03)
        cyl(f"drum_band_{side}", 0.27, 0.04, (-3.52, side * 0.80, 1.28), "Y", m["dark"], drums, seg=24, lods=NEAR)


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", dome_rings(), mat=m["paint"], parent=turret, bevel=0.03)
    cyl("turret_ring_guard", 1.10, 0.10, (0, 0, -0.07), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # The broad cheek kit: flat panels round the front arc.
        for k in range(4):
            a = side * (0.30 + k * 0.26)
            x, y = 0.05 + 1.50 * math.cos(a), 1.50 * math.sin(a)
            VP.bolted_panel(f"cheek_panel_{s}_{k}", (x, y, 0.02), (0.26, 0.40, 0.52), m, turret, bolts=(1, 2),
                            bevel=0.04, rot=(0, 0, a), lods=VP.ALL)
        side_modules(v, turret, side, s, count=4, start=1.36, radius=1.40)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.55, side * 1.06, 0.64), m, turret, count=4, tube_radius=0.05,
                                 tube_length=0.22, elevation=0.30, spread=0.5, rot=(0, 0, side * 0.9))
        VP.stowage_box(f"rear_box_{s}", (-1.35, side * 0.80, 0.10), (0.42, 0.70, 0.40), m, turret,
                       rot=(0, 0, math.pi / 2))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-0.95, side * 0.70, 0.72), m, whip, height=2.2)
    for row in range(2):
        for k in range(4):
            y = (k - 1.5) * 0.36
            box(f"roof_era_{row}_{k}", (0.30, 0.30, 0.10), (0.88 - row * 0.32, y, 0.76 + row * 0.05), m["paint"],
                turret, bevel=0.02, rot=(0, 0.18, 0))
    VP.sight_housing("gunner_sight", (0.50, 0.60, 0.62), m, turret, size=(0.42, 0.34, 0.28))
    VP.cupola("commander_cupola", (-0.25, -0.58, 0.72), m, turret, radius=0.36, periscopes=4)
    VP.hatch("gunner_hatch", (-0.25, 0.60, 0.74), m, turret, radius=0.30)
    VP.slat_armour("bustle_bars", (-1.70, 0, 0.0), (2.20, 0.50), m, turret, rot=(0, 0, math.pi / 2))


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.45, -0.35), pitch=0.15)
    soviet_skirts(v, label="331", front=3.20, bars=(-2.45, 2.40))
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    gun_2a46(v, gun)
    cupola_mg(v, hmg, hmg_gun)
    v.head_out("commander", turret, -0.25, -0.58, 2.10)


def wreck(variant, v):
    soviet_wreck(v, ("gunner_hatch", "rear_box_"), seed=5)


run("t80", "russian_green", build, wreck, chip=1.0)
