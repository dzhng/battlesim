"""Ural-4320 cargo truck, from assets/references/ural/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/ural.py -- [--wreck]

What the photos and Mick Bell's drawing settle: six black bar-tread tyres on
painted rims with big hubs, the front axle under the bonnet and the rear
bogie under the body; the long bonnet sloping to the upright grille between
the headlights set in the rounded front wings, the steel bumper; the
rounded cab with its two-pane windscreen and door windows, mirrors on arms;
behind the cab the exhaust and air intake stacks and the spare wheel; the
fuel tank and battery box under the cab's side; the cargo body with its
board sides and the tarp over its bows, tied down along its foot; tail
lights and the hitch on the rear. Cab interior as the dark glass shows it:
the dash and seat backs.

Built to the catalog frame (hull 7.37 x 2.5 x 3.0 m): nothing here moves
it. The axles are the side photo's (2.53, -0.95, -2.30), not the legacy
helper's (2.48, -1.30, -2.54).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, prism  # noqa: E402
from vehicle_export import run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

AXLES = (2.53, -0.95, -2.30)
WHEEL_R = 0.60
WHEEL_W = 0.38
WHEEL_Y = 1.03
RAIL_Z = 1.00
BED_FLOOR = 1.25
CAB_REAR = 0.90
CAB_FRONT = 2.13
ROOF = 2.70
NOSE = 3.683


def build(variant, v):
    chassis(v)
    cab(v)
    body(v)


def chassis(v):
    """The wheels, axles, rails and mudflaps (the BM-21 stands on them too)."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.30, tread="bar", hub_bolts=8)
            cyl(f"hub_{s}_{k}", 0.12, 0.10, (x, side * (WHEEL_Y + WHEEL_W / 2 + 0.05), WHEEL_R), "Y", m["dark"],
                hull, seg=12, bevel=0.015, lods=MID)
            box(f"axle_{s}_{k}", (0.18, 0.55, 0.18), (x, side * 0.55, WHEEL_R), m["dark"], hull, lods=NEAR)
        box(f"chassis_rail_{s}", (6.90, 0.14, 0.24), (0.0, side * 0.45, RAIL_Z), m["dark"], hull, bevel=0.01,
            lods=MID)
        VP.mudflap(f"mudflap_{s}", (-3.00, side * WHEEL_Y, BED_FLOOR - 0.10), (0.40, 0.50), m, hull)


def cab(v):
    """The cab, bonnet and wings, and behind the cab its stacks, spare wheel,
    fuel tank and battery box (the BM-21's Ural-375D shares them)."""
    m, hull = v.mats, v.hull
    width = 2.10
    # The rounded cab and the long bonnet.
    profile = [(CAB_REAR, 0.92), (CAB_FRONT, 0.92), (CAB_FRONT, 1.95), (CAB_FRONT - 0.12, ROOF - 0.08),
               (CAB_FRONT - 0.25, ROOF), (CAB_REAR + 0.10, ROOF), (CAB_REAR, ROOF - 0.10)]
    prism("cab", profile, width, mat=m["paint"], parent=hull, bevel=0.08)
    bonnet = [(CAB_FRONT, 1.20), (NOSE - 0.12, 1.20), (NOSE - 0.12, 1.85), (NOSE - 0.30, 1.98),
              (CAB_FRONT, 2.10)]
    prism("bonnet", bonnet, 1.20, mat=m["paint"], parent=hull, bevel=0.06)
    box("grille", (0.04, 0.95, 0.60), (NOSE - 0.10, 0, 1.55), m["dark"], hull, bevel=0.01, lods=MID)
    for j in range(8):
        box(f"grille_slot_{j}", (0.03, 0.07, 0.50), (NOSE - 0.08, -0.40 + j * 0.115, 1.55), m["black"], hull,
            lods=NEAR)
    box("bumper", (0.14, 2.30, 0.20), (NOSE - 0.07, 0, 1.00), m["paint"], hull, bevel=0.02)
    for side, s in ((1, "L"), (-1, "R")):
        # The rounded front wing over the wheel, the headlight in its nose.
        x = AXLES[0]
        wing = [(x - 0.70, 0.95), (x - 0.62, 1.30), (x - 0.30, 1.55), (x + 0.30, 1.58), (x + 0.75, 1.50),
                (NOSE - 0.12, 1.45), (NOSE - 0.12, 1.20), (x + 0.70, 1.30), (x + 0.30, 1.42), (x - 0.30, 1.40),
                (x - 0.55, 1.20), (x - 0.62, 0.95)]
        prism(f"wing_{s}", wing, 0.50, loc=(0, side * 1.00, 0), mat=m["paint"], parent=hull, bevel=0.03)
        cyl(f"headlight_{s}", 0.09, 0.05, (NOSE - 0.10, side * 0.95, 1.35), "X", m["lamp"], hull, seg=16, lods=MID)
        cyl(f"headlight_rim_{s}", 0.11, 0.04, (NOSE - 0.13, side * 0.95, 1.35), "X", m["dark"], hull, seg=16,
            lods=MID)
        VP.tow_hook(f"front_tow_{s}", (NOSE - 0.02, side * 0.55, 0.92), m, hull, size=0.10)
        # Door, window, handle; the step; the mirror on its arm.
        y = side * (width / 2 + 0.005)
        VP.weld_line(f"door_seam_{s}", [(2.00, y, 0.95), (2.00, y, ROOF - 0.15), (1.05, y, ROOF - 0.15),
                                       (1.05, y, 0.95)], m, hull, radius=0.012)
        box(f"door_window_{s}", (0.68, 0.03, 0.46), (1.55, y, 2.28), m["glass"], hull, lods=MID)
        box(f"door_handle_{s}", (0.14, 0.04, 0.04), (1.20, y + side * 0.02, 1.80), m["steel"], hull, lods=FINE)
        box(f"step_{s}", (0.30, 0.20, 0.04), (1.40, side * 1.05, 0.62), m["steel"], hull, lods=MID)
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_arm_{s}", (0.04, 0.22, 0.04), (CAB_FRONT, side * 1.16, 2.20), m["black"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.05, 0.14, 0.30), (CAB_FRONT, side * 1.27, 2.15), m["black"], mirror, bevel=0.012,
            lods=MID)
    for side in (-1, 1):
        box(f"windscreen_{side}", (0.03, 0.88, 0.50), (CAB_FRONT - 0.04, side * 0.48, 2.28), m["glass"], hull,
            rot=(0, -0.12, 0), lods=MID)
        box(f"seat_back_{side}", (0.12, 0.45, 0.55), (1.05, side * 0.50, 1.70), m["black"], hull, lods=NEAR)
    box("dash", (0.35, 1.90, 0.20), (1.95, 0, 1.75), m["black"], hull, lods=NEAR)
    cyl("cab_marker", 0.04, 0.05, (CAB_FRONT - 0.30, 0.0, ROOF + 0.02), "Z", m["tail"], hull, seg=10, lods=FINE)
    # Behind the cab: exhaust and intake stacks, the spare wheel.
    cyl("exhaust_stack", 0.09, 1.70, (0.72, -0.75, 1.95), "Z", m["steel"], hull, seg=12)
    cyl("exhaust_cap", 0.10, 0.10, (0.72, -0.75, 2.85), "Z", m["dark"], hull, seg=12, lods=MID)
    cyl("intake_stack", 0.08, 1.60, (0.72, -0.55, 1.95), "Z", m["paint"], hull, seg=12, lods=MID)
    cyl("spare_tyre", WHEEL_R * 0.97, WHEEL_W, (0.62, 0.35, 1.92), "X", m["rubber"], hull, seg=28)
    cyl("spare_rim", 0.30, 0.06, (0.82, 0.35, 1.92), "X", m["paint"], hull, seg=18, lods=MID)
    # Under the cab's side: the fuel tank, the battery box.
    VP.fuel_tank("fuel_tank", (0.55, -0.95, 0.85), 0.90, 0.26, m, hull)
    VP.stowage_box("battery_box", (0.55, 0.95, 0.60), (0.70, 0.40, 0.40), dict(m, paint=m["dark"]), hull)


def body(v):
    m, hull = v.mats, v.hull
    length = 4.10
    centre = 0.42 - length / 2
    VP.cargo_bed("cargo_body", (centre, 0, BED_FLOOR), (length, 2.46, 0.40), m, hull, stakes=5, tarp=1.30)
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{s}", (-3.65, side * 1.00, 1.05), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (CAB_REAR + 0.05, side * 0.90, ROOF - 0.05), m, whip, height=1.8)
    box("rear_bumper", (0.12, 2.00, 0.14), (-3.55, 0, 0.88), m["dark"], hull, bevel=0.012)
    VP.tow_hook("hitch", (-3.55, 0, 0.88), m, hull, size=0.10, rot=(0, 0, math.pi))


def wreck(variant, v):
    """The Ural after its fire: the front right wheel blown off and the
    bonnet down on that corner, the tarp burnt off its bows, the bonnet
    buckled up, the cab warped, the spare burnt and fallen into the bed, a mirror gone.
    Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "hub_R_0", "cargo_body_canvas", "cargo_body_tie_", "spare_", "dressing_mirror_R",
           "mirror_arm_R", "mirror_R")
    plate("spare_lying", [(-0.55, -0.55), (0.55, -0.55), (0.55, 0.55), (-0.55, 0.55)], 0.30, (-1.6, 0.3, BED_FLOOR + 0.17),
          (0.0, 0.0, 0.0), m["rubber"], v.root, seed=271)
    shell = parts("cab", "bonnet", "wing_", "cargo_body_side")
    densify(shell, scale=1.5)
    warp(shell, heat(0.025, 0.7, seed=34.0), dent((3.0, 0.0, 1.95), 0.5, -0.12, (0, 0, 1)))
    plate("debris_0", [(-0.3, -0.2), (0.27, -0.21), (0.3, 0.15), (-0.21, 0.24)], 0.03, (-2.5, -0.5, BED_FLOOR + 0.03),
          (0.03, 0.04, 0.9), m["paint"], v.root, curl=0.12, seed=281)
    v.root.rotation_euler = (0.03, 0.0, 0)
    v.root.location.z -= 0.04
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("ural", "russian_green", build, wreck, chip=0.6)
