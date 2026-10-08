"""ACV-P (Amphibious Combat Vehicle, personnel), from assets/references/acv/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/acv.py -- [--wreck]

What the photos settle: eight big black tyres on green rims, four evenly
spaced axles well back from a long bow; a narrow lower hull, and over it the
tall upper hull flaring out over the wheels and leaning back in to the roof;
the wedge prow, an upper and a lower plate meeting at a sharp edge, with
lights recessed in its corners; the roof's hatches, the remote station with
its heavy machine gun forward of centre, stowage tubes racked on the rear
right, antennas at the rear corners; the rear: the ramp with its door
between two flared corner housings, and under each a propeller in its
shroud; tail lights in the housings.

Built to the catalog frame (hull 9.2 x 3.1 x 2.9 m, HMG pivot 3.03 m):
nothing here moves it. The roof is drawn at 2.70 m; the station's fixed
base rises to the frame's top.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [2.45, 0.90, -0.65, -2.20]
WHEEL_R = 0.59
WHEEL_W = 0.38
WHEEL_Y = 1.28
BELLY = 0.70
SHELF = 1.32
FLARE = 1.80
ROOF = 2.70
BOW = 4.60
EDGE_Z = 1.55
# The flared upper side leaning back in to the roof, as (y, z) at its foot
# and top (`vehicle_parts.on_side`).
UPPER_SIDE = ((1.55, FLARE), (1.32, ROOF))
GLACIS_TOP = 3.00


def glacis_z(x):
    return EDGE_Z + (ROOF - EDGE_Z) * (BOW - x) / (BOW - GLACIS_TOP)


def build(variant, v):
    m, hull = v.mats, v.hull
    loft("acv_lower", [(BELLY, VP.hull_plan(-4.35, 3.40, 0.95, 0.35)), (SHELF, VP.hull_plan(-4.50, 3.90, 1.00, 0.45))],
         mat=m["dark"], parent=hull, bevel=0.04)
    loft("acv_upper", [(SHELF, VP.hull_plan(-4.55, 4.05, 1.30, 0.70)),
                       (EDGE_Z, VP.hull_plan(-4.60, BOW, 1.50, 0.85)),
                       (FLARE, VP.hull_plan(-4.60, 4.35, UPPER_SIDE[0][0], 0.80)),
                       (ROOF, VP.hull_plan(-4.50, GLACIS_TOP, UPPER_SIDE[1][0], 0.45))], mat=m["paint"], parent=hull, bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.33, ctis=True, hub_bolts=10)
            box(f"suspension_arm_{s}_{k}", (0.20, 0.36, 0.12), (x, side * (WHEEL_Y - 0.34), WHEEL_R), m["dark"],
                hull, lods=NEAR)
            cyl(f"strut_{s}_{k}", 0.07, 0.55, (x + 0.30, side * 1.05, WHEEL_R + 0.35), "Z", m["dark"], hull, seg=10,
                lods=FINE)
    fittings(v)
    station, pitch, _, pivot = rig(v.frame, v.root)["HMG"]
    protector(v, station, pitch, pivot)


def fittings(v):
    m, hull = v.mats, v.hull
    slope = math.atan((ROOF - EDGE_Z) / (BOW - GLACIS_TOP))
    # The prow: bolted plates on its upper face, lights in its corners, tow
    # shackles under the edge.
    for k, y in enumerate((0.65, -0.65)):
        VP.bolted_panel(f"prow_plate_{k}", (3.85, y, glacis_z(3.85)), (0.95, 1.15, 0.035), m, hull, bolts=(3, 4),
                        rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        box(f"light_recess_{s}", (0.30, 0.30, 0.20), (4.12, side * 1.18, EDGE_Z + 0.12), m["black"], hull, lods=MID)
        for k, y in enumerate((1.10, 1.26)):
            cyl(f"headlight_{s}_{k}", 0.055, 0.03, (4.26, side * y, EDGE_Z + 0.12), "X", m["lamp"], hull, seg=12,
                lods=MID)
        VP.shackle(f"prow_shackle_{s}", (4.30, side * 0.50, EDGE_Z - 0.12), m, hull, size=0.14,
                   rot=(0, 0, math.pi / 2))
        # Bolted panels along the flared upper side.
        for k, x in enumerate((2.70, 1.20, -0.30, -1.80, -3.30)):
            loc, rot = VP.on_side(x, 2.15, side, *UPPER_SIDE)
            VP.bolted_panel(f"side_panel_{s}_{k}", loc, (1.35, 0.62, 0.025), m, hull, bolts=(4, 3), rot=rot,
                            bevel=0.012)
        # The rear: a flared corner housing with its tail light, and under it
        # the propeller in its shroud.
        box(f"rear_housing_{s}", (0.40, 0.95, 1.10), (-4.42, side * 1.05, 2.05), m["paint"], hull, bevel=0.05)
        box(f"tail_lamp_{s}", (0.03, 0.12, 0.10), (-4.625, side * 1.30, 1.85), m["tail"], hull, lods=MID)
        prop = (-4.45, side * 1.05, 1.02)
        cyl(f"prop_shroud_{s}", 0.36, 0.22, prop, "X", m["paint"], hull, seg=24, bevel=0.02)
        cyl(f"prop_hole_{s}", 0.31, 0.24, prop, "X", m["black"], hull, seg=24, lods=MID)
        cyl(f"prop_hub_{s}", 0.07, 0.14, (prop[0] - 0.08, prop[1], prop[2]), "X", m["dark"], hull, seg=12, lods=MID)
        for j in range(3):
            a = j * math.tau / 3
            box(f"prop_blade_{s}_{j}", (0.03, 0.10, 0.26), (prop[0] - 0.08, prop[1] + 0.14 * math.sin(a),
                                                           prop[2] + 0.14 * math.cos(a)),
                m["dark"], hull, rot=(-a, 0.4, 0), lods=NEAR)
    # The ramp and its door between the housings.
    box("rear_ramp", (0.06, 1.10, ROOF - 0.90), (-4.58, 0, (ROOF + 0.90) / 2), m["paint"], hull, bevel=0.02)
    box("ramp_door", (0.03, 0.62, 1.15), (-4.62, 0.0, 1.85), m["paint"], hull, bevel=0.012, lods=NEAR)
    cyl("ramp_hinge", 0.05, 1.00, (-4.60, 0, 0.92), "Y", m["steel"], hull, seg=10, lods=NEAR)
    # Roof: hatches, the vents, the stowage tubes, antennas.
    for k, (x, y) in enumerate(((-1.20, 0.65), (-1.20, -0.65), (-2.40, 0.65))):
        VP.hatch(f"roof_hatch_{k}", (x, y, ROOF), m, hull, size=(0.85, 0.66))
    VP.hatch("driver_hatch", (2.55, 0.70, ROOF), m, hull, radius=0.30)
    for k, y in enumerate((0.45, 0.70, 0.95)):
        VP.periscope(f"driver_periscope_{k}", (2.95, y, ROOF), m, hull, size=(0.12, 0.17, 0.09))
    VP.grille("engine_grille", (2.30, -0.65, ROOF), (0.90, 0.70), m, hull, slats=8)
    rack = empty("dressing_stowage_tubes", parent=hull)
    for k, y in enumerate((-0.80, -1.12)):
        cyl(f"stowage_tube_{k}", 0.15, 1.60, (-3.10, y, ROOF + 0.15), "X", dict(m, paint=m["dark"])["paint"], rack,
            seg=16, bevel=0.02)
    for k, x in enumerate((-3.70, -2.50)):
        box(f"tube_rack_{k}", (0.06, 0.70, 0.18), (x, -0.96, ROOF + 0.09), m["steel"], rack, lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-4.20, side * 1.15, ROOF), m, whip, height=2.8)
    VP.exhaust("exhaust", (1.20, -1.58, 2.05), 0.08, 0.40, m, hull, rot=(0, 0, -math.pi / 2))


def protector(v, station, pitch, pivot):
    """The remote station on its fixed base, its heavy machine gun, the sight
    on its left and the ammunition box on its right."""
    m = v.mats
    cyl("station_adapter", 0.36, 2.86 - ROOF, (pivot.x, pivot.y, (2.86 + ROOF) / 2), "Z", m["paint"], v.hull, seg=24,
        bevel=0.012)
    cyl("station_bearing", 0.32, 0.05, (pivot.x, pivot.y, 2.885), "Z", m["dark"], v.hull, seg=24, lods=MID)
    below = 2.91 - pivot.z
    cyl("station_base", 0.30, 0.10, (0, 0, below + 0.05), "Z", m["paint"], station, seg=20, bevel=0.01)
    for side in (-1, 1):
        box(f"station_cradle_{side}", (0.38, 0.05, 0.30), (0.0, side * 0.17, -0.02), m["paint"], station,
            bevel=0.012)
    VP.sight_housing("station_sight", (0.08, 0.32, -0.10), m, pitch, size=(0.32, 0.20, 0.24))
    box("station_ammo", (0.32, 0.15, 0.26), (-0.02, -0.30, -0.08), m["paint"], pitch, bevel=0.012, lods=MID)
    VP.browning_m2(pitch, v.frame["mounts"][0]["muzzle_m"][0], m)


def wreck(variant, v):
    """The ACV after its fire: the front left wheel pair blown off and the
    prow down on that side, the ramp's door blown out and lying beside it,
    the stowage tubes burnt off, a prop shroud torn, hatches gone, plates
    warped and a dent in the right side, torn plate on the ground. Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "wheel_L_2_", "ramp_door", "dressing_stowage_tubes", "stowage_tube_", "tube_rack_",
           "roof_hatch_", "prop_blade_R")
    plate("door_lying", [(-0.57, -0.31), (0.58, -0.32), (0.57, 0.31), (-0.58, 0.30)], 0.05, (-3.2, 1.75, 0.04),
          (0.02, 0.03, 0.15), m["paint"], v.root, seed=171)
    shell = parts("acv_upper", "acv_lower", "side_panel_", "prow_plate_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=22.0), dent((0.5, -1.52, 2.10), 0.6, 0.14, (0, 1, -0.1)))
    for k, (loc, rot, size) in enumerate((((3.0, 1.95, 0.03), (0.03, 0.04, 0.9), 0.26),
                                          ((0.0, -1.95, 0.03), (-0.04, 0.02, 2.2), 0.24),
                                          ((-2.0, 0.3, ROOF + 0.04), (0.02, 0.04, 0.3), 0.34))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12,
              seed=181 + k)
    v.root.rotation_euler = (-0.04, 0.045, 0)
    v.root.location.z -= 0.08
    rest_on_ground(0.004)


run("acv", "us_desert_tan", build, wreck, chip=0.6)
