"""CV9040C and CV90 Mk IV, from assets/references/cv90/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/cv90.py -- [--variant=<id>] [--wreck]

What the photos settle: seven road wheels a side with no return rollers,
the track's top run lying on them, the drive sprocket at the front and the
idler at the rear; the nose plate with its lamps and shackles under a long
shallow glacis; tall flat hull sides in bolted armour panels, a tool rail
along their top; the rear door. The turret is low and angular: the 40 mm
Bofors in its mantlet on the front, its barrel in a perforated jacket with a
spring sleeve at its root and the large muzzle; smoke dischargers in a row
on the front face. CV9040C (Swedish and Ukrainian vehicles): add-on armour
boxes on the turret sides and front, the commander's sight on the roof, the
bustle wrapped in tarps. Mk IV (Czech and Slovak vehicles): an angular
faceted turret, the commander's panoramic sight on its mast, a Spike
launcher on the left and an independent viewer on the right. No rear or top
view of the Mk IV (gaps): its hull rear is the C's.

Built to the catalog frame (hull 6.8 x 3.2 x 2.8 m, turret pivot 1.82 m,
autocannon muzzle 3.9 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TRACK_Y = 1.27
TRACK_W = 0.53
ROAD_R = 0.31
ROAD_Z = 0.39
ROAD_X = [2.05 - 0.683 * k for k in range(7)]
SPROCKET = (2.88, 0.62, 0.28)  # front drive
IDLER = (-2.95, 0.48, 0.28)
ROOF_Z = 1.80
SIDE_Y = 1.62
FOOT = -0.02
ROOF = 0.72


def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.90), (3.15, 0.90), (half, 1.05), (half, 1.28), (1.30, ROOF_Z), (-3.35, ROOF_Z),
                         (-half, 1.75)], 3.00, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.10, 0.44), (2.85, 0.44), (3.30, 0.92), (-half, 0.92), (-half, 0.72)], 2.24,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan((ROOF_Z - 1.28) / (half - 1.30))
    VP.hatch("driver_hatch", (2.20, 0.60, 1.28 + (half - 2.20) * math.tan(slope)), m, hull, radius=0.28,
             rot=(0, slope, 0))
    for k, y in enumerate((0.40, 0.60, 0.80)):
        VP.periscope(f"driver_periscope_{k}", (2.55, y, 1.28 + (half - 2.55) * math.tan(slope)), m, hull,
                     size=(0.12, 0.14, 0.08), rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        for k, dy in enumerate((0.0, -0.22)):
            box(f"nose_lamp_{s}_{k}", (0.03, 0.16, 0.10), (half + 0.005, side * (1.30 + dy), 1.18), m["lamp"], hull,
                lods=MID)
        box(f"nose_lamp_hood_{s}", (0.08, 0.50, 0.03), (half - 0.02, side * 1.20, 1.25), m["paint"], hull, lods=NEAR)
        VP.shackle(f"front_shackle_{s}", (half - 0.02, side * 0.62, 0.86), m, hull, size=0.15, rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.10, side * 1.35, 1.55), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.12, side * 0.95, 0.80), m, hull, size=0.12, rot=(0, 0, math.pi))
        box(f"front_fender_{s}", (0.40, 0.52, 0.04), (3.10, side * TRACK_Y, 0.98), m["paint"], hull, bevel=0.012,
            rot=(0, 0.30, 0), lods=MID)
    VP.bolted_panel("rear_door", (-half - 0.01, 0, 0.80), (0.04, 1.10, 0.90), m, hull, bolts=(0, 0),
        bevel=0.02, lods=VP.ALL)
    box("rear_door_handle", (0.04, 0.20, 0.04), (-half - 0.04, 0.30, 1.30), m["steel"], hull, lods=NEAR)
    VP.grille("engine_grille", (2.00, -0.75, 1.28 + (half - 2.00) * math.tan(slope) - 0.01), (0.70, 0.90), m, hull,
              slats=7, rot=(0, slope, 0))
    VP.hatch("roof_hatch_L", (-2.20, 0.55, ROOF_Z), m, hull, size=(0.80, 0.70))
    VP.hatch("roof_hatch_R", (-2.20, -0.55, ROOF_Z), m, hull, size=(0.80, 0.70))


def hull_sides(v, mk4):
    """The tall hull sides in bolted armour panels, a tool rail along their
    top; the Mk IV's panels larger and flatter."""
    m, hull = v.mats, v.hull
    cols = 4 if mk4 else 6
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"side_armour_{s}", (-0.15, side * (SIDE_Y - 0.08), 1.18), (6.20, 1.08), (cols, 2), 0.08,
                        m, hull, rot=(-side * math.pi / 2, 0, 0))
        box(f"tool_rail_{s}", (5.8, 0.04, 0.04), (-0.20, side * (SIDE_Y - 0.02), 1.76), m["steel"], hull, lods=MID)
        for k in range(5):
            box(f"rail_bracket_{s}_{k}", (0.04, 0.08, 0.06), (2.4 - k * 1.2, side * (SIDE_Y - 0.05), 1.76), m["dark"],
                hull, lods=NEAR)
        box(f"skirt_lip_{s}", (6.40, 0.10, 0.06), (-0.10, side * (SIDE_Y - 0.06), 0.62), m["paint"], hull,
            bevel=0.02, lods=MID)
        box(f"reflector_{s}", (0.10, 0.01, 0.10), (2.60, side * (SIDE_Y + 0.001), 1.02), m["tail"], hull, lods=NEAR)


def turret_body(v, turret, mk4):
    m = v.mats
    half = [(1.10, 0.30), (1.00, 0.95), (-0.95, 1.05), (-1.30, 0.85)]

    def ring(pts, lean, nose):
        left = [(x - (nose if x > 0.9 else 0.0), y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    rings = [(FOOT, ring(half, 0.0, 0.0)), (0.36, ring(half, -0.08 if mk4 else 0.0, 0.0)),
             (ROOF, ring(half, 0.12, 0.30))]
    loft("turret_shell", rings, mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.00, 0.08, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=36, lods=MID)
    VP.smoke_discharger_bank("smoke_L", (0.75, 0.62, 0.42), m, turret, count=4, tube_radius=0.045,
                             tube_length=0.18, elevation=0.25, spread=0.3, rot=(0, 0, 0.35))
    VP.smoke_discharger_bank("smoke_R", (0.75, -0.62, 0.42), m, turret, count=4, tube_radius=0.045,
                             tube_length=0.18, elevation=0.25, spread=0.3, rot=(0, 0, -0.35))
    for side, s in ((1, "L"), (-1, "R")):
        if not mk4:
            VP.bolted_panel(f"turret_addon_{s}", (-0.10, side * 1.02, 0.08), (1.40, 0.10, 0.56), m, turret,
                            bolts=(4, 1), bevel=0.03, lods=VP.ALL)
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.10, side * 0.70, ROOF), m, whip, height=2.0)
    VP.sight_housing("gunner_sight", (0.55, 0.55, ROOF - 0.04), m, turret, size=(0.40, 0.32, 0.22))
    VP.hatch("commander_hatch", (-0.35, -0.45, ROOF), m, turret, radius=0.28)
    VP.hatch("gunner_hatch", (-0.35, 0.45, ROOF), m, turret, radius=0.26)
    if mk4:
        mast = empty("dressing_commander_sight", parent=turret)
        cyl("sight_mast", 0.10, 0.28, (0.20, -0.55, ROOF + 0.14), "Z", m["dark"], mast, seg=16, lods=MID)
        cyl("sight_head", 0.14, 0.24, (0.20, -0.55, ROOF + 0.38), "Z", m["paint"], mast, seg=20, bevel=0.02)
        box("sight_head_window", (0.02, 0.16, 0.10), (0.34, -0.55, ROOF + 0.38), m["glass"], mast, lods=MID)
        # The Spike launcher on its arm on the left, the viewer on the right.
        box("spike_arm", (0.20, 0.14, 0.24), (-0.30, 1.00, ROOF + 0.06), m["dark"], turret, bevel=0.02)
        for k, z in enumerate((ROOF + 0.09, ROOF + 0.26)):
            cyl(f"spike_tube_{k}", 0.08, 0.90, (-0.25, 1.02, z), "X", m["paint"], turret, seg=16, bevel=0.01)
        VP.sight_housing("viewer", (0.30, -0.95, ROOF - 0.02), m, turret, size=(0.34, 0.26, 0.24))
    else:
        VP.sight_housing("commander_sight", (0.20, -0.60, ROOF - 0.02), m, turret, size=(0.38, 0.30, 0.26))
        VP.tarp_roll("bustle_tarp", (-1.20, 0.0, ROOF - 0.05), 1.60, 0.16, m, turret, straps=3)
        box("bustle_bag", (0.40, 0.70, 0.36), (-1.40, 0.55, 0.40), m["canvas"], turret, bevel=0.08, lods=MID)


def bofors(v, gun):
    """The 40 mm Bofors: a boxy mantlet, the recoil spring sleeve at the
    barrel's root, the perforated jacket and the large muzzle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.44, 0.52, 0.40), (1.05, 0, -0.02), m["paint"], gun, bevel=0.05)
    cyl("spring_sleeve", 0.095, 0.55, (1.55, 0, 0), "X", m["dark"], gun, seg=20, lods=MID)
    for k in range(6):
        cyl(f"spring_coil_{k}", 0.105, 0.03, (1.32 + k * 0.09, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("barrel_jacket", 0.065, reach - 2.30, ((reach + 1.80) / 2 - 0.25, 0, 0), "X", m["paint"], gun, seg=18)
    for k in range(5):
        box(f"jacket_slot_{k}", (0.20, 0.02, 0.03), (2.10 + k * 0.30, 0, 0.065), m["black"], gun, lods=FINE)
    cyl("muzzle", 0.10, 0.40, (reach - 0.20, 0, 0), "X", m["paint"], gun, seg=20, bevel=0.015)
    cyl("muzzle_bore", 0.03, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=12, lods=MID)


def build(variant, v):
    mk4 = "mk_iv" in variant["id"]
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.20, SPROCKET, IDLER, (),
                            bolts=6, teeth=11, arm=(0.40, 0.40), pitch=0.15)
    hull_sides(v, mk4)
    mounts = rig(v.frame, v.root)
    turret, gun, _, _ = mounts["autocannon"]
    turret_body(v, turret, mk4)
    bofors(v, gun)
    if not mk4:
        v.head_out("commander", turret, -0.35, -0.45, 1.82 + ROOF - 0.03)


def wreck(variant, v):
    """The vehicle after its fire: the right track off and lying along the
    hull, two road wheels gone, side panels blown off and lying at its foot,
    the rear door blown open, plates warped. `wreckage.burn` heaves the
    turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_3_", "wheel_R_5_", "side_armour_R_1_", "side_armour_R_2_", "rear_door",
           "bustle_tarp", "bustle_bag", "roof_hatch_R")
    thrown = solid("thrown_track", (3.6, TRACK_W, 0.05), (-0.2, -1.74, 0.03), m["track"], v.hull, rot=(0, 0, 0.03),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=6.0))
    for k, x in enumerate((1.4, 0.2)):
        solid(f"fallen_panel_{k}", (1.40, 0.08, 0.50), (x, -1.80, 0.28), m["paint"], v.hull, rot=(-0.55, 0.0, 0.05),
              bevel=0.02)
    shell = parts("hull_upper", "hull_lower", "side_armour_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.026, 0.9, seed=8.0), dent((2.80, 0.50, 1.30), 0.45, 0.10, (-0.6, 0, -1)))
    plate("door_fallen", [(-0.5, -0.42), (0.5, -0.45), (0.52, 0.42), (-0.5, 0.45)], 0.04, (-3.0, 0.6, 0.30),
          (0.0, 1.0, 0.2), m["paint"], v.hull, seed=29)
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("cv90", "swedish_splinter", build, wreck, chip=1.0)
