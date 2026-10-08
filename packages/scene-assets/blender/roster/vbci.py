"""VBCI infantry fighting vehicle, from assets/references/vbci/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/vbci.py -- [--wreck]

What the photos settle: eight black road-tread tyres on dark rims with a
ring of wheel nuts, axles in two pairs with the wide gap amidships; a dark,
recessed lower hull, and over it the slab-sided upper hull whose bolted side
plates overhang the wheels; the V bow, its upper plate sloping up to the
roof and its lower plate back under the nose; the driver's hatch and
periscopes on the left front of the roof, the engine's grille beside them;
the side plates' outlined hatches and a dense pattern of bolts; the one-man
DRAGAR turret right of centre with its 25 mm, smoke tubes on its cheeks and
the gunner's sight up on its pedestal; the commander's hatch beside it,
crewed in the parade photo; the troop hatches and big rear door with its
stowage cage.

Built to the catalog frame (hull 7.6 x 2.98 x 3.0 m with the turret's sight):
nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [2.43, 0.86, -0.97, -2.62]
WHEEL_R = 0.62
WHEEL_W = 0.38
WHEEL_Y = 1.18
SKIRT = 1.54  # the upper hull's underside
ROOF = 2.40
NOSE = 3.80
NOSE_Z = 1.74
# The slab sides as (y, z) at foot and top (`vehicle_parts.on_side`): under
# the bow's edge, and leaning in from over it to the roof.
LOWER_SIDE = ((1.47, SKIRT), (1.49, NOSE_Z))
UPPER_SIDE = ((1.49, NOSE_Z + 0.06), (1.45, ROOF))


def build(variant, v):
    m, hull = v.mats, v.hull
    dark = dict(m, paint=m["dark"])
    # The recessed lower hull and its V bow, in the dark under-paint.
    loft("vbci_lower", [(0.55, VP.hull_plan(-3.55, 3.00, 0.95, 0.35)), (SKIRT, VP.hull_plan(-3.72, 3.35, 1.02, 0.50))],
         mat=m["dark"], parent=hull, bevel=0.04)
    # The overhanging upper hull: the bow comes to its point at NOSE_Z, the
    # upper plate rises to the roof.
    loft("vbci_upper", [(SKIRT, VP.hull_plan(-3.78, 3.45, 1.47, 0.70)),
                        (NOSE_Z, VP.hull_plan(-3.80, NOSE, 1.49, 0.80)),
                        (NOSE_Z + 0.06, VP.hull_plan(-3.80, NOSE - 0.12, UPPER_SIDE[0][0], 0.80)),
                        (ROOF, VP.hull_plan(-3.76, 2.55, UPPER_SIDE[1][0], 0.45))], mat=m["paint"], parent=hull, bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, dark, hull,
                          rim_radius=0.33, hub_bolts=12)
            box(f"suspension_arm_{s}_{k}", (0.22, 0.36, 0.12), (x, side * (WHEEL_Y - 0.34), WHEEL_R), m["dark"],
                hull, lods=NEAR)
        VP.mudflap(f"mudflap_{s}", (-3.30, side * WHEEL_Y, SKIRT - 0.02), (0.40, 0.60), m, hull)
    fittings(v)
    turret, gun, _, _ = rig(v.frame, v.root)["autocannon"]
    dragar(v, turret, gun)


def fittings(v):
    m, hull = v.mats, v.hull
    slope = math.atan((ROOF - NOSE_Z - 0.06) / (NOSE - 0.12 - 2.55))

    def bow_z(x):
        return NOSE_Z + 0.06 + (ROOF - NOSE_Z - 0.06) * (NOSE - 0.12 - x) / (NOSE - 0.12 - 2.55)

    # The bow: bolted plates on the upper bow, lights at its corners, the
    # engine's grille on the right of the roof's front, the driver's hatch
    # and periscopes on the left.
    for k, y in enumerate((0.55, -0.55)):
        VP.bolted_panel(f"bow_plate_{k}", (3.15, y, bow_z(3.15)), (0.80, 0.95, 0.035), m, hull, bolts=(3, 4),
                        rot=(0, slope, 0))
    VP.grille("engine_grille", (1.90, -0.70, ROOF), (0.90, 0.80), m, hull, slats=8)
    VP.hatch("driver_hatch", (2.10, 0.70, ROOF), m, hull, radius=0.30)
    for k, y in enumerate((0.45, 0.70, 0.95)):
        VP.periscope(f"driver_periscope_{k}", (2.48, y, ROOF), m, hull, size=(0.12, 0.17, 0.09))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (3.30, side * 1.20, bow_z(3.30) + 0.10), 0.07, m, hull,
                            rot=(0, 0, side * 0.3))
        VP.shackle(f"bow_shackle_{s}", (3.62, side * 0.55, 1.30), m, hull, size=0.13, rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-3.82, side * 1.25, 2.05), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-3.70, side * 0.70, 1.15), m, hull, size=0.10, rot=(0, 0, math.pi))
        # The slab side: outlined hatches and rows of bolts.
        for k, x in enumerate((1.70, -0.30, -2.30)):
            loc, rot = VP.on_side(x, 2.00, side, *UPPER_SIDE)
            VP.bolted_panel(f"side_hatch_{s}_{k}", loc, (0.70, 0.62, 0.025), m, hull, bolts=(3, 3), rot=rot,
                            bevel=0.012)
        for k in range(18):
            x = 3.10 - k * 0.38
            for j, (z, face) in enumerate(((1.62, LOWER_SIDE), (2.30, UPPER_SIDE))):
                loc, rot = VP.on_side(x, z, side, *face, proud=0.012)
                cyl(f"side_bolt_{s}_{k}_{j}", 0.026, 0.03, loc, "Z", m["steel"], hull, seg=6, rot=rot, lods=FINE)
        # The upper hull's lower edge, a darker strip over the wheels.
        box(f"side_rail_{s}", (6.90, 0.05, 0.06), (-0.10, side * 1.48, SKIRT + 0.03), m["dark"], hull, lods=MID)
    VP.exhaust("exhaust", (2.85, -1.50, 1.90), 0.07, 0.35, m, hull, rot=(0, 0, -math.pi / 2))
    # Troop hatches, the rear door and its stowage cage.
    for k, y in enumerate((0.60, -0.60)):
        VP.hatch(f"troop_hatch_{k}", (-2.70, y, ROOF), m, hull, size=(0.85, 0.66))
    box("rear_door", (0.05, 1.10, 1.35), (-3.80, -0.15, 1.45), m["paint"], hull, bevel=0.02)
    box("rear_door_glass", (0.02, 0.18, 0.08), (-3.83, -0.15, 1.90), m["glass"], hull, lods=NEAR)
    for j, z in enumerate((1.00, 1.85)):
        box(f"rear_hinge_{j}", (0.06, 0.10, 0.12), (-3.82, -0.72, z), m["steel"], hull, lods=FINE)
    cage = empty("dressing_rear_cage", parent=hull)
    for side in (-1, 1):
        box(f"cage_post_{side}", (0.04, 0.04, 0.60), (-3.98, side * 1.05 + 0.25, 1.80), m["steel"], cage, lods=NEAR)
    box("cage_rail", (0.04, 0.90, 0.04), (-3.98, 1.00, 2.08), m["steel"], cage, lods=NEAR)
    VP.stowage_box("cage_box", (-3.95, 1.00, 1.52), (0.24, 0.80, 0.52), dict(m, paint=m["dark"]), cage)
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-3.50, side * 1.25, ROOF), m, whip, height=2.8)
    # The commander's hatch beside the turret, crewed in the parade photo.
    VP.hatch("commander_hatch", (0.10, -1.05, ROOF), m, hull, radius=0.28)
    if not v.wreck:
        seat = empty("dressing_commander", parent=hull)
        x, y, z = 0.10, -1.05, ROOF
        v.crew.append(("commander", seat, (x - 0.05, y, z - 0.30),
                       ((x + 0.22, y + 0.20, z + 0.08), (x + 0.22, y - 0.20, z + 0.08)),
                       ((x, y + 0.12, z - 1.15), (x, y - 0.12, z - 1.15))))


def dragar(v, turret, gun):
    """The DRAGAR one-man turret: a compact faceted body, the 25 mm M811
    with its long barrel, smoke tubes on its cheeks, the gunner's sight up
    on its pedestal and stowage on its rear."""
    m = v.mats
    pz = v.frame["mounts"][0]["pivot_m"][2]
    base = ROOF - pz
    cyl("turret_ring", 0.75, 0.06, (0, 0, base + 0.03), "Z", m["dark"], turret, seg=28, lods=MID)
    foot = [(0.80, 0.25), (0.55, 0.62), (-0.70, 0.62), (-0.85, 0.40), (-0.85, -0.40), (-0.70, -0.62),
            (0.55, -0.62), (0.80, -0.25)]
    crown = [(0.50, 0.20), (0.35, 0.50), (-0.62, 0.52), (-0.78, 0.35), (-0.78, -0.35), (-0.62, -0.52),
             (0.35, -0.50), (0.50, -0.20)]
    loft("turret_shell", [(base + 0.05, foot), (0.25, foot), (0.50, crown)], mat=m["paint"], parent=turret,
         bevel=0.04)
    cyl("sight_pedestal", 0.10, 0.08, (-0.15, -0.30, 0.46), "Z", m["dark"], turret, seg=12)
    VP.sight_housing("gunner_sight", (-0.15, -0.30, 0.48), m, turret, size=(0.30, 0.28, 0.20))
    VP.hatch("gunner_hatch", (-0.30, 0.25, 0.50), m, turret, radius=0.25)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.35, side * 0.62, 0.15), m, turret, count=3,
                                 tube_radius=0.05, tube_length=0.24, elevation=0.35, spread=0.3,
                                 rot=(0, 0, side * 0.9))
    VP.stowage_box("bustle_box", (-0.95, 0, 0.0), (0.30, 0.90, 0.32), dict(m, paint=m["dark"]), turret)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.36, 0.42, 0.30), (0.65, 0, 0), m["paint"], gun, bevel=0.035)
    cyl("gun_sleeve", 0.07, 0.45, (1.02, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("gun_barrel", 0.040, reach - 1.20, ((reach + 1.20) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("gun_muzzle", 0.055, 0.16, (reach - 0.08, 0, 0), "X", m["dark"], gun, seg=12)
    cyl("gun_bore", 0.026, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.02, 0.35, (0.95, 0.20, -0.02), "X", m["steel"], gun, seg=8, lods=NEAR)


def wreck(variant, v):
    """The VBCI after its fire: the front right wheel pair blown off and the
    bow down on that side, the rear door blown open and thrown onto the roof, the
    stowage cage burnt away, troop hatches gone, the slab sides warped and
    holed by a dent, torn plate on the ground. It throws its turret."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "wheel_R_2_", "rear_door", "troop_hatch_", "dressing_rear_cage", "cage_")
    plate("door_lying", [(-0.66, -0.52), (0.67, -0.53), (0.66, 0.52), (-0.67, 0.51)], 0.05, (-2.0, 0.25, ROOF + 0.06),
          (0.03, 0.02, 0.1), m["paint"], v.root, seed=111)
    shell = parts("vbci_upper", "vbci_lower", "side_hatch_", "bow_plate_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=16.0), dent((0.6, -1.49, 1.95), 0.55, 0.14, (0, 1, -0.1)))
    for k, (loc, rot, size) in enumerate((((-0.05, -1.30, 0.03), (0.03, 0.04, 0.9), 0.17),
                                          ((-0.05, 1.30, 0.03), (-0.04, 0.02, 2.2), 0.16),
                                          ((-1.8, 0.3, ROOF + 0.04), (0.02, 0.04, 0.3), 0.34))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12,
              seed=121 + k)
    v.root.rotation_euler = (0.01, 0.02, 0)
    v.root.location.z -= 0.08
    rest_on_ground(0.004)


run("vbci", "french_three_tone", build, wreck, chip=0.6)
