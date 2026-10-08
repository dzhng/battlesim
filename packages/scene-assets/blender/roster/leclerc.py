"""Leclerc XLR, from assets/references/leclerc/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/leclerc.py -- [--variant=<id>] [--wreck]

What the photos settle: one XLR photo (Eurosatory 2022, front left) and
pre-XLR Leclercs for the shared hull, running gear and turret core. Six large
evenly spaced road wheels a side with eight-bolt hubs, the sprocket raised at
the rear and the idler raised at the front, thin skirts with a top band; a
short steep glacis, the lamp clusters in boxes on its corners and shackles on
the nose plate. The turret is low and flat-faced, its armour modules either
side of the mantlet; the commander's panoramic sight tower on the left of the
roof with his cupola beside it, the gunner's sight box on the right front, the
long autoloader bustle behind, GALIX launchers along the bustle sides. XLR
(that one photo): stacked armour modules over the front of the hull sides,
bar armour over the rear and round the bustle, armour blocks on the turret
sides and the PL1 remote weapon station on the right of the roof. The
commander stands in his hatch (pre-XLR side photo).

Built to the catalog frame (hull 6.88 x 3.6 x 2.53 m, turret pivot 1.518 m,
cannon muzzle 6.43 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

TRACK_Y = 1.46
TRACK_W = 0.62
ROAD_R = 0.35
ROAD_Z = 0.43
ROAD_X = [1.95 - 0.78 * k for k in range(6)]
SPROCKET = (-2.78, 0.62, 0.31)
IDLER = (2.88, 0.60, 0.29)
RETURNS = [(1.15, 0.90, 0.09), (-0.55, 0.91, 0.09), (-2.05, 0.90, 0.09)]
DECK = 1.50
SIDE_Y = 1.80  # the skirts' outer face
FOOT = -0.02
ROOF = 0.64
TRUNNION = 1.30


def glacis_z(x):
    if x >= 2.70:
        return DECK - (x - 2.70) / 0.74 * 0.28
    return DECK


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.46, 0.40), pitch=0.16)
    hull_sides(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    main_gun(v, gun)
    weapon_station(v, hmg, hmg_gun)
    v.head_out("commander", turret, -0.30, 0.70, 1.518 + ROOF - 0.02)


# ---------------------------------------------------------------- hull
def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 1.00), (3.20, 1.00), (half, 1.22), (2.70, DECK), (-3.40, DECK), (-half, 1.44)],
          3.30, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.20, 0.48), (2.85, 0.48), (3.40, 1.02), (-half, 1.02), (-half, 0.84)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    VP.hatch("driver_hatch", (2.30, 0.55, DECK), m, hull, size=(0.56, 0.50))
    for k, y in enumerate((0.35, 0.55, 0.75)):
        VP.periscope(f"driver_periscope_{k}", (2.66, y, DECK), m, hull, size=(0.14, 0.16, 0.08))
    VP.weld_line("glacis_weld", [(2.70, -1.6, DECK + 0.004), (2.70, 1.6, DECK + 0.004)], m, hull)
    for side, s in ((1, "L"), (-1, "R")):
        # The lamp clusters in their boxes on the glacis corners.
        lamp = (3.05, side * 1.30, glacis_z(3.05))
        VP.bolted_plate(f"lamp_box_{s}", lamp, (0.30, 0.42, 0.18), m, hull, bolts=(1, 2), bevel=0.03)
        for k, dy in enumerate((-0.10, 0.10)):
            cyl(f"lamp_{s}_{k}", 0.065, 0.03, (lamp[0] + 0.16, lamp[1] + dy, lamp[2] + 0.10), "X", m["lamp"], hull,
                seg=14, lods=MID)
        VP.shackle(f"front_shackle_{s}", (3.40, side * 0.62, 0.92), m, hull, size=0.16, rot=(0, 0, math.pi / 2))
        VP.tow_hook(f"rear_tow_{s}", (-3.32, side * 0.78, 0.86), m, hull, size=0.13, rot=(0, 0, math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-3.36, side * 1.45, 1.36), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-3.20, side * 1.54, DECK + 0.025), (-1.6, side * 1.56, DECK + 0.025),
                                    (0.4, side * 1.56, DECK + 0.02), (2.1, side * 1.54, DECK + 0.02)], m, hull,
                 radius=0.022)
    stencil("nose_number", "6044-0116", 0.09, (3.441, 0.0, 1.10), (math.pi / 2, 0, math.pi / 2), m["marking"], hull)
    for k, y in enumerate((0.72, -0.72)):
        VP.grille(f"engine_grille_{k}", (-2.70, y, DECK), (1.10, 1.15), m, hull, slats=9)
        VP.bolted_plate(f"engine_door_{k}", (-1.75, y, DECK), (0.70, 1.10, 0.03), m, hull, bolts=(2, 2), bevel=0.01,
                        lods=MID)
    VP.grille("rear_louvre", (-half, 0, 1.18), (0.34, 2.40), m, hull, slats=7, rot=(0, -math.pi / 2, 0))
    VP.slat_armour("rear_bars", (-half - 0.06, 0, 0.66), (3.10, 0.70), m, hull, rot=(0, 0, math.pi / 2))


def hull_sides(v):
    """The thin skirt with its top band; XLR armour modules stacked over the
    front, bar armour standing off the rear."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        prism(f"skirt_{s}", [(-3.30, 0.82), (3.10, 0.82), (3.36, 1.10), (3.36, 1.46), (-3.30, 1.46)], 0.06,
              loc=(0, side * (SIDE_Y - 0.03), 0), mat=m["paint"], parent=hull, bevel=0.02)
        box(f"skirt_band_{s}", (6.6, 0.10, 0.10), (0.03, side * (SIDE_Y - 0.04), 1.42), m["paint"], hull,
            bevel=0.03, lods=MID)
        VP.armour_tiles(f"side_modules_{s}", (1.05, side * (SIDE_Y - 0.02), 1.08), (4.40, 0.62), (6, 2), 0.10, m,
                        hull, rot=(-side * math.pi / 2, 0, 0))
        VP.slat_armour(f"side_bars_{s}", (-2.25, side * (SIDE_Y + 0.03), 0.82), (2.10, 0.66), m, hull)
        stencil(f"side_number_{s}", "52", 0.22, (0.40, side * (SIDE_Y + 0.085), 1.12),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- turret
def turret_rings():
    """A low flat-faced turret: wide front modules, sides angling in, the
    long narrower autoloader bustle."""
    half = [(1.72, 0.44), (1.72, 1.10), (1.30, 1.36), (-0.70, 1.36), (-1.10, 1.08), (-2.30, 1.02), (-2.40, 0.90)]

    def ring(pts, lean):
        left = [(x, y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    return [(FOOT, ring(half, 0.0)), (ROOF, ring([(x - (0.10 if x > 1.0 else 0.0), y) for x, y in half], 0.06))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_rings(), mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.08, 0.10, (0, 0, FOOT - 0.04), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # The front armour modules either side of the mantlet.
        VP.bolted_plate(f"front_module_{s}", (1.82, side * 0.80, 0.05), (0.22, 0.66, 0.52), m, turret,
                        bolts=(1, 2), bevel=0.04)
        # Armour blocks on the turret sides.
        VP.armour_tiles(f"turret_blocks_{s}", (0.30, side * 1.36, 0.30), (1.80, 0.44), (3, 1), 0.12, m, turret,
                        rot=(-side * math.pi / 2, 0, 0))
        # GALIX launchers along the bustle sides, angled up and out.
        for k in range(2):
            VP.smoke_discharger_bank(f"galix_{s}_{k}", (-1.10 - k * 0.55, side * 1.10, ROOF - 0.04), m, turret,
                                     count=4, tube_radius=0.045, tube_length=0.22, elevation=0.85, spread=0.2,
                                     rot=(0, 0, side * math.pi / 2))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.80, side * 0.72, ROOF), m, whip, height=2.4)
    VP.slat_armour("bustle_bars", (-2.52, 0, -0.02), (2.10, 0.58), m, turret, rot=(0, 0, math.pi / 2))
    for side in (-1, 1):
        box(f"bustle_bar_arm_{side}", (0.16, 0.05, 0.05), (-2.44, side * 0.96, 0.50), m["dark"], turret, lods=MID)
    # Roof: the commander's sight tower and cupola on the left, the gunner's
    # sight box at the right front, the loading hatch on the bustle.
    cyl("commander_sight_tower", 0.22, 0.24, (0.62, 0.62, ROOF + 0.12), "Z", m["paint"], turret, seg=24,
        bevel=0.02)
    VP.sight_housing("commander_sight", (0.62, 0.62, ROOF + 0.22), m, turret, size=(0.40, 0.40, 0.20))
    VP.cupola("commander_cupola", (-0.30, 0.70, ROOF), m, turret, radius=0.36)
    VP.sight_housing("gunner_sight", (1.08, -0.72, ROOF - 0.06), m, turret, size=(0.50, 0.42, 0.26))
    VP.hatch("bustle_hatch", (-1.60, 0.0, ROOF), m, turret, size=(0.70, 0.60))
    VP.bolted_plate("roof_plate", (0.55, -0.20, ROOF), (0.70, 0.55, 0.03), m, turret, bolts=(2, 2), bevel=0.01,
                    lods=MID)


# ---------------------------------------------------------------- weapons
def main_gun(v, gun):
    """The 120 mm CN120-26: a compact mantlet, the long plain thermal
    sleeve with its bands (no bore evacuator) and the muzzle reference
    sensor's arm over the muzzle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.55, 0.66, 0.50), (0.22, 0, -0.02), m["paint"], gun, bevel=0.06)
    cyl("gun_collar", 0.19, 0.30, (0.62, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.02)
    cyl("coax_port", 0.035, 0.05, (0.50, 0.24, 0.10), "X", m["black"], gun, seg=10, lods=FINE)
    cyl("thermal_sleeve", 0.11, reach - 1.0, ((reach + 0.75) / 2, 0, 0), "X", m["paint"], gun, seg=24, r2=0.095)
    for k, x in enumerate((1.20, 2.40, 3.60, reach - 0.50)):
        cyl(f"sleeve_band_{k}", 0.112, 0.05, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("muzzle_end", 0.11, 0.26, (reach - 0.13, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.064, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)
    box("muzzle_reference_arm", (0.05, 0.04, 0.14), (reach - 0.30, 0, 0.16), m["dark"], gun, lods=NEAR)
    box("muzzle_reference", (0.16, 0.12, 0.10), (reach - 0.30, 0, 0.26), m["dark"], gun, bevel=0.015, lods=NEAR)


def weapon_station(v, hmg, hmg_gun):
    """The PL1 remote weapon station: a bearing, a squat armoured body with
    the sight block on its side and the 12.7 mm gun in its cradle."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    cyl("rws_bearing", 0.24, 0.10, (0, 0, -0.10), "Z", m["dark"], hmg, seg=24, bevel=0.012)
    box("rws_body", (0.44, 0.38, lift - 0.02), (-0.04, 0, (lift - 0.02) / 2 - 0.04), m["paint"], hmg, bevel=0.03)
    VP.sight_housing("rws_sight", (0.06, -0.28, -0.10), m, hmg_gun, size=(0.30, 0.16, 0.24))
    box("rws_ammo", (0.28, 0.14, 0.24), (-0.05, 0.26, -0.08), m["paint"], hmg_gun, bevel=0.02, lods=MID)
    VP.browning_m2(hmg_gun, muzzle[0], m)


# ---------------------------------------------------------------- wreck
def wreck(variant, v):
    """The tank after its cook-off: the left track thrown with two road
    wheels gone, side modules blown off the left front and lying by it, the
    rear bars bent, plates warped, the glacis dented, the bustle hatch blown
    off. `wreckage.burn` then heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_L_band", "wheel_L_2_", "wheel_L_5_", "side_modules_L_0_", "side_modules_L_1_", "bustle_hatch",
           "side_number_L")
    thrown = solid("thrown_track", (3.8, TRACK_W, 0.05), (-0.6, 2.25, 0.03), m["track"], v.hull, rot=(0, 0, -0.07),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=2.0))
    for k, (loc, rot) in enumerate((((2.4, 2.45, 0.08), (1.4, 0.2, 0.6)), ((1.6, 2.7, 0.08), (1.5, -0.1, -0.4)))):
        solid(f"fallen_module_{k}", (0.70, 0.29, 0.12), loc, m["paint"], v.hull, rot=rot, bevel=0.02)
    bend(parts("side_bars_R"), (-2.25, -1.83, 1.48), (1, 0, 0), (0, 0, -1), -0.5)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=8.0), dent((3.10, -0.40, 1.30), 0.5, 0.10, (-0.6, 0, -1)))
    plate("bustle_lid", [(-0.36, -0.3), (0.36, -0.32), (0.36, 0.3), (-0.34, 0.32)], 0.05, (-2.9, -0.7, DECK + 0.05),
          (0.06, -0.1, 0.4), m["paint"], v.hull, seed=19)
    for k, (loc, rot, size) in enumerate((((2.2, -2.2, 0.03), (0.04, 0.02, 0.3), 0.40),
                                          ((-2.8, 2.0, 0.03), (-0.03, 0.05, 1.8), 0.32),
                                          ((3.9, 0.8, 0.03), (0.0, 0.06, 2.6), 0.28))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=51 + k)
    rest_on_ground(0.004)


run("leclerc", "french_three_tone", build, wreck, chip=1.0)
