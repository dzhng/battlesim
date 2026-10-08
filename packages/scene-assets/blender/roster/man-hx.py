"""MAN HX cargo truck, from assets/references/man-hx/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/man-hx.py -- [--wreck]

What the photos settle: black bar-tread tyres on painted rims; the tall,
flat-fronted tilt cab over the first axle: the massive bumper with its
lights in grilled recesses, the tow jaw and the steps, the two-pane
windscreen with its wipers, the raised roof box with its vents and the
amber beacon, the big mirrors on their arms, the door with its window and
the ladder, the air intake's grille panel behind the cab; the cargo body
under a tarp, tied down along its foot, with the tool lockers and fuel
tank under it. Cab interior as the dark glass shows it: the dash and seat
backs.

The photos mix 4x4, 6x6 and 8x8 HX trucks (choices.md, slice 08): the
catalog frame (hull 10.34 x 2.5 x 3.3 m) is the HX77 8x8's length, so this
draws eight wheels, two axles under the cab and two under the body.
Nothing here moves the frame.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, prism  # noqa: E402
from vehicle_export import run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

AXLES = (3.91, 2.29, -2.68, -4.05)
WHEEL_R = 0.60
WHEEL_W = 0.40
WHEEL_Y = 1.03
RAIL_Z = 1.05
BED_FLOOR = 1.45
CAB_REAR = 2.95
CAB_FLOOR = 1.32
NOSE = 5.168
ROOF = 3.05


def build(variant, v):
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.30, tread="bar", hub_bolts=10)
            box(f"axle_{s}_{k}", (0.18, 0.55, 0.18), (x, side * 0.55, WHEEL_R), m["dark"], hull, lods=NEAR)
        box(f"chassis_rail_{s}", (9.60, 0.14, 0.26), (-0.10, side * 0.45, RAIL_Z), m["dark"], hull, bevel=0.01,
            lods=MID)
        VP.mudflap(f"mudflap_{s}", (-4.80, side * WHEEL_Y, BED_FLOOR - 0.15), (0.42, 0.55), m, hull)
    cab(v)
    body(v)


def cab(v):
    m, hull = v.mats, v.hull
    width = v.width - 0.06
    profile = [(CAB_REAR, CAB_FLOOR), (NOSE - 0.10, CAB_FLOOR), (NOSE - 0.14, 2.20), (NOSE - 0.30, ROOF - 0.05),
               (NOSE - 0.40, ROOF), (CAB_REAR, ROOF)]
    prism("cab", profile, width, mat=m["paint"], parent=hull, bevel=0.06)
    box("roof_box", (1.40, width - 0.30, 0.16), (CAB_REAR + 0.85, 0, ROOF + 0.07), m["paint"], hull, bevel=0.04)
    VP.grille("roof_vent", (CAB_REAR + 1.45, 0, ROOF + 0.15), (0.20, 0.80), m, hull, slats=4)
    cyl("beacon", 0.08, 0.12, (CAB_REAR + 0.20, -0.95, ROOF + 0.06), "Z", m["tail"], hull, seg=12, lods=MID)
    # The bumper with its lights in grilled recesses, the tow jaw, steps;
    # the engine block down between the wheels.
    prism("engine_block", [(CAB_REAR + 0.20, 0.85), (NOSE - 0.30, 0.85), (NOSE - 0.30, CAB_FLOOR),
                           (CAB_REAR + 0.20, CAB_FLOOR)], 1.20, mat=m["paint"], parent=hull, bevel=0.03)
    box("bumper", (0.45, v.width - 0.04, 0.48), (NOSE - 0.22, 0, 1.08), m["paint"], hull, bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"lamp_recess_{s}", (0.04, 0.42, 0.24), (NOSE + 0.005, side * 0.85, 1.08), m["black"], hull, lods=MID)
        for k, y in enumerate((0.72, 0.98)):
            cyl(f"headlight_{s}_{k}", 0.06, 0.02, (NOSE + 0.02, side * y, 1.10), "X", m["lamp"], hull, seg=12,
                lods=MID)
        box(f"indicator_{s}", (0.03, 0.18, 0.14), (NOSE - 0.12, side * 1.05, 1.70), m["tail"], hull, lods=MID)
        box(f"step_{s}", (0.30, 0.20, 0.04), (NOSE - 0.55, side * 1.10, 0.70), m["steel"], hull, lods=MID)
        # Door, window, handle, ladder; the big mirror on its arm.
        y = side * (width / 2 + 0.005)
        VP.weld_line(f"door_seam_{s}", [(4.75, y, CAB_FLOOR + 0.05), (4.75, y, ROOF - 0.15), (3.65, y, ROOF - 0.15),
                                       (3.65, y, CAB_FLOOR + 0.05)], m, hull, radius=0.012)
        box(f"door_window_{s}", (0.80, 0.03, 0.60), (4.20, y, 2.50), m["glass"], hull, lods=MID)
        box(f"door_handle_{s}", (0.14, 0.04, 0.04), (3.80, y + side * 0.02, 2.05), m["steel"], hull, lods=FINE)
        for k in range(4):
            box(f"ladder_rung_{s}_{k}", (0.30, 0.04, 0.03), (3.45, side * (width / 2 + 0.03), 1.45 + k * 0.30),
                m["steel"], hull, lods=FINE)
        VP.grille(f"intake_{s}", (CAB_REAR + 0.20, side * (width / 2 + 0.01), 2.20), (0.40, 0.80), m, hull, slats=8,
                  rot=(-side * math.pi / 2, 0, 0))
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_arm_{s}", (0.04, 0.26, 0.04), (4.95, side * 1.32, 2.55), m["black"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.06, 0.16, 0.45), (4.95, side * 1.46, 2.40), m["black"], mirror, bevel=0.015, lods=MID)
    # The windscreen and wipers; behind it, the dash and seat backs.
    rake = math.atan(0.16 / (ROOF - 0.05 - 2.20))
    for side in (-1, 1):
        box(f"windscreen_{side}", (0.03, 1.00, 0.72), (NOSE - 0.21, side * 0.56, 2.58), m["glass"], hull,
            rot=(0, -rake, 0), lods=MID)
        box(f"wiper_{side}", (0.02, 0.03, 0.55), (NOSE - 0.17, side * 0.50, 2.45), m["black"], hull,
            rot=(side * 0.5, -rake, 0), lods=FINE)
        box(f"seat_back_{side}", (0.12, 0.50, 0.60), (3.45, side * 0.55, 2.05), m["black"], hull, lods=NEAR)
    box("dash", (0.40, 2.00, 0.22), (4.75, 0, 2.05), m["black"], hull, lods=NEAR)
    box("plate", (0.02, 0.50, 0.12), (NOSE - 0.07, 0, 1.55), m["marking"], hull, lods=MID)


def body(v):
    m, hull = v.mats, v.hull
    length = 7.10
    centre = 2.75 - length / 2
    VP.cargo_bed("cargo_body", (centre, 0, BED_FLOOR), (length, 2.46, 0.62), m, hull, stakes=6, tarp=1.22)
    for side, s in ((1, "L"), (-1, "R")):
        VP.stowage_box(f"locker_{s}", (0.10, side * 0.95, 0.75), (1.20, 0.45, 0.55), m, hull,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-5.15, side * 1.00, 1.20), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
    VP.fuel_tank("fuel_tank", (-1.20, 0.95, 1.00), 1.30, 0.30, m, hull)
    box("rear_underrun", (0.12, 2.20, 0.14), (-5.05, 0, 0.70), m["dark"], hull, bevel=0.012)
    VP.tow_hook("pintle", (-5.00, 0, 0.92), m, hull, size=0.10, rot=(0, 0, math.pi))
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (CAB_REAR + 0.15, side * 1.05, ROOF), m, whip, height=2.0)


def wreck(variant, v):
    """The HX after its fire: the front right wheel blown off and the cab
    down on that corner, the tarp burnt off its bows, the bed's sides
    sagging, the cab buckled and a mirror gone, a locker torn off and
    thrown into the bed. Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "cargo_body_canvas", "cargo_body_tie_", "locker_R", "dressing_mirror_R", "mirror_arm_R",
           "mirror_R")
    VP.stowage_box("spilled_locker", (-1.0, -0.4, BED_FLOOR), (1.20, 0.45, 0.55), dict(m, paint=m["dark"]), v.hull,
                   rot=(0.05, 0.03, 0.25))
    shell = parts("cab", "cargo_body_side", "cargo_body_end", "bumper")
    densify(shell, scale=1.5)
    warp(shell, heat(0.025, 0.7, seed=32.0), dent((4.6, -0.6, 2.4), 0.6, 0.12, (-1, 0, 0)))
    plate("debris_0", [(-0.3, -0.2), (0.27, -0.21), (0.3, 0.15), (-0.21, 0.24)], 0.03, (-3.0, 0.5, BED_FLOOR + 0.03),
          (0.03, 0.04, 0.9), m["paint"], v.root, curl=0.12, seed=261)
    v.root.rotation_euler = (0.02, 0.015, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


run("man-hx", "german_three_tone", build, wreck, chip=0.6)
