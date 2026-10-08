"""Tigr-M, from assets/references/tigr/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/tigr.py -- [--wreck]

What the photos settle: four big black bar-tread tyres on dark rims with
deep hubs, the wheelbase long and the overhangs short; the armoured body one
box from the windscreen to the tail, its sides plain under a roof rail; the
hood with its raised scoop sloping to the grille's two mesh panels between
round headlights and amber markers, the tubular bull bar and bumper; the
flared arches; a split, near-upright windscreen, door windows and small
side windows each with a round gun-port cover; mirrors on arms; the rear
door carrying the spare wheel; the roof hatch. The walk-around vehicle is
unarmed; the roster's Tigr carries a Kord on the roof hatch ring, its gunner
standing in the hatch (slice 16 crew rule; reference gap noted).

Built to the catalog frame (hull 5.67 x 2.2 x 2.0 m, HMG pivot 1.82 m): the
photos put the roof near 2.45 m. Nothing here moves the frame: the wheels
are drawn true and the cab lower, so the body reads squat (choices.md,
slice 16).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_R = 0.56
WHEEL_W = 0.32
WHEEL_Y = 0.90
AXLES = (1.90, -1.85)
SILL = 0.52
ROOF = 1.95
COWL = (1.35, 1.40)
NOSE = 2.62


def build(variant, v):
    m, hull = v.mats, v.hull
    body(v)
    dark = dict(m, paint=m["dark"])
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, dark, hull,
                          rim_radius=0.26, tread="bar", hub_bolts=8)
            cyl(f"hub_{s}_{k}", 0.11, 0.10, (x, side * (WHEEL_Y + WHEEL_W / 2 + 0.06), WHEEL_R), "Y", m["dark"], hull,
                seg=12, bevel=0.015, lods=MID)
            box(f"a_arm_{s}_{k}", (0.26, 0.45, 0.08), (x, side * (WHEEL_Y - 0.38), WHEEL_R - 0.06), m["dark"], hull,
                lods=NEAR)
        VP.mudflap(f"mudflap_{s}", (-2.55, side * WHEEL_Y, SILL + 0.45), (0.34, 0.40), m, hull)
    station, pitch, _, pivot = rig(v.frame, v.root)["HMG"]
    kord(v, station, pitch, pivot)


def body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    width = v.width - 0.10
    # The one-box body from the cowl to the tail, and the hood ahead of it.
    cab = [(COWL[0], SILL), (COWL[0], COWL[1]), (1.05, ROOF - 0.03), (0.98, ROOF), (-half + 0.08, ROOF),
           (-half, ROOF - 0.08), (-half, SILL + 0.20), (-half + 0.30, SILL)]
    prism("body", cab, width, mat=m["paint"], parent=hull, bevel=0.05, taper_y=lambda z: 1.0 - 0.06 * max(0.0, z - 1.2))
    hood = [(COWL[0], SILL + 0.10), (COWL[0], COWL[1]), (NOSE - 0.30, 1.25), (NOSE - 0.12, 1.10),
            (NOSE - 0.12, SILL + 0.25)]
    prism("hood", hood, width - 0.20, mat=m["paint"], parent=hull, bevel=0.05)
    box("hood_scoop", (0.70, 0.70, 0.10), (2.0, 0.30, 1.30), m["paint"], hull, rot=(0, 0.12, 0), bevel=0.03)
    # The flared arches over each wheel.
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            arch = [(x - 0.72, 0.95), (x - 0.50, 1.22), (x + 0.50, 1.22), (x + 0.72, 0.95), (x + 0.62, 0.90),
                    (x + 0.42, 1.12), (x - 0.42, 1.12), (x - 0.62, 0.90)]
            prism(f"arch_{s}_{k}", arch, 0.12, loc=(0, side * (width / 2 + 0.02), 0), mat=m["dark"], parent=hull,
                  bevel=0.02, lods=MID)
    # The grille's mesh panels between the headlights, the bull bar.
    for k, y in enumerate((0.25, -0.25)):
        box(f"grille_{k}", (0.03, 0.40, 0.42), (NOSE - 0.10, y, 0.88), m["black"], hull, lods=MID)
        for j in range(5):
            box(f"grille_bar_{k}_{j}", (0.03, 0.40, 0.025), (NOSE - 0.09, y, 0.72 + j * 0.08), m["dark"], hull,
                lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        cyl(f"headlight_bezel_{s}", 0.11, 0.05, (NOSE - 0.12, side * 0.62, 0.92), "X", m["dark"], hull, seg=16,
            lods=MID)
        cyl(f"headlight_{s}", 0.085, 0.02, (NOSE - 0.09, side * 0.62, 0.92), "X", m["lamp"], hull, seg=16, lods=MID)
        cyl(f"marker_{s}", 0.05, 0.02, (NOSE - 0.10, side * 0.84, 0.92), "X", m["tail"], hull, seg=10, lods=MID)
    bar = empty("dressing_bull_bar", parent=hull)
    for side in (-1, 1):
        box(f"bull_post_{side}", (0.06, 0.06, 0.55), (half - 0.06, side * 0.55, 0.78), m["black"], bar, lods=MID)
        box(f"bull_arm_{side}", (0.30, 0.05, 0.05), (half - 0.20, side * 0.55, 1.03), m["black"], bar, lods=NEAR)
    box("bull_top", (0.06, 1.20, 0.06), (half - 0.06, 0, 1.03), m["black"], bar, lods=MID)
    box("bull_mid", (0.06, 1.20, 0.05), (half - 0.06, 0, 0.75), m["black"], bar, lods=NEAR)
    box("front_bumper", (0.14, 1.80, 0.16), (NOSE - 0.05, 0, 0.55), m["black"], hull, bevel=0.015)
    VP.tow_hook("front_tow", (NOSE + 0.02, 0, 0.50), m, hull, size=0.10)
    # Windscreen, windows with their port covers, doors and handles.
    lean = math.atan((COWL[0] - 1.05) / (ROOF - 0.03 - COWL[1]))
    for side, s in ((1, "L"), (-1, "R")):
        box(f"windscreen_{s}", (0.03, 0.85, 0.42), (1.21, side * 0.47, 1.68), m["glass"], hull, rot=(0, -lean, 0),
            lods=MID)
        y = side * (width / 2 + 0.005)
        for k, x in enumerate((0.70, -0.35, -1.45)):
            box(f"window_{s}_{k}", (0.55 if k == 0 else 0.48, 0.03, 0.26), (x, y, 1.66), m["glass"], hull, lods=MID)
            box(f"window_frame_{s}_{k}", (0.63 if k == 0 else 0.56, 0.025, 0.34), (x, y - side * 0.01, 1.66),
                m["dark"], hull, lods=NEAR)
            cyl(f"port_cover_{s}_{k}", 0.08, 0.03, (x + 0.10, y + side * 0.02, 1.66), "Y", m["paint"], hull, seg=14,
                bevel=0.01, lods=MID)
        for k, (front, rear) in enumerate(((1.30, 0.20), (0.10, -0.95))):
            VP.weld_line(f"door_seam_{s}_{k}", [(front, y, SILL + 0.15), (front, y, ROOF - 0.12),
                                                (rear, y, ROOF - 0.12), (rear, y, SILL + 0.15)], m, hull,
                         radius=0.012)
            box(f"door_handle_{s}_{k}", (0.14, 0.04, 0.05), (rear + 0.15, y + side * 0.02, 1.25), m["steel"], hull,
                lods=FINE)
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_arm_{s}", (0.04, 0.20, 0.03), (1.20, side * 1.12, 1.55), m["dark"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.05, 0.12, 0.26), (1.20, side * 1.18, 1.62), m["black"], mirror, bevel=0.01, lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.02, side * 0.88, 1.10), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
        box(f"roof_rail_{s}", (2.60, 0.04, 0.06), (-1.10, side * 0.90, ROOF + 0.03), m["steel"], hull, lods=NEAR)
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-2.50, side * 0.85, ROOF), m, whip, height=2.4)
    # Amber beacons over the windscreen, the rear door and its spare wheel.
    for k, y in enumerate((-0.30, 0.30)):
        cyl(f"beacon_{k}", 0.04, 0.05, (0.95, y, ROOF + 0.025), "Z", m["tail"], hull, seg=10, lods=MID)
    box("rear_door", (0.04, 0.80, 1.05), (-half - 0.01, -0.20, 1.25), m["paint"], hull, bevel=0.015, lods=MID)
    spare = empty("dressing_spare_wheel", parent=hull)
    cyl("spare_tyre", WHEEL_R * 0.97, 0.30, (-half - 0.14, 0.35, 1.10), "X", m["rubber"], spare, seg=28)
    cyl("spare_rim", 0.26, 0.06, (-half - 0.26, 0.35, 1.10), "X", m["dark"], spare, seg=18, lods=MID)
    box("rear_bumper", (0.12, 1.90, 0.14), (-half + 0.04, 0, 0.55), m["black"], hull, bevel=0.012)


def kord(v, station, pitch, pivot):
    """The Kord on its ring round the roof hatch, a small shield ahead of
    the gunner, the ammunition box; the gunner stands in the hatch and turns
    with the ring."""
    m = v.mats
    foot = ROOF - pivot.z
    cyl("hatch_ring", 0.45, 0.06, (0, 0, foot + 0.03), "Z", m["dark"], station, seg=24, lods=MID)
    box("shield", (0.04, 0.70, 0.36), (0.45, 0, foot + 0.24), m["paint"], station, bevel=0.012)
    for side in (-1, 1):
        box(f"shield_wing_{side}", (0.04, 0.26, 0.30), (0.36, side * 0.43, foot + 0.21), m["paint"], station,
            rot=(0, 0, side * 0.7), bevel=0.01, lods=MID)
    box("pintle", (0.10, 0.10, 0.20), (0.20, 0, -0.12), m["dark"], station, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("kord_receiver", (0.55, 0.12, 0.15), (0.10, 0, 0), m["dark"], pitch, bevel=0.012)
    cyl("kord_barrel", 0.024, reach - 0.37, ((reach + 0.37) / 2, 0, 0), "X", m["steel"], pitch, seg=10)
    cyl("kord_brake", 0.035, 0.10, (reach - 0.05, 0, 0), "X", m["dark"], pitch, seg=10, lods=MID)
    box("kord_ammo", (0.26, 0.10, 0.18), (0.10, -0.13, -0.06), m["dark"], pitch, bevel=0.01, lods=MID)
    if not v.wreck:
        v.crew.append(("gunner", station, (-0.40, 0.0, ROOF - 0.20),
                       ((-0.18, 0.12, pivot.z + 0.28), (-0.18, -0.12, pivot.z + 0.28)),
                       ((-0.42, 0.12, ROOF - 1.05), (-0.42, -0.12, ROOF - 1.05))))


def wreck(variant, v):
    """The Tigr after its fire: the front right wheel blown off and the nose
    down on it, the right front door torn off and lying beside it, the hood
    buckled and the body warped, the bull bar bent, the spare burnt off,
    torn plate on the ground. Whole."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "hub_R_0", "dressing_spare_wheel", "spare_", "mirror_R", "dressing_mirror_R")
    plate("door_lying", [(-0.55, -0.45), (0.55, -0.47), (0.53, 0.45), (-0.55, 0.43)], 0.04, (0.6, -1.35, 0.04),
          (0.02, 0.03, 0.0), m["paint"], v.root, seed=191)
    bend(parts("bull_"), (v.length / 2 - 0.06, 0, 0.50), (0, 1, 0), (1, 0, 0), 0.35)
    shell = parts("body", "hood")
    densify(shell, scale=1.5)
    warp(shell, heat(0.025, 0.7, seed=24.0), dent((2.0, 0.0, 1.30), 0.55, -0.12, (0, 0, 1)))
    for k, (loc, rot, size) in enumerate((((1.6, 1.55, 0.03), (0.03, 0.04, 0.9), 0.24),
                                          ((-1.6, -1.55, 0.03), (-0.04, 0.02, 2.2), 0.22))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.root, curl=0.12,
              seed=201 + k)
    v.root.rotation_euler = (0.05, 0.05, 0)
    v.root.location.z -= 0.05
    rest_on_ground(0.004)


run("tigr", "russian_green", build, wreck, chip=0.6)
