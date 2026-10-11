"""BTR-82A, from assets/references/btr/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/btr.py -- [--wreck]

What the photos settle: eight black road-tread tyres on green-painted rims
with the big hub caps of their CTIS, four axles with the wider gap between
the second and third; the boat hull: a lower hull tapering in between the
wheels, a chine above them, upper sides leaning in to the roof; the pointed
bow, its upper glacis carrying the folded trim vane, the driver's and
commander's windscreens with their armoured covers, lights in boxes; the
side door between the second and third axles with its firing ports, more
ports along the side; the engine deck at the rear with its stowage tube on
the right and the louvres; the small conical BPPU turret with its 30 mm
2A72, coaxial PKTM, the sight head on its roof and three smoke tubes on
each side.

Built to the catalog frame (hull 7.7 x 2.9 x 2.41 m, gun axis 2.0 m): the
photos put the roof near 2.1 m and the turret's top near 2.6 m. The roof is
drawn at 1.90 m under a low turret (specs/done/unit-models/choices.md); the
turret ring is 0.85 m ahead of amidships, over the second axle, where the
photos put it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [2.45, 0.98, -0.95, -2.42]
WHEEL_R = 0.56
WHEEL_W = 0.34
WHEEL_Y = 1.24
BELLY = 0.70
CHINE = 1.22
ROOF = 1.90
BOW = 3.85
BOW_Z = 1.40
GLACIS_TOP = 2.65
# The upper side face, leaning in from the bow's edge to the roof, as (y, z)
# at its foot and top (`vehicle_parts.on_side`).
UPPER_SIDE = ((1.36, BOW_Z + 0.05), (1.02, ROOF))


def glacis_z(x):
    return BOW_Z + 0.05 + (ROOF - BOW_Z - 0.05) * (BOW - 0.05 - x) / (BOW - 0.05 - GLACIS_TOP)


def build(variant, v):
    m, hull = v.mats, v.hull
    loft("btr_hull", [(BELLY, VP.hull_plan(-3.55, 3.30, 0.85, 0.35)),
                      (CHINE - 0.10, VP.hull_plan(-3.80, 3.70, 1.32, 0.60)),
                      (CHINE, VP.hull_plan(-3.85, 3.78, 1.42, 0.65)),
                      (BOW_Z, VP.hull_plan(-3.80, BOW, 1.38, 0.70)),
                      (BOW_Z + 0.05, VP.hull_plan(-3.78, BOW - 0.05, UPPER_SIDE[0][0], 0.70)),
                      (ROOF, VP.hull_plan(-3.65, GLACIS_TOP, UPPER_SIDE[1][0], 0.45))], mat=m["paint"], parent=hull, bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.30, ctis=True, hub_bolts=8)
            cyl(f"hub_cap_{s}_{k}", 0.13, 0.08, (x, side * (WHEEL_Y + WHEEL_W / 2 + 0.06), WHEEL_R), "Y", m["paint"],
                hull, seg=14, bevel=0.015, lods=MID)
            box(f"suspension_arm_{s}_{k}", (0.18, 0.40, 0.10), (x, side * (WHEEL_Y - 0.38), WHEEL_R), m["dark"],
                hull, lods=NEAR)
        # The struts that brace the chine between the wheel pairs.
        for k, x in enumerate((1.70, -1.70)):
            box(f"chine_strut_{s}_{k}", (0.08, 0.06, 0.42), (x, side * 1.25, CHINE - 0.22), m["paint"], hull,
                rot=(0, 0.35, 0), lods=MID)
    fittings(v)
    turret, gun, _, _ = rig(v.frame, v.root)["autocannon"]
    bppu(v, turret, gun)


def fittings(v):
    m, hull = v.mats, v.hull
    slope = math.atan((ROOF - BOW_Z - 0.05) / (BOW - 0.05 - GLACIS_TOP))
    # The trim vane folded on the upper glacis, the windscreens' armoured
    # covers, lights in boxes, tow eyes on the bow.
    VP.bolted_panel("trim_vane", (3.30, 0, glacis_z(3.30)), (0.70, 1.90, 0.04), m, hull, bolts=(2, 5),
                    rot=(0, slope, 0), bevel=0.015)
    for k, y in enumerate((0.42, -0.42)):
        x = GLACIS_TOP + 0.20
        box(f"windscreen_cover_{k}", (0.42, 0.70, 0.05), (x, y, glacis_z(x) + 0.03), m["paint"], hull,
            rot=(0, slope, 0), bevel=0.015, lods=MID)
        box(f"windscreen_hinge_{k}", (0.05, 0.62, 0.05), (x + 0.22, y, glacis_z(x + 0.22) + 0.04), m["steel"], hull,
            lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        for k, y in enumerate((1.05, 0.80)):
            box(f"light_box_{s}_{k}", (0.10, 0.18, 0.16), (3.45, side * y, glacis_z(3.45) + 0.08), m["dark"], hull,
                bevel=0.015, lods=MID)
            box(f"light_lens_{s}_{k}", (0.01, 0.13, 0.10), (3.505, side * y, glacis_z(3.45) + 0.08), m["lamp"], hull,
                lods=MID)
        VP.shackle(f"bow_shackle_{s}", (3.70, side * 0.55, 1.12), m, hull, size=0.12, rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-3.80, side * 1.05, CHINE + 0.20), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-3.70, side * 0.60, CHINE - 0.10), m, hull, size=0.10, rot=(0, 0, math.pi))
        # The side door between the middle axles and the firing ports.
        loc, rot = VP.on_side(0.02, 1.58, side, *UPPER_SIDE)
        VP.bolted_panel(f"side_door_{s}", loc, (0.80, 0.42, 0.03), m, hull, bolts=(3, 2), rot=rot, bevel=0.012)
        for k, x in enumerate((2.05, 1.35, -1.20, -1.90)):
            loc, rot = VP.on_side(x, 1.50, side, *UPPER_SIDE)
            cyl(f"firing_port_{s}_{k}", 0.07, 0.05, loc, "Z", m["dark"], hull, seg=12, rot=rot, lods=MID)
        box(f"step_{s}", (0.50, 0.12, 0.04), (0.02, side * 1.40, CHINE - 0.06), m["steel"], hull, lods=NEAR)
    # The engine deck: louvres, the stowage tube on the right.
    for k, y in enumerate((0.45, -0.45)):
        VP.grille(f"engine_louvre_{k}", (-2.70, y, ROOF), (0.90, 0.60), m, hull, slats=7)
    tube = empty("dressing_stowage_tube", parent=hull)
    cyl("stowage_tube", 0.16, 2.10, (-2.10, -0.90, ROOF + 0.14), "X", m["paint"], tube, seg=16, bevel=0.02)
    for k, x in enumerate((-2.90, -1.30)):
        box(f"tube_strap_{k}", (0.06, 0.36, 0.30), (x, -0.90, ROOF + 0.12), m["dark"], tube, lods=NEAR)
    cyl("tube_cap", 0.17, 0.08, (-1.02, -0.90, ROOF + 0.14), "X", m["dark"], tube, seg=16, lods=MID)
    # Roof hatches over the troop compartment, the driver and commander.
    for k, (x, y) in enumerate(((-0.80, 0.55), (-0.80, -0.55))):
        VP.hatch(f"troop_hatch_{k}", (x, y, ROOF), m, hull, size=(0.70, 0.55))
    for k, y in enumerate((0.42, -0.42)):
        VP.hatch(f"front_hatch_{k}", (GLACIS_TOP - 0.40, y, ROOF), m, hull, radius=0.24)
    VP.exhaust("exhaust", (-3.10, 1.30, 1.55), 0.07, 0.40, m, hull, rot=(0, 0, math.pi / 2))
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-3.30, side * 0.80, ROOF), m, whip, height=2.8)


def bppu(v, turret, gun):
    """The BPPU turret: a low truncated cone, the 30 mm 2A72 with its long
    barrel and the coaxial PKTM, the sight head on its roof, three smoke
    tubes each side and the searchlight."""
    m = v.mats
    pz = v.frame["mounts"][0]["pivot_m"][2]
    base = ROOF - pz
    top = 2.36 - pz
    cyl("turret_ring", 0.72, 0.05, (0, 0, base + 0.025), "Z", m["dark"], turret, seg=28, lods=MID)
    cyl("turret_cone", 0.70, top - base - 0.04, (0, 0, (top + base) / 2 + 0.02), "Z", m["paint"], turret, seg=24,
        r2=0.50, bevel=0.03)
    box("gun_housing", (0.50, 0.40, 0.28), (0.45, 0, base + 0.13), m["paint"], turret, bevel=0.04)
    VP.sight_housing("gunner_sight", (-0.05, -0.30, top), m, turret, size=(0.30, 0.22, 0.10))
    VP.hatch("turret_hatch", (-0.20, 0.20, top), m, turret, radius=0.22)
    cyl("searchlight", 0.09, 0.14, (0.28, 0.40, top + 0.06), "X", m["dark"], turret, seg=14, lods=MID)
    cyl("searchlight_lens", 0.075, 0.01, (0.355, 0.40, top + 0.06), "X", m["lamp"], turret, seg=14, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (-0.10, side * 0.58, base + 0.22), m, turret, count=3,
                                 tube_radius=0.045, tube_length=0.26, elevation=0.5, spread=0.3,
                                 rot=(0, 0, side * 1.2))
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.30, 0.34, 0.22), (0.70, 0, 0), m["paint"], gun, bevel=0.03)
    cyl("gun_sleeve", 0.06, 0.40, (1.00, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("gun_barrel", 0.038, reach - 1.20, ((reach + 1.20) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("gun_muzzle", 0.055, 0.16, (reach - 0.08, 0, 0), "X", m["dark"], gun, seg=12)
    cyl("gun_bore", 0.025, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.018, 0.40, (0.95, 0.17, 0.0), "X", m["steel"], gun, seg=8, lods=NEAR)


def wreck(variant, v):
    """The BTR after its fire: the front left wheel pair blown off and the
    bow down on that side, the stowage tube burnt and fallen, the side door
    blown out and lying beside it, the hatches gone, plates warped and a
    dent in the right side, torn plate on the ground. It throws its turret."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "wheel_L_2_", "hub_cap_L_0", "hub_cap_L_1", "dressing_stowage_tube", "stowage_tube",
           "tube_", "side_door_L", "troop_hatch_", "front_hatch_")
    plate("door_lying", [(-0.40, -0.22), (0.41, -0.23), (0.40, 0.22), (-0.41, 0.21)], 0.04, (0.2, 1.80, 0.04),
          (0.02, 0.03, 0.2), m["paint"], v.root, seed=131)
    plate("tube_lying", [(-1.0, -0.14), (1.0, -0.15), (1.0, 0.14), (-1.0, 0.13)], 0.10, (-2.2, -1.75, 0.06),
          (0.0, 0.0, 0.1), m["paint"], v.root, curl=0.1, seed=132)
    shell = parts("btr_hull", "trim_vane", "side_door_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=18.0), dent((0.8, -1.30, 1.50), 0.5, 0.12, (0, 1, -0.2)))
    for k, (loc, rot, size) in enumerate((((2.6, 1.85, 0.03), (0.03, 0.04, 0.9), 0.24),
                                          ((-0.6, -1.85, 0.03), (-0.04, 0.02, 2.2), 0.22),
                                          ((-1.6, 0.3, ROOF + 0.04), (0.02, 0.04, 0.3), 0.32))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12,
              seed=141 + k)
    v.root.rotation_euler = (-0.045, 0.04, 0)
    v.root.location.z -= 0.08
    rest_on_ground(0.004)


run("btr", "russian_green", build, wreck, chip=0.6)
