"""VBL machine-gun scout, from assets/references/vbl/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/vbl.py -- [--wreck]

What the photos settle: four black bar-tread tyres on black steel wheels
with a ring of nuts, a short wheelbase; a small armoured body, its upper
sides leaning in under a low roof, wide flared arches; the long sloping
hood ending in a blunt nose with the headlights and amber markers in it,
tow eyes below, its engine louvres on the sides ahead of the doors; small
windscreens; the round stowage canister, tools and a box hung on the flank
behind the door; the rear door with the jerrycans and kit stowed beside it;
on the roof the ring mount with its machine gun and a small shield, the
gunner standing in it (photos: side, three-quarter front).

Built to the catalog frame (hull 3.8 x 2.02 x 1.7 m, HMG pivot 1.55 m):
nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_R = 0.42
WHEEL_W = 0.30
WHEEL_Y = 0.80
AXLES = (1.40, -0.98)
BELLY = 0.40
WAIST = 0.98
ROOF = 1.60
NOSE = 1.90


def plan(rear, front, half, chamfer):
    return [(rear, -half), (front - chamfer, -half), (front, -half + chamfer), (front, half - chamfer),
            (front - chamfer, half), (rear, half)]


def build(variant, v):
    m, hull = v.mats, v.hull
    black = dict(m, paint=m["black"])
    loft("vbl_body", [(BELLY, plan(-1.75, 1.75, 0.70, 0.25)),
                      (WAIST, plan(-1.90, NOSE, 0.98, 0.35)),
                      (1.05, plan(-1.88, NOSE - 0.05, 0.98, 0.35)),
                      (1.25, plan(-1.82, 1.20, 0.92, 0.20)),
                      (ROOF, plan(-1.55, 0.55, 0.80, 0.15))], mat=m["paint"], parent=hull, bevel=0.045)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, black, hull,
                          rim_radius=0.24, tread="bar", hub_bolts=6)
            box(f"a_arm_{s}_{k}", (0.22, 0.30, 0.07), (x, side * (WHEEL_Y - 0.30), WHEEL_R - 0.04), m["dark"], hull,
                lods=NEAR)
            arch = [(x - 0.55, 0.62), (x - 0.38, 0.95), (x + 0.38, 0.95), (x + 0.55, 0.62), (x + 0.47, 0.58),
                    (x + 0.32, 0.87), (x - 0.32, 0.87), (x - 0.47, 0.58)]
            prism(f"arch_{s}_{k}", arch, 0.09, loc=(0, side * 0.96, 0), mat=m["black"], parent=hull, bevel=0.015,
                  lods=MID)
    fittings(v)
    station, pitch, _, pivot = rig(v.frame, v.root)["HMG"]
    ring_mount(v, station, pitch, pivot)


def fittings(v):
    m, hull = v.mats, v.hull
    lean = math.atan((0.92 - 0.80) / (ROOF - 1.25))
    # Nose: lights, markers, tow eyes; the hood's grille cover; windscreens.
    for side, s in ((1, "L"), (-1, "R")):
        cyl(f"headlight_{s}", 0.065, 0.03, (NOSE - 0.02, side * 0.48, 0.90), "X", m["lamp"], hull, seg=14, lods=MID)
        box(f"marker_{s}", (0.02, 0.10, 0.12), (NOSE - 0.02, side * 0.66, 0.90), m["tail"], hull, lods=MID)
        VP.tow_hook(f"front_tow_{s}", (NOSE - 0.10, side * 0.45, 0.62), m, hull, size=0.09)
        # Engine louvres on the side ahead of the door.
        VP.grille(f"side_louvre_{s}", (1.05, side * 0.95, 1.12), (0.45, 0.22), m, hull, slats=7,
                  rot=(-side * math.pi / 2, 0, 0))
        y = side * 0.88
        box(f"door_{s}", (0.70, 0.03, 0.55), (0.10, y, 1.30), m["paint"], hull, rot=(-side * lean, 0, 0),
            bevel=0.012, lods=MID)
        box(f"door_window_{s}", (0.40, 0.02, 0.14), (0.15, side * 0.85, 1.48), m["glass"], hull,
            rot=(-side * lean, 0, 0), lods=MID)
        box(f"door_handle_{s}", (0.12, 0.04, 0.04), (-0.15, side * 0.92, 1.20), m["steel"], hull, lods=FINE)
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_arm_{s}", (0.03, 0.03, 0.30), (0.55, side * 1.02, 1.40), m["black"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.04, 0.14, 0.22), (0.55, side * 1.05, 1.62), m["black"], mirror, bevel=0.01, lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-1.90, side * 0.80, 1.00), 0.04, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-1.40, side * 0.70, ROOF), m, whip, height=2.4)
    slope = math.atan((1.25 - 1.05) / (NOSE - 0.05 - 1.20))
    VP.grille("hood_grille", (1.55, 0.25, 1.15), (0.30, 0.40), m, hull, slats=5, rot=(0, slope, 0))
    rake = math.atan((1.20 - 0.55) / (ROOF - 1.25))
    for k, y in enumerate((0.38, -0.38)):
        box(f"windscreen_{k}", (0.02, 0.62, 0.34), (0.87, y, 1.43), m["glass"], hull, rot=(0, -rake, 0),
            lods=MID)
    # The flank's stowage: the round canister, tools and a box behind the door.
    kit = empty("dressing_flank_stowage", parent=hull)
    cyl("canister", 0.25, 0.16, (-0.55, 1.00, 1.30), "Y", m["dark"], kit, seg=20, bevel=0.02)
    for k, a in enumerate((0.7, -0.7)):
        box(f"canister_strap_{k}", (0.50, 0.02, 0.03), (-0.55, 1.085, 1.30), m["black"], kit, rot=(a, 0, 0),
            lods=NEAR)
    box("shovel_handle", (0.80, 0.03, 0.04), (-1.30, 0.97, 1.32), m["dark"], kit, lods=NEAR)
    box("shovel_blade", (0.20, 0.02, 0.16), (-0.85, 0.97, 1.32), m["steel"], kit, lods=NEAR)
    VP.stowage_box("flank_box", (-1.25, -0.98, 1.05), (0.50, 0.18, 0.36), dict(m, paint=m["dark"]), kit)
    for k, y in enumerate((0.35, 0.65)):
        VP.jerrycan(f"rear_jerrycan_{k}", (-1.98, y, 0.95), dict(m, paint=m["dark"]), kit)
    box("rear_door", (0.03, 0.55, 0.55), (-1.88, -0.25, 1.25), m["paint"], hull, rot=(0, -0.3, 0), bevel=0.012,
        lods=MID)


def ring_mount(v, station, pitch, pivot):
    """The roof ring and its machine gun with a small shield; the gunner
    stands in the ring and turns with it."""
    m = v.mats
    foot = ROOF - pivot.z
    cyl("ring", 0.42, 0.05, (0, 0, foot + 0.025), "Z", m["dark"], station, seg=24, lods=MID)
    cyl("ring_rail", 0.44, 0.03, (0, 0, foot + 0.14), "Z", m["black"], station, seg=24, lods=NEAR)
    box("shield", (0.03, 0.42, 0.24), (0.32, 0, 0.02), m["paint"], station, bevel=0.01)
    box("pintle", (0.08, 0.08, 0.20), (0.15, 0, foot + 0.12), m["dark"], station, lods=MID)
    VP.browning_m2(pitch, v.frame["mounts"][0]["muzzle_m"][0], m, grips=True)
    box("ammo_box", (0.26, 0.10, 0.18), (0.06, -0.13, -0.06), m["dark"], pitch, bevel=0.01, lods=MID)
    if not v.wreck:
        v.crew.append(("gunner", station, (-0.30, 0.0, ROOF - 0.18),
                       ((-0.22, 0.10, pivot.z + 0.30), (-0.22, -0.10, pivot.z + 0.30)),
                       ((-0.30, 0.12, ROOF - 1.05), (-0.30, -0.12, ROOF - 1.05))))


def wreck(variant, v):
    """The VBL after its fire: the front left wheel blown off and the nose
    down on it, the left door torn off and lying beside it, the flank
    stowage burnt away, the body warped and dented, torn plate on the
    ground. Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "door_L", "door_window_L", "door_handle_L", "dressing_flank_stowage", "canister",
           "shovel_", "flank_box", "rear_jerrycan_")
    plate("door_lying", [(-0.35, -0.27), (0.35, -0.28), (0.35, 0.27), (-0.35, 0.26)], 0.03, (0.1, 1.40, 0.04),
          (0.02, 0.03, 0.0), m["paint"], v.root, seed=231)
    shell = parts("vbl_body")
    densify(shell, scale=1.2)
    warp(shell, heat(0.02, 0.6, seed=28.0), dent((1.4, 0.0, 1.15), 0.4, -0.10, (0, 0, 1)))
    for k, (loc, rot, size) in enumerate((((1.2, -1.30, 0.03), (0.03, 0.04, 0.9), 0.20),
                                          ((-1.2, 1.30, 0.03), (-0.04, 0.02, 2.2), 0.18))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.root, curl=0.12,
              seed=241 + k)
    v.root.rotation_euler = (-0.05, 0.05, 0)
    v.root.location.z -= 0.04
    rest_on_ground(0.004)


run("vbl", "french_three_tone", build, wreck, chip=0.6)
