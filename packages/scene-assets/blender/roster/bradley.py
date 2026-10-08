"""M2A4 Bradley IFV and M3A3 Bradley CFV, from assets/references/bradley/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/bradley.py -- [--variant=<id>] [--wreck]

What the photos settle: six dual road wheels a side, three return rollers,
the drive sprocket at the front (the Bradley is front-driven) and the idler
at the rear; the upper hull sides in bolted applique tiles over a skirt of
panels with lifting handles; a steep glacis with the headlight clusters in
their guards on its corners and the folded trim vane across it; the big
rear ramp with its door, tail lights in guards and tow shackles. The turret
is a squared box: the 25 mm Bushmaster in its mantlet on the front, the
twin TOW launcher box on its left side, the gunner's sight on the front
right of the roof, the commander's independent viewer on its pedestal at
the rear, stowage baskets round the bustle and smoke dischargers either side
of the mantlet. M2A4 and M3A3 differ little outside (gaps): the M2A4 keeps
its firing port plates on the rear hull sides, the M3A3 carries more
stowage on the rear sponsons.

Paint: US desert tan (`us_desert_tan`), as the M3A3 photos and the Abrams;
the M2A4 photos in Europe show green, a scheme slice 13 does not have.

Built to the catalog frame (hull 6.55 x 3.6 x 2.98 m, turret pivot 1.937 m,
autocannon muzzle 3.6 m ahead, launcher on the left at 0.972 m): nothing here
moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TRACK_Y = 1.30
TRACK_W = 0.53
ROAD_R = 0.30
ROAD_Z = 0.38
ROAD_X = [2.05 - 0.80 * k for k in range(6)]
SPROCKET = (2.85, 0.62, 0.28)  # front drive
IDLER = (-2.88, 0.52, 0.27)
RETURNS = [(1.25, 0.76, 0.08), (-0.10, 0.77, 0.08), (-1.45, 0.76, 0.08)]
ROOF_Z = 1.92
SIDE_Y = 1.70  # the applique's outer face
FOOT = -0.02
ROOF = 0.60


def hull_body(v, m3):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.90), (3.05, 0.90), (half, 1.20), (2.05, ROOF_Z), (-3.20, ROOF_Z),
                         (-half, 1.84)], 3.20, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.05, 0.44), (2.75, 0.44), (3.20, 0.92), (-half, 0.92), (-half, 0.72)], 2.30,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan((ROOF_Z - 1.20) / (half - 2.05))
    # The folded trim vane across the glacis, the driver's hatch at its head.
    VP.bolted_panel("trim_vane", (2.66, 0.0, 1.56), (0.80, 2.20, 0.05), m, hull, bolts=(2, 4), bevel=0.02,
                    rot=(0, slope, 0), lods=VP.ALL)
    VP.hatch("driver_hatch", (1.75, 0.62, ROOF_Z), m, hull, radius=0.30)
    for k, a in enumerate((-0.6, 0.0, 0.6)):
        VP.periscope(f"driver_periscope_{k}", (2.02, 0.62 + a * 0.30, ROOF_Z), m, hull, size=(0.12, 0.16, 0.08),
                     rot=(0, 0, a * 0.5))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (3.05, side * 1.40, 1.30), 0.07, m, hull)
        VP.light_with_guard(f"headlight_b_{s}", (3.05, side * 1.20, 1.30), 0.06, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.12, side * 0.70, 0.80), m, hull, size=0.13)
        VP.shackle(f"rear_shackle_{s}", (-3.25, side * 0.95, 0.72), m, hull, size=0.13, rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-3.22, side * 1.38, 1.62), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        box(f"front_fender_{s}", (0.40, 0.56, 0.04), (3.05, side * TRACK_Y, 0.98), m["paint"], hull, bevel=0.012,
            rot=(0, 0.30, 0), lods=MID)
    # The rear ramp, its door and the cable coiled round it.
    VP.bolted_panel("rear_ramp", (-half + 0.0, 0, 0.70), (0.08, 1.80, 1.10), m, hull, bolts=(0, 0),
        bevel=0.03, lods=VP.ALL)
    cyl("ramp_door", 0.42, 0.04, (-half - 0.05, 0.35, 1.30), "X", m["paint"], hull, seg=24, bevel=0.012, lods=MID)
    VP.cable("ramp_cable", [(-half - 0.055, 0.80, 1.60), (-half - 0.055, 0.80, 0.92), (-half - 0.055, -0.40, 0.84),
                            (-half - 0.055, -0.75, 1.40)], m, hull, radius=0.018, eyes=False)
    # Roof: the squad's cargo hatch behind the turret, the engine grille ahead.
    VP.hatch("cargo_hatch", (-2.45, 0.0, ROOF_Z), m, hull, size=(1.10, 1.20))
    VP.grille("engine_grille", (1.55, -0.72, ROOF_Z), (0.80, 0.80), m, hull, slats=7)
    VP.exhaust("exhaust", (1.30, -1.62, 1.75), 0.08, 0.30, m, hull, rot=(0, 0, -math.pi / 2))
    if m3:
        for side, s in ((1, "L"), (-1, "R")):
            VP.stowage_box(f"sponson_box_{s}", (-2.60, side * 1.30, ROOF_Z), (0.80, 0.50, 0.30), m, hull)
            VP.jerrycan(f"sponson_can_{s}", (-3.05, side * 1.25, ROOF_Z), dict(m, paint=m["dark"]), hull,
                        rot=(0, 0, math.pi / 2))


def hull_sides(v, m3):
    """Bolted applique tiles on the upper sides, the skirt of panels with
    lifting handles under them; the M2A4's firing port plates at the rear."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"applique_{s}", (-0.10, side * (SIDE_Y - 0.10), 1.43), (6.10, 0.82), (8, 2), 0.10, m, hull,
                        rot=(-side * math.pi / 2, 0, 0))
        for k in range(6):
            x = 2.55 - k * 1.02
            VP.bolted_panel(f"lower_skirt_{s}_{k}", (x, side * (SIDE_Y - 0.06), 0.50), (0.98, 0.06, 0.52), m, hull,
                            bolts=(2, 1), bevel=0.02, lods=VP.ALL)
            box(f"skirt_handle_{s}_{k}", (0.14, 0.04, 0.05), (x, side * (SIDE_Y - 0.01), 0.64), m["dark"], hull,
                lods=NEAR)
        if not m3:
            for k in range(2):
                box(f"firing_port_{s}_{k}", (0.22, 0.03, 0.22), (-2.10 - k * 0.55, side * (SIDE_Y + 0.005), 1.62),
                    m["dark"], hull, bevel=0.01, lods=MID)
        stencil(f"side_bumper_{s}", "B30" if m3 else "A12", 0.16, (1.10, side * (SIDE_Y + 0.003), 1.10),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


def turret_body(v, turret):
    m = v.mats
    half = [(1.05, 0.30), (0.95, 0.95), (-1.10, 0.98), (-1.30, 0.85)]

    def ring(pts, lean, nose):
        left = [(x - (nose if x > 0.9 else 0.0), y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    loft("turret_shell", [(FOOT, ring(half, 0.0, 0.0)), (ROOF, ring(half, 0.08, 0.25))], mat=m["paint"],
         parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.00, 0.08, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=36, lods=MID)
    VP.armour_tiles("turret_applique_front", (0.95, 0, 0.30), (0.50, 1.70), (1, 4), 0.07, m, turret,
                    rot=(0, math.pi / 2 - 0.30, 0))
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 0.72, ROOF - 0.04), m, turret, count=4,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.30, spread=0.4,
                                 rot=(0, 0, side * 0.4))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.15, side * 0.70, ROOF), m, whip, height=2.0)
    # Stowage baskets round the bustle.
    for side in (-1, 1):
        box(f"basket_rail_{side}", (1.30, 0.03, 0.03), (-0.75, side * 1.12, ROOF - 0.02), m["steel"], turret,
            lods=MID)
        for j in range(4):
            box(f"basket_post_{side}_{j}", (0.03, 0.03, 0.40), (-0.20 - j * 0.38, side * 1.12, ROOF - 0.22),
                m["steel"], turret, lods=NEAR)
        box(f"basket_floor_{side}", (1.30, 0.16, 0.03), (-0.75, side * 1.04, 0.18), m["steel"], turret, lods=MID)
        box(f"basket_load_{side}", (1.10, 0.14, 0.26), (-0.75, side * 1.04, 0.34), m["canvas"], turret, bevel=0.05,
            lods=MID)
    box("basket_rear_rail", (0.03, 2.00, 0.03), (-1.45, 0, ROOF - 0.02), m["steel"], turret, lods=MID)
    VP.tarp_roll("basket_tarp", (-1.40, 0.0, ROOF - 0.20), 1.60, 0.16, m, turret, straps=3)
    # Roof: the gunner's sight at the front right, the commander's
    # independent viewer on its pedestal at the rear, the hatches.
    VP.sight_housing("gunner_sight", (0.55, -0.55, ROOF - 0.04), m, turret, size=(0.42, 0.34, 0.24))
    cyl("civ_pedestal", 0.17, 0.16, (-0.70, -0.40, ROOF + 0.08), "Z", m["paint"], turret, seg=20, bevel=0.02)
    cyl("civ_head", 0.22, 0.30, (-0.70, -0.40, ROOF + 0.30), "Y", m["paint"], turret, seg=24, bevel=0.03)
    box("civ_window", (0.02, 0.24, 0.16), (-0.48, -0.40, ROOF + 0.30), m["glass"], turret, lods=MID)
    VP.hatch("commander_hatch", (-0.30, -0.40, ROOF), m, turret, radius=0.28)
    VP.hatch("gunner_hatch", (-0.30, 0.45, ROOF), m, turret, radius=0.26)


def autocannon(v, gun):
    """The 25 mm M242 Bushmaster in its mantlet, the coaxial gun beside it."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.40, 0.48, 0.34), (0.95, 0, -0.04), m["paint"], gun, bevel=0.04)
    cyl("cannon_sleeve", 0.075, 0.70, (1.45, 0, 0), "X", m["paint"], gun, seg=20, bevel=0.01)
    cyl("cannon_barrel", 0.040, reach - 1.75, ((reach + 1.75) / 2, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("cannon_muzzle", 0.055, 0.16, (reach - 0.08, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("cannon_bore", 0.022, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=MID)
    cyl("coax_barrel", 0.020, 0.40, (1.30, -0.20, 0.0), "X", m["dark"], gun, seg=8, lods=NEAR)


def tow_launcher(v, launcher, launcher_tube):
    """The twin TOW launcher box on its arm on the turret's left side."""
    m = v.mats
    reach = v.frame["mounts"][1]["muzzle_m"][0]
    box("launcher_arm", (0.30, 0.20, 0.30), (-0.05, -0.10, -0.10), m["dark"], launcher, bevel=0.02)
    box("launcher_box", (reach + 0.30, 0.40, 0.48), ((reach - 0.30) / 2, 0.08, 0.0), m["paint"], launcher_tube,
        bevel=0.04)
    for k, z in enumerate((0.12, -0.12)):
        cyl(f"launcher_mouth_{k}", 0.085, 0.02, (reach + 0.005, 0.08, z), "X", m["black"], launcher_tube, seg=16,
            lods=MID)


def build(variant, v):
    m3 = "m3a3" in variant["id"]
    hull_body(v, m3)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.20, SPROCKET, IDLER,
                            RETURNS, bolts=6, teeth=11, arm=(0.42, 0.40), pitch=0.15)
    hull_sides(v, m3)
    mounts = rig(v.frame, v.root)
    turret, gun, _, _ = mounts["autocannon"]
    launcher, launcher_tube, _, _ = mounts["launcher"]
    turret_body(v, turret)
    autocannon(v, gun)
    tow_launcher(v, launcher, launcher_tube)


def wreck(variant, v):
    """The vehicle after its fire: the left track off with two road wheels
    gone, applique tiles blown off the left side and lying by it, the skirt
    bent out, the cargo hatch and ramp door blown open, plates warped by the
    aluminium hull's fire. `wreckage.burn` then heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_L_band", "wheel_L_2_", "wheel_L_5_", "applique_L_3_", "applique_L_4_", "cargo_hatch", "ramp_door",
           "basket_load_", "basket_tarp", "side_bumper_L", "sponson_box_", "sponson_can_")
    thrown = solid("thrown_track", (3.4, TRACK_W, 0.05), (-0.4, 1.82, 0.03), m["track"], v.hull, rot=(0, 0, -0.03),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=2.0))
    for k, loc in enumerate(((1.6, 1.98, 0.22), (2.4, 1.98, 0.20))):
        solid(f"fallen_tile_{k}", (0.70, 0.10, 0.38), loc, m["paint"], v.hull, rot=(0.5, 0.05, 0.1 * k), bevel=0.02)
    bend(parts("lower_skirt_R_2"), (0.51, -1.64, 1.02), (1, 0, 0), (0, 0, -1), -0.5)
    shell = parts("hull_upper", "hull_lower", "applique_", "turret_shell", "rear_ramp")
    densify(shell, scale=2.0)
    warp(shell, heat(0.03, 0.9, seed=4.0), dent((2.70, 0.60, 1.40), 0.45, 0.10, (-0.6, 0, -1)))
    plate("ramp_door_fallen", [(-0.4, -0.38), (0.42, -0.40), (0.42, 0.38), (-0.4, 0.40)], 0.04, (-2.95, -0.2, 0.40),
          (0.0, 1.2, 0.3), m["paint"], v.hull, seed=23)
    for k, (loc, rot, size) in enumerate((((2.0, -1.90, 0.03), (0.04, 0.02, 0.3), 0.26),
                                          ((-2.4, -1.88, 0.03), (-0.03, 0.05, 1.8), 0.24))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=91 + k)
    rest_on_ground(0.004)


run("bradley", "us_desert_tan", build, wreck, chip=1.0)
