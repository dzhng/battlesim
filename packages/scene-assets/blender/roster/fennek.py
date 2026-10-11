"""Fennek reconnaissance vehicle, from assets/references/fennek/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/fennek.py -- [--wreck]

What the photos settle: four black road-tread tyres on green rims with a
ring of wheel nuts, the front axle near the nose and the rear one well
forward of the long tail; a low armoured body, its upper sides leaning in
under a flat roof and bolted all over, flared plates round the arches; the
blunt nose with the lights in boxes at its corners, tow points and the
licence plate; the raked windscreens, mirrors on
frames; the big side doors; the long sloping rear deck; the remote weapon
station with its heavy machine gun on the roof, and the BAA sensor head on
its mast, folded down on the rear roof.

Built to the catalog frame (hull 5.58 x 2.55 x 2.29 m, HMG pivot 2.08 m):
the photos put the roof near 1.85 m. Nothing here moves the frame: the roof
is drawn at its real height, and the folded sensor head reaches the box's
top.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_R = 0.54
WHEEL_W = 0.36
WHEEL_Y = 1.05
AXLES = (1.95, -1.20)
BELLY = 0.42
WAIST = 1.20
ROOF = 1.84
NOSE = 2.79
# The upper sides, leaning in from over the waist to the roof, as (y, z) at
# their foot and top (`vehicle_parts.on_side`).
UPPER_SIDE = ((1.17, 1.40), (0.98, ROOF))


def build(variant, v):
    m, hull = v.mats, v.hull
    # A low body: the narrow belly, the waist at its widest, the leaning
    # upper sides to the roof; the nose blunt, the windscreen raked, the
    # tail long.
    loft("fennek_body", [(BELLY, VP.hull_plan(-2.60, 2.55, 0.85, 0.30)),
                         (WAIST, VP.hull_plan(-2.79, NOSE, 1.20, 0.40)),
                         (1.40, VP.hull_plan(-2.75, 2.70, UPPER_SIDE[0][0], 0.40)),
                         (ROOF, VP.hull_plan(-1.60, 1.25, UPPER_SIDE[1][0], 0.25))], mat=m["paint"], parent=hull, bevel=0.05)
    box("rear_deck", (1.20, 1.90, 0.10), (-2.15, 0, 1.55), m["paint"], hull, rot=(0, 0.20, 0), bevel=0.03)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.30, ctis=True, hub_bolts=10)
            box(f"a_arm_{s}_{k}", (0.24, 0.40, 0.08), (x, side * (WHEEL_Y - 0.36), WHEEL_R - 0.05), m["dark"], hull,
                lods=NEAR)
            arch = [(x - 0.70, 0.62), (x - 0.48, 1.12), (x + 0.48, 1.12), (x + 0.70, 0.62), (x + 0.60, 0.58),
                    (x + 0.40, 1.03), (x - 0.40, 1.03), (x - 0.60, 0.58)]
            prism(f"arch_{s}_{k}", arch, 0.08, loc=(0, side * 1.20, 0), mat=m["dark"], parent=hull, bevel=0.015,
                  lods=MID)
    fittings(v)
    station, pitch, _, pivot = rig(v.frame, v.root)["HMG"]
    flw100(v, station, pitch, pivot)


def fittings(v):
    m, hull = v.mats, v.hull
    rake = math.atan((2.70 - 1.25) / (ROOF - 1.40))
    # Windscreens, the nose's lamp boxes and tow points.
    for k, y in enumerate((0.48, -0.48)):
        box(f"windscreen_{k}", (0.02, 0.80, 0.48), (2.00, y, 1.62), m["glass"], hull, rot=(0, -rake, 0),
            lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"lamp_box_{s}", (0.18, 0.30, 0.22), (NOSE - 0.12, side * 0.85, 1.05), m["dark"], hull, bevel=0.02,
            lods=MID)
        cyl(f"headlight_{s}", 0.07, 0.02, (NOSE - 0.02, side * 0.85, 1.05), "X", m["lamp"], hull, seg=14, lods=MID)
        VP.tow_hook(f"front_tow_{s}", (NOSE - 0.12, side * 0.50, 0.68), m, hull, size=0.10)
        VP.tow_hook(f"rear_tow_{s}", (-2.70, side * 0.55, 0.70), m, hull, size=0.10, rot=(0, 0, math.pi))
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_frame_{s}", (0.04, 0.28, 0.04), (1.95, side * 1.25, 1.55), m["black"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.06, 0.14, 0.26), (1.95, side * 1.38, 1.55), m["black"], mirror, bevel=0.01, lods=MID)
        # The door on the leaning upper side, the window strip over it.
        loc, rot = VP.on_side(0.45, 1.52, side, *UPPER_SIDE)
        VP.bolted_panel(f"side_door_{s}", loc, (1.10, 0.62, 0.025), m, hull, bolts=(4, 3), rot=rot, bevel=0.012)
        loc, rot = VP.on_side(0.30, 1.70, side, *UPPER_SIDE, proud=0.035, standing=True)
        box(f"side_window_{s}", (1.25, 0.02, 0.10), loc, m["glass"], hull, rot=rot, lods=MID)
        loc, rot = VP.on_side(0.10, 1.36, side, *UPPER_SIDE, proud=0.045, standing=True)
        box(f"door_handle_{s}", (0.14, 0.04, 0.04), loc, m["steel"], hull, rot=rot, lods=FINE)
        for k in range(10):
            x = 2.30 - k * 0.48
            cyl(f"waist_bolt_{s}_{k}", 0.024, 0.03, (x, side * 1.19, 1.30), "Y", m["steel"], hull, seg=6, lods=FINE)
        VP.light_with_guard(f"tail_light_{s}", (-2.78, side * 0.95, 1.10), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-2.30, side * 0.75, 1.62), m, whip, height=2.4)
    box("front_plate", (0.02, 0.50, 0.12), (NOSE + 0.005, 0.30, 0.82), m["marking"], hull, lods=MID)
    # The crew's roof hatches just behind the station's ring, clear of what
    # it swings round, and the folded BAA sensor mast on the rear roof.
    ring = v.frame["mounts"][0]["pivot_m"][0]
    VP.hatch("commander_hatch", (ring - 0.50, 0.55, ROOF), m, hull, radius=0.28)
    VP.hatch("driver_hatch", (ring - 0.60, -0.60, ROOF), m, hull, size=(0.55, 0.50))
    cyl("mast_foot", 0.12, 0.20, (-1.05, -0.45, ROOF + 0.10), "Z", m["dark"], hull, seg=14, lods=MID)
    cyl("mast_column", 0.07, 0.95, (-1.05, -0.45, ROOF + 0.24), "X", m["dark"], hull, seg=12)
    box("sensor_head", (0.36, 0.34, 0.28), (-0.62, -0.45, ROOF + 0.30), m["paint"], hull, bevel=0.03)
    box("sensor_window", (0.01, 0.22, 0.14), (-0.435, -0.45, ROOF + 0.32), m["glass"], hull, lods=MID)
    tarp = empty("dressing_rear_stowage", parent=hull)
    VP.stowage_box("rear_box", (-2.25, 0.45, 1.60), (0.55, 0.60, 0.30), dict(m, paint=m["dark"]), tarp,
                   rot=(0, 0.20, 0))
    VP.jerrycan("rear_jerrycan", (-2.25, -0.45, 1.60), dict(m, paint=m["dark"]), tarp, size=(0.47, 0.165, 0.345),
                rot=(0, 0.20, 0))


def flw100(v, station, pitch, pivot):
    """The remote station on its roof ring: a low turning base, the cradle,
    the heavy machine gun, the sight on its left and the ammunition box."""
    m = v.mats
    below = ROOF - pivot.z
    cyl("station_ring", 0.32, 0.05, (0, 0, below + 0.025), "Z", m["dark"], station, seg=20, lods=MID)
    box("station_base", (0.55, 0.50, -below - 0.05), (-0.05, 0, below / 2), m["paint"], station, bevel=0.025)
    for side in (-1, 1):
        box(f"station_cradle_{side}", (0.36, 0.05, 0.28), (0.02, side * 0.16, 0.0), m["paint"], station,
            bevel=0.012)
    VP.sight_housing("station_sight", (0.08, 0.30, -0.10), m, pitch, size=(0.30, 0.18, 0.22))
    box("station_ammo", (0.30, 0.14, 0.24), (-0.02, -0.28, -0.08), m["paint"], pitch, bevel=0.012, lods=MID)
    VP.browning_m2(pitch, v.frame["mounts"][0]["muzzle_m"][0], m)


def wreck(variant, v):
    """The Fennek after its fire: the front left wheel blown off and the
    nose down on it, the left door torn off and thrown onto the roof, the
    sensor head fallen onto the rear deck, the rear stowage burnt away, the body warped and dented,
    torn plate on the ground. Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "side_door_L", "sensor_", "dressing_rear_stowage", "rear_box", "rear_jerrycan")
    # The door lies on the roof behind the weapon station's ring.
    behind = v.frame["mounts"][0]["pivot_m"][0] - 0.95
    plate("door_lying", [(-0.55, -0.31), (0.55, -0.32), (0.55, 0.31), (-0.55, 0.30)], 0.04, (behind, 0.3, ROOF + 0.06),
          (0.02, 0.03, 0.0), m["paint"], v.root, seed=211)
    plate("sensor_lying", [(-0.18, -0.17), (0.18, -0.17), (0.18, 0.17), (-0.18, 0.17)], 0.25, (-2.25, -0.45, 1.75),
          (0.2, 0.1, 0.5), m["paint"], v.root, seed=212)
    shell = parts("fennek_body", "rear_deck")
    densify(shell, scale=1.5)
    warp(shell, heat(0.025, 0.7, seed=26.0), dent((0.8, -1.15, 1.40), 0.5, 0.12, (0, 1, -0.1)))
    for k, (loc, rot, size) in enumerate((((0.35, -1.35, 0.03), (0.03, 0.04, 0.9), 0.19),
                                          ((0.4, 1.35, 0.03), (-0.04, 0.02, 2.2), 0.18))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.root, curl=0.12,
              seed=221 + k)
    v.root.rotation_euler = (-0.05, 0.05, 0)
    v.root.location.z -= 0.05
    rest_on_ground(0.004)


run("fennek", "german_three_tone", build, wreck, chip=0.6)
