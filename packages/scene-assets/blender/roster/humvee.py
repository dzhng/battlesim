"""M1151 HMMWV with its M2 in the O-GPK gunner's turret, from assets/references/humvee/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/humvee.py -- [--wreck]

What the photos settle: four 37-inch tyres on beadlock rims, wheelbase 3.3 m
and track 1.8 m; the long low hood over the fenders with its air louvre, the
upright grille of seven slots between round headlights, the steel bumper with
its D-rings; the near-upright split windscreen, four armoured doors with
small windows and outside hinges; the roof flat to behind the rear doors,
then the armoured rear deck sloping to the tail; the air intake snorkel on
the right windscreen pillar, mirrors on arms; the spare wheel on the rear,
tail lights at its corners; on the roof the O-GPK turret: a front shield with
the gun's slot, angled wing shields with armoured glass, side and rear
shields, the gunner standing in it behind the M2.

Built to the HMMWV's own frame (hull 4.9 x 2.2 x 2.0 m, eye 2.42 m, HMG
pivot 2.28 m; choices.md, slice 14), not the JLTV's it wore before.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_R = 0.47
WHEEL_W = 0.32
WHEEL_Y = 0.91
AXLES = (1.62, -1.68)
SILL = 0.72
FENDER_FOOT = 1.00
ROOF = 1.92
COWL = (0.95, 1.36)


def build(variant, v):
    m, hull = v.mats, v.hull
    body(v)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.24)
            # The double wishbone and the hub reduction behind the wheel.
            for j, z in enumerate((WHEEL_R - 0.10, WHEEL_R + 0.16)):
                box(f"a_arm_{s}_{k}_{j}", (0.30, 0.55, 0.06), (x, side * (WHEEL_Y - 0.42), z), m["dark"], hull,
                    lods=NEAR)
            cyl(f"coil_{s}_{k}", 0.08, 0.36, (x - 0.12, side * (WHEEL_Y - 0.48), WHEEL_R + 0.30), "Z", m["dark"],
                hull, seg=10, lods=FINE)
        VP.mudflap(f"mudflap_{s}", (-2.20, side * WHEEL_Y, FENDER_FOOT - 0.02), (0.36, 0.42), m, hull)
    mounts = rig(v.frame, v.root)
    turret, pitch, _, pivot = mounts["HMG"]
    ogpk(v, turret, pitch, pivot)
    # The gunner standing in the turret behind his M2 (photos: front,
    # detail); he turns with it.
    if not v.wreck:
        v.crew.append(("gunner", turret, (-0.48, 0.0, 1.76),
                       ((-0.22, 0.11, 2.53), (-0.22, -0.11, 2.53)),
                       ((-0.50, 0.13, 0.80), (-0.50, -0.13, 0.80))))


def body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    centre = 1.30
    fender_y = (v.width / 2 + 0.65) / 2 - 0.03
    fender_w = v.width / 2 - 0.65 - 0.04
    # Hood and fenders: the centre block reaches down between the wheels,
    # the fenders stand over them.
    hood = [(half - 0.06, None), (half, 1.16), (half - 0.15, 1.25), (COWL[0], COWL[1]), (COWL[0], None)]

    def outline(foot):
        return [(x, foot if z is None else z) for x, z in hood]

    prism("hood_centre", outline(0.68), centre, mat=m["paint"], parent=hull, bevel=0.03)
    for side, s in ((1, "L"), (-1, "R")):
        prism(f"front_fender_{s}", outline(FENDER_FOOT), fender_w, loc=(0, side * fender_y, 0), mat=m["paint"],
              parent=hull, bevel=0.035)
    # The cab: windscreen up from the cowl, roof, doors' foot at the sill.
    cab = [(COWL[0], SILL), (COWL[0], COWL[1]), (0.78, ROOF - 0.04), (0.72, ROOF), (-1.12, ROOF), (-1.12, SILL)]
    prism("cab", cab, v.width - 0.06, mat=m["paint"], parent=hull, bevel=0.035)
    # The armoured rear deck sloping from the roof to the tail, over the
    # rear wheels.
    deck = [(-1.12, None), (-1.12, ROOF - 0.02), (-2.20, 1.50), (-half, 1.42), (-half, None)]
    prism("rear_centre", [(x, 0.70 if z is None else z) for x, z in deck], centre, mat=m["paint"], parent=hull,
          bevel=0.03)
    for side, s in ((1, "L"), (-1, "R")):
        prism(f"rear_fender_{s}", [(x, FENDER_FOOT if z is None else z) for x, z in deck], fender_w,
              loc=(0, side * fender_y, 0), mat=m["paint"], parent=hull, bevel=0.035)
    # Front: grille slots between the headlights, the bumper with D-rings,
    # the hood's air louvre.
    box("grille_panel", (0.04, 1.00, 0.36), (half + 0.005, 0, 0.98), m["dark"], hull, bevel=0.01, lods=MID)
    for k in range(7):
        box(f"grille_slot_{k}", (0.03, 0.07, 0.28), (half + 0.02, -0.36 + k * 0.12, 0.98), m["black"], hull,
            lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        cyl(f"headlight_bezel_{s}", 0.11, 0.05, (half + 0.0, side * 0.66, 1.00), "X", m["dark"], hull, seg=16,
            lods=MID)
        cyl(f"headlight_{s}", 0.085, 0.02, (half + 0.03, side * 0.66, 1.00), "X", m["lamp"], hull, seg=16, lods=MID)
        cyl(f"marker_{s}", 0.05, 0.03, (half - 0.10, side * 0.95, 1.20), "X", m["lamp"], hull, seg=10,
            lods=FINE)
        VP.shackle(f"front_shackle_{s}", (half + 0.04, side * 0.45, 0.66), m, hull, size=0.10, rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.04, side * 0.95, 1.10), 0.045, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
    box("front_bumper", (0.10, 2.00, 0.18), (half - 0.02, 0, 0.68), m["dark"], hull, bevel=0.012)
    box("rear_bumper", (0.10, 1.90, 0.16), (-half + 0.03, 0, 0.74), m["dark"], hull, bevel=0.012)
    VP.tow_hook("pintle", (-half + 0.06, 0, 0.74), m, hull, size=0.12, rot=(0, 0, math.pi))
    slope = math.atan((COWL[1] - 1.25) / (half - 0.15 - COWL[0]))
    VP.grille("hood_louvre", (1.55, 0, 1.31), (0.40, 0.90), m, hull, slats=8, rot=(0, slope, 0))
    VP.grille("rear_side_louvre", (-1.70, 1.065, 1.28), (0.75, 0.16), m, hull, slats=10, rot=(-math.pi / 2, 0, 0))
    # Windscreen: two panes in a frame on the near-upright front.
    lean = math.atan((COWL[0] - 0.78) / (ROOF - 0.04 - COWL[1]))
    for side, s in ((1, "L"), (-1, "R")):
        box(f"windscreen_{s}", (0.03, 0.86, 0.42), (0.875, side * 0.48, 1.62), m["glass"], hull,
            rot=(0, -lean, 0), lods=MID)
        # Doors with their windows, hinges outside and a handle.
        for k, (front, rear) in enumerate(((0.90, 0.04), (0.00, -1.06))):
            mid = (front + rear) / 2
            y = side * (v.width / 2 - 0.02)
            box(f"door_{s}_{k}", (front - rear - 0.04, 0.04, ROOF - SILL - 0.20), (mid, y, (ROOF + SILL) / 2 - 0.06),
                m["paint"], hull, bevel=0.015, lods=MID)
            box(f"door_window_{s}_{k}", (0.50, 0.03, 0.34), (mid + 0.04, y + side * 0.02, 1.60), m["glass"], hull,
                lods=MID)
            box(f"window_frame_{s}_{k}", (0.58, 0.035, 0.42), (mid + 0.04, y + side * 0.012, 1.60), m["paint"], hull,
                bevel=0.012, lods=NEAR)
            for j, z in enumerate((1.05, 1.55)):
                box(f"door_hinge_{s}_{k}_{j}", (0.06, 0.04, 0.10), (front - 0.03, y + side * 0.025, z), m["steel"],
                    hull, lods=FINE)
            box(f"door_handle_{s}_{k}", (0.12, 0.03, 0.04), (rear + 0.12, y + side * 0.03, 1.22), m["steel"], hull,
                lods=FINE)
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_arm_{s}", (0.04, 0.22, 0.03), (0.86, side * 1.16, 1.50), m["dark"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.04, 0.14, 0.24), (0.86, side * 1.26, 1.55), m["dark"], mirror, bevel=0.01, lods=MID)
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-2.30, side * 0.80, 1.46), m, whip, height=2.6)
    # The air intake snorkel up the right windscreen pillar.
    cyl("snorkel", 0.07, 0.95, (0.98, -1.02, 1.42), "Z", m["paint"], hull, seg=12, lods=MID)
    cyl("snorkel_cap", 0.09, 0.10, (0.98, -1.02, 1.93), "Z", m["dark"], hull, seg=12, lods=MID)
    # The roof: the turret's fixed ring base; the amber beacon behind it.
    cyl("turret_ring_base", 0.66, 0.08, (0, 0, ROOF + 0.04), "Z", m["paint"], hull, seg=28, bevel=0.01)
    cyl("beacon", 0.07, 0.09, (-1.0, 0.70, ROOF + 0.045), "Z", m["tail"], hull, seg=12, lods=MID)
    # The spare wheel on the rear, left of centre.
    spare = empty("dressing_spare_wheel", parent=hull)
    cyl("spare_tyre", WHEEL_R * 0.97, 0.30, (-v.length / 2 - 0.13, 0.42, 1.05), "X", m["rubber"], spare, seg=28)
    cyl("spare_rim", 0.24, 0.06, (-v.length / 2 - 0.25, 0.42, 1.05), "X", m["paint"], spare, seg=18, lods=MID)
    box("spare_bracket", (0.12, 0.40, 0.08), (-v.length / 2 - 0.05, 0.42, 1.05), m["dark"], spare, lods=NEAR)


def ogpk(v, turret, pitch, pivot):
    """The O-GPK gunner's turret on its traversing ring: a front shield with
    the gun's slot, angled wing shields with armoured glass, side shields
    and two low rear shields; and the M2 with its ammunition can and the
    gunner's spade grips."""
    m = v.mats
    foot = ROOF + 0.08 - pivot.z
    cyl("ogpk_ring", 0.62, 0.07, (0, 0, foot + 0.035), "Z", m["dark"], turret, seg=28, lods=MID)
    top = foot + 0.72
    height = top - foot

    def shield(name, x, y, length, yaw, tall=height, window=False):
        box(name, (0.05, length, tall), (x, y, foot + tall / 2), m["paint"], turret, rot=(0, 0, yaw), bevel=0.012)
        if window:
            dx = math.cos(yaw) * 0.03
            dy = math.sin(yaw) * 0.03
            box(f"{name}_glass", (0.02, length * 0.6, tall * 0.35), (x + dx, y + dy, foot + tall * 0.68), m["glass"],
                turret, rot=(0, 0, yaw), lods=MID)
            box(f"{name}_frame", (0.03, length * 0.7, tall * 0.45), (x + dx * 0.6, y + dy * 0.6, foot + tall * 0.68),
                m["dark"], turret, rot=(0, 0, yaw), lods=NEAR)

    for side, s in ((1, "L"), (-1, "R")):
        shield(f"ogpk_front_{s}", 0.58, side * 0.25, 0.32, 0.0)
        shield(f"ogpk_wing_{s}", 0.42, side * 0.52, 0.42, side * 0.85, window=True)
        shield(f"ogpk_side_{s}", 0.02, side * 0.64, 0.52, math.pi / 2, window=True)
        shield(f"ogpk_rear_{s}", -0.46, side * 0.40, 0.40, side * -0.75, tall=height * 0.6)
    box("ogpk_front_lower", (0.05, 0.22, 0.22), (0.58, 0, foot + 0.11), m["paint"], turret, bevel=0.01)
    box("ogpk_front_upper", (0.05, 0.22, 0.12), (0.58, 0, top - 0.06), m["paint"], turret, bevel=0.01, lods=MID)
    for k in range(6):
        cyl(f"ogpk_bolt_{k}", 0.018, 0.02, (0.61, -0.30 + k * 0.12, foot + 0.12), "X", m["steel"], turret, seg=6,
            lods=FINE)
    # The pintle and cradle up to the gun.
    lift = v.frame["mounts"][0]["muzzle_m"][2]
    cyl("pintle_post", 0.04, lift - 0.05 - foot, (0.10, 0, (foot + lift - 0.05) / 2), "Z", m["dark"], turret, seg=10)
    box("pintle_cradle", (0.20, 0.14, 0.06), (0.10, 0, lift - 0.06), m["dark"], turret, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("m2_receiver", (0.52, 0.13, 0.15), (0.10, 0, 0), m["dark"], pitch, bevel=0.012)
    cyl("m2_barrel", 0.024, reach - 0.40, ((reach + 0.38) / 2, 0, 0), "X", m["steel"], pitch, seg=10)
    cyl("m2_jacket", 0.040, 0.22, (0.47, 0, 0), "X", m["dark"], pitch, seg=12, lods=MID)
    cyl("m2_flash_hider", 0.034, 0.08, (reach - 0.04, 0, 0), "X", m["steel"], pitch, seg=10, r2=0.026, lods=NEAR)
    for side in (-1, 1):
        cyl(f"m2_grip_{side}", 0.018, 0.12, (-0.20, side * 0.07, -0.02), "Z", m["black"], pitch, seg=8, lods=NEAR)
    box("m2_ammo_can", (0.28, 0.12, 0.20), (0.10, 0.17, -0.06), m["dark"], pitch, bevel=0.01, lods=MID)


def wreck(variant, v):
    """The HMMWV after its fire: the front right wheel blown off and the nose
    down on that corner, the right rear door torn off and lying by it, the
    hood buckled up and the body warped, the turret's front shield bent
    back, the spare burnt off its bracket, torn plate on the ground. Whole."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "door_R_1", "door_window_R_1", "window_frame_R_1", "door_hinge_R_1", "door_handle_R_1")
    plate("door_lying", [(-0.50, -0.40), (0.50, -0.42), (0.48, 0.40), (-0.50, 0.38)], 0.04, (-0.55, -1.25, 0.04),
          (0.02, 0.03, 0.3), m["paint"], v.root, seed=51)
    bend(parts("ogpk_front_", "ogpk_wing_R"), (0.58, 0, ROOF + 0.25), (0, 1, 0), (0, 0, 1), -0.35)
    shell = parts("hood_", "front_fender_", "cab", "rear_")
    densify(shell, scale=1.5)
    warp(shell, heat(0.025, 0.7, seed=8.0), dent((1.6, 0.0, 1.30), 0.6, -0.12, (0.0, 0.0, 1.0)))
    for k, (loc, rot, size) in enumerate((((1.9, 1.45, 0.03), (0.03, 0.04, 0.9), 0.26),
                                          ((-1.9, -1.35, 0.03), (-0.04, 0.02, 2.2), 0.24))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.root, curl=0.12, seed=61 + k)
    v.root.rotation_euler = (0.05, 0.05, 0)
    v.root.location.z -= 0.05
    rest_on_ground(0.004)


run("humvee", "us_desert_tan", build, wreck)
