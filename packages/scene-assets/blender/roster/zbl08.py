"""ZBL-08 wheeled IFV, from assets/references/zbl08/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/zbl08.py -- [--wreck]

What the photos settle: eight black road-tread tyres on green rims with big
hub caps, axles in two pairs with the wide gap amidships; a narrow dark lower
hull and a tall slab-sided upper hull over it, bolted all over, its lower
edge dropping between the wheels; a long low glacis from the nose up to the
roof, its louvres on the right and the driver's hatch with periscopes on
the left, mirrors on stalks at the nose corners; the low wedge turret with
its 30 mm, smoke tubes in two banks of four on its cheeks, the sight head
and hatches on its roof; the grab rails, roof hatches over the troop
compartment, rear door and the two water-jet housings low on the rear.

Built to the catalog frame (hull 8.0 x 3.0 x 2.8 m): the photos put the
turret ring about 1 m behind amidships and its muzzle about 2.5 m ahead of
it; the frame puts the ring amidships and the muzzle 4.25 m out (past the
bow). Nothing here moves it: the turret is drawn on the frame's pivot and
the barrel to its muzzle (choices.md, slice 16).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [2.55, 1.06, -1.03, -2.55]
WHEEL_R = 0.61
WHEEL_W = 0.38
WHEEL_Y = 1.28
SKIRT = 1.40
ROOF = 2.20  # the photos put it near 2.3 m; lowered so the frame's 2.35 m gun axis clears it
NOSE = 4.0
NOSE_Z = 1.72
GLACIS_TOP = 1.75


def glacis_z(x):
    return NOSE_Z + 0.06 + (ROOF - NOSE_Z - 0.06) * (NOSE - 0.10 - x) / (NOSE - 0.10 - GLACIS_TOP)


def build(variant, v):
    m, hull = v.mats, v.hull
    loft("zbl_lower", [(0.60, VP.hull_plan(-3.75, 3.15, 0.92, 0.35)), (SKIRT, VP.hull_plan(-3.95, 3.55, 1.00, 0.50))],
         mat=m["dark"], parent=hull, bevel=0.04)
    loft("zbl_upper", [(SKIRT, VP.hull_plan(-3.98, 3.60, 1.48, 0.60)),
                       (NOSE_Z, VP.hull_plan(-4.0, NOSE, 1.48, 0.70)),
                       (NOSE_Z + 0.06, VP.hull_plan(-4.0, NOSE - 0.10, 1.48, 0.70)),
                       (ROOF, VP.hull_plan(-3.98, GLACIS_TOP, 1.45, 0.30))], mat=m["paint"], parent=hull, bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.33, ctis=True, hub_bolts=8)
            cyl(f"hub_cap_{s}_{k}", 0.15, 0.06, (x, side * (WHEEL_Y + WHEEL_W / 2 + 0.05), WHEEL_R), "Y", m["paint"],
                hull, seg=14, bevel=0.015, lods=MID)
            box(f"suspension_arm_{s}_{k}", (0.20, 0.36, 0.12), (x, side * (WHEEL_Y - 0.34), WHEEL_R), m["dark"],
                hull, lods=NEAR)
        # The upper hull's edge drops between each pair of wheels.
        gaps = [(WHEEL_X[k] + WHEEL_X[k + 1]) / 2 for k in range(3)]
        for k, x in enumerate(gaps):
            half = 0.30 if k != 1 else 0.70
            prism(f"skirt_{s}_{k}", [(x - half - 0.20, SKIRT), (x + half + 0.20, SKIRT),
                                     (x + half - 0.05, SKIRT - 0.28), (x - half + 0.05, SKIRT - 0.28)], 0.06,
                  loc=(0, side * 1.45, 0),
                  mat=m["paint"], parent=hull, bevel=0.015, lods=MID)
        VP.mudflap(f"mudflap_{s}", (-3.35, side * WHEEL_Y, SKIRT - 0.02), (0.40, 0.55), m, hull)
    fittings(v)
    turret, gun, _, _ = rig(v.frame, v.root)["autocannon"]
    zbl_turret(v, turret, gun)


def fittings(v):
    m, hull = v.mats, v.hull
    slope = math.atan((ROOF - NOSE_Z - 0.06) / (NOSE - 0.10 - GLACIS_TOP))
    VP.grille("glacis_louvre", (2.90, -0.65, glacis_z(2.90) + 0.01), (0.90, 0.80), m, hull, slats=8,
              rot=(0, slope, 0))
    VP.hatch("driver_hatch", (2.20, 0.65, glacis_z(2.20)), m, hull, size=(0.60, 0.55), rot=(0, slope, 0))
    for k, y in enumerate((0.40, 0.65, 0.90)):
        VP.periscope(f"driver_periscope_{k}", (2.62, y, glacis_z(2.62)), m, hull, size=(0.12, 0.16, 0.09),
                     rot=(0, slope, 0))
    VP.bolted_panel("glacis_plate", (3.40, 0.40, glacis_z(3.40)), (0.60, 1.10, 0.035), m, hull, bolts=(3, 4),
                    rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        box(f"light_box_{s}", (0.06, 0.30, 0.14), (NOSE - 0.02, side * 1.10, NOSE_Z - 0.05), m["dark"], hull,
            bevel=0.012, lods=MID)
        for k, y in enumerate((1.02, 1.18)):
            cyl(f"headlight_{s}_{k}", 0.05, 0.02, (NOSE + 0.01, side * y, NOSE_Z - 0.05), "X", m["lamp"], hull, seg=12,
                lods=MID)
        VP.shackle(f"nose_shackle_{s}", (NOSE - 0.15, side * 0.60, NOSE_Z - 0.40), m, hull, size=0.13,
                   rot=(0, 0, math.pi / 2))
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_stalk_{s}", (0.04, 0.04, 0.45), (NOSE - 0.25, side * 1.40, NOSE_Z + 0.22), m["dark"], mirror,
            lods=NEAR)
        box(f"mirror_{s}", (0.05, 0.16, 0.30), (NOSE - 0.25, side * 1.40, NOSE_Z + 0.55), m["dark"], mirror,
            bevel=0.012, lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-4.0, side * 1.25, 1.90), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        # Water-jet housings low on the rear.
        cyl(f"waterjet_{s}", 0.22, 0.35, (-3.85, side * 0.85, 0.95), "X", m["dark"], hull, seg=18, bevel=0.02,
            lods=MID)
        cyl(f"waterjet_mouth_{s}", 0.16, 0.01, (-4.03, side * 0.85, 0.95), "X", m["black"], hull, seg=18, lods=MID)
        # The slab side: bolted panels and a grab rail.
        face = side * 1.48
        for k, x in enumerate((2.30, 0.95, -0.40, -1.75, -3.10)):
            VP.bolted_panel(f"side_panel_{s}_{k}", (x, face, 1.85), (1.25, 0.72, 0.025), m, hull, bolts=(4, 3),
                            rot=(-side * math.pi / 2, 0, 0), bevel=0.012)
        box(f"grab_rail_{s}", (2.40, 0.04, 0.04), (-1.60, side * 1.38, ROOF + 0.10), m["steel"], hull, lods=NEAR)
    VP.exhaust("exhaust", (2.10, -1.52, 2.00), 0.07, 0.35, m, hull, rot=(0, 0, -math.pi / 2))
    for k, (x, y) in enumerate(((-1.95, 0.60), (-1.95, -0.60), (-3.05, 0.60), (-3.05, -0.60))):
        VP.hatch(f"troop_hatch_{k}", (x, y, ROOF), m, hull, size=(0.80, 0.62))
    box("rear_door", (0.05, 1.10, 1.10), (-3.99, 0, 1.70), m["paint"], hull, bevel=0.02)
    box("rear_door_glass", (0.02, 0.16, 0.08), (-4.01, 0, 2.05), m["glass"], hull, lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-3.60, side * 1.25, ROOF), m, whip, height=2.8)


def zbl_turret(v, turret, gun):
    """The low wedge turret: its 30 mm with a long barrel, the coax, two
    banks of four smoke tubes on its cheeks, the sight head and the hatches
    on its roof."""
    m = v.mats
    pz = v.frame["mounts"][0]["pivot_m"][2]
    base = ROOF - pz
    top = 2.72 - pz
    cyl("turret_ring", 0.95, 0.06, (0, 0, base + 0.03), "Z", m["dark"], turret, seg=32, lods=MID)
    foot = [(1.05, 0.35), (0.65, 0.92), (-1.10, 0.92), (-1.25, 0.70), (-1.25, -0.70), (-1.10, -0.92), (0.65, -0.92),
            (1.05, -0.35)]
    crown = [(0.30, 0.25), (0.10, 0.72), (-1.05, 0.75), (-1.20, 0.58), (-1.20, -0.58), (-1.05, -0.75),
             (0.10, -0.72), (0.30, -0.25)]
    loft("turret_shell", [(base + 0.05, foot), (base + 0.22, foot), (top, crown)], mat=m["paint"], parent=turret,
         bevel=0.045)
    VP.sight_housing("gunner_sight", (-0.05, -0.45, top), m, turret, size=(0.34, 0.24, 0.12))
    for k, y in enumerate((0.40, -0.10)):
        VP.hatch(f"turret_hatch_{k}", (-0.65, y, top), m, turret, radius=0.25)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.35, side * 0.88, base + 0.18), m, turret, count=4,
                                 tube_radius=0.05, tube_length=0.26, elevation=0.45, spread=0.3,
                                 rot=(0, 0, side * 0.7))
    box("bustle_rack", (0.30, 1.50, 0.22), (-1.38, 0, base + 0.20), m["steel"], turret, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.40, 0.42, 0.26), (0.90, 0, 0), m["paint"], gun, bevel=0.035)
    cyl("gun_sleeve", 0.07, 0.55, (1.30, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("gun_barrel", 0.040, reach - 1.55, ((reach + 1.55) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("gun_muzzle", 0.058, 0.18, (reach - 0.09, 0, 0), "X", m["dark"], gun, seg=12)
    cyl("gun_bore", 0.026, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.02, 0.40, (1.20, 0.22, 0.0), "X", m["steel"], gun, seg=8, lods=NEAR)


def wreck(variant, v):
    """The ZBL-08 after its fire: the rear right wheel pair blown off and
    the tail down on that side, the rear door blown out and lying beside it,
    a side panel torn away and the mirrors bent, hatches gone, plates warped
    and a dent in the left side, torn plate on the ground. It throws its
    turret."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_3_", "wheel_R_4_", "hub_cap_R_2", "hub_cap_R_3", "rear_door", "side_panel_R_3", "troop_hatch_")
    plate("door_lying", [(-0.55, -0.52), (0.56, -0.53), (0.55, 0.52), (-0.56, 0.51)], 0.05, (-2.6, -1.60, 0.04),
          (0.03, 0.02, 0.15), m["paint"], v.root, seed=151)
    bend(parts("mirror_"), (NOSE - 0.25, 1.40, NOSE_Z), (1, 0, 0), (0, 0, 1), 0.7)
    shell = parts("zbl_upper", "zbl_lower", "side_panel_", "glacis_plate")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=20.0), dent((0.3, 1.48, 1.85), 0.55, 0.13, (0, -1, -0.1)))
    for k, (loc, rot, size) in enumerate((((2.4, 1.90, 0.03), (0.03, 0.04, 0.9), 0.24),
                                          ((0.4, -1.90, 0.03), (-0.04, 0.02, 2.2), 0.22),
                                          ((-2.4, 0.3, ROOF + 0.04), (0.02, 0.04, 0.3), 0.32))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12,
              seed=161 + k)
    v.root.rotation_euler = (0.04, -0.035, 0)
    v.root.location.z -= 0.08
    rest_on_ground(0.004)


run("zbl08", "chinese_digital", build, wreck, chip=0.6)
