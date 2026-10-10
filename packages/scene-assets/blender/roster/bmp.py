"""BMP-2M Berezhok and BMP-3, from assets/references/bmp/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/bmp.py -- [--variant=<id>] [--wreck]

What the photos settle. BMP-2M (Engineering Technologies 2010, Army-2020):
six road wheels with ribbed discs a side, three return rollers, the drive
sprocket at the front and the idler at the rear; the long ribbed glacis
(the trim-vane plate) running to a sharp nose, the driver's hatch and the
commander's behind it on the left, the engine grille on the right of the
glacis; flat sloped sides with a stowage bin run along their top; the two
rear doors. The low Berezhok turret: the 30 mm 2A42 with its slotted
muzzle, the AG-30 grenade launcher on the right, the Kornet launcher pair on
the left arm, sight boxes on the roof and smoke tubes either side.
BMP-3: its own longer, boxier hull with the engine at the rear, the
sprocket at the rear and the idler at the front, six road wheels and four
return rollers, a skirt with a long bolted rail over the tracks, the
bow's splash board and the twin machine guns' ports; the cast turret with
the 100 mm 2A70 and the 30 mm 2A72 beside it on the right, sights on the
roof, smoke tubes on the front corners. The BMP-3's commander rides head
out (photo: side).

Built to the catalog frames (BMP-2M hull 6.735 x 3.15 x 2.45 m, turret pivot
1.593 m, 30 mm muzzle 2.829 m ahead, launcher on the left at 0.851 m;
BMP-3 7.14 x 3.2 x 2.4 m, pivot 1.6 m, 100 mm muzzle 3.8 m ahead, 30 mm on
the right at 0.22 m; both rings 0.3 m ahead of amidships, as the photos put
them): nothing here moves them.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID


class Bmp2:
    TRACK_Y = 1.24
    TRACK_W = 0.30
    ROAD_R = 0.36
    ROAD_Z = 0.44
    ROAD_X = [1.92 - 0.75 * k for k in range(6)]
    SPROCKET = (2.70, 0.58, 0.30)
    IDLER = (-2.92, 0.50, 0.30)
    RETURNS = [(1.20, 0.86, 0.08), (0.10, 0.87, 0.08), (-1.00, 0.86, 0.08)]
    ROOF_Z = 1.62


class Bmp3:
    TRACK_Y = 1.30
    TRACK_W = 0.38
    ROAD_R = 0.35
    ROAD_Z = 0.43
    ROAD_X = [2.25 - 0.86 * k for k in range(6)]
    SPROCKET = (-3.00, 0.58, 0.30)
    IDLER = (2.95, 0.55, 0.28)
    RETURNS = [(1.75, 0.86, 0.08), (0.60, 0.87, 0.08), (-0.55, 0.87, 0.08), (-1.70, 0.86, 0.08)]
    ROOF_Z = 1.62


# ---------------------------------------------------------------- BMP-2M
def bmp2_hull(v):
    m, hull = v.mats, v.hull
    g = Bmp2
    half = v.length / 2
    prism("hull_upper", [(-half, 0.86), (2.80, 0.86), (half, 0.95), (0.70, g.ROOF_Z), (-half, g.ROOF_Z)], 2.90,
          mat=m["paint"], parent=hull, bevel=0.05, taper_y=lambda z: 1.0 if z < 1.2 else 0.97)
    prism("hull_lower", [(-3.10, 0.42), (2.60, 0.42), (half, 0.95), (-half, 0.95), (-half, 0.70)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan((g.ROOF_Z - 0.95) / (half - 0.70))

    def glacis(x):
        return 0.95 + (half - x) * math.tan(slope)

    # The ribbed trim-vane plate down the glacis.
    for k in range(9):
        x = 3.05 - k * 0.20
        box(f"glacis_rib_{k}", (0.05, 2.40, 0.04), (x, 0, glacis(x) + 0.03), m["paint"], hull, rot=(0, slope, 0),
            bevel=0.01, lods=MID)
    VP.grille("engine_grille", (1.55, -0.75, glacis(1.55)), (0.90, 1.00), m, hull, slats=7, rot=(0, slope, 0))
    # The driver's and commander's hatches on the left, placed from the turret ring.
    ring = v.frame["mounts"][0]["pivot_m"][0]
    VP.hatch("driver_hatch", (ring + 0.95, 0.75, glacis(ring + 0.95)), m, hull, radius=0.28, rot=(0, slope, 0))
    VP.hatch("commander_hatch", (ring + 0.30, 0.75, g.ROOF_Z), m, hull, radius=0.28)
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (1.95, side * 1.20, glacis(1.95) + 0.10), 0.07, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.20, side * 0.62, 0.86), m, hull, size=0.12)
        VP.tow_hook(f"rear_tow_{s}", (-3.30, side * 0.80, 0.82), m, hull, size=0.12, rot=(0, 0, math.pi))
        VP.stowage_box(f"side_bin_{s}", (-1.40, side * 1.45, 1.40), (2.60, 0.24, 0.22), m, hull,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        box(f"track_guard_{s}", (6.40, 0.42, 0.04), (-0.10, side * 1.36, 0.96), m["paint"], hull, bevel=0.012,
            lods=MID)
        cyl(f"rear_door_{s}", 0.55, 0.06, (-half - 0.01, side * 0.50, 0.90), "X", m["paint"], hull, seg=24,
            bevel=0.02, lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.08, side * 1.30, 1.30), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
    for k in range(4):
        VP.hatch(f"troop_hatch_{k}", (-1.00 - (k // 2) * 1.10, (k % 2 - 0.5) * 0.90, g.ROOF_Z), m, hull,
                 size=(0.80, 0.70))


def bmp2_turret(v, turret, gun, launcher, tube):
    m = v.mats
    half = [(0.85, 0.40), (0.60, 0.90), (-0.70, 0.95), (-0.95, 0.70)]

    def ring(pts, lean):
        left = [(x, y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    loft("turret_shell", [(-0.03, ring(half, 0.0)), (0.62, ring([(x * 0.85, y) for x, y in half], 0.20))],
         mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 0.95, 0.06, (0, 0, -0.04), "Z", m["dark"], turret, seg=32, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.30, side * 0.78, 0.20), m, turret, count=3, tube_radius=0.05,
                                 tube_length=0.24, elevation=0.4, spread=0.3, rot=(0, 0, side * 0.9))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-0.70, side * 0.50, 0.62), m, whip, height=2.0)
    VP.sight_housing("gunner_sight", (0.20, 0.35, 0.58), m, turret, size=(0.34, 0.26, 0.24))
    VP.sight_housing("commander_sight", (-0.15, -0.40, 0.58), m, turret, size=(0.32, 0.26, 0.26))
    VP.hatch("turret_hatch", (-0.40, 0.30, 0.62), m, turret, radius=0.25)
    # The AG-30 on the right side.
    box("ag30_body", (0.50, 0.14, 0.16), (0.40, -0.92, 0.20), m["dark"], turret, bevel=0.015)
    cyl("ag30_barrel", 0.035, 0.40, (0.85, -0.92, 0.20), "X", m["dark"], turret, seg=10, lods=MID)
    # The 2A42: its mantlet, barrel and slotted muzzle.
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.30, 0.28, 0.24), (0.80, 0, 0), m["paint"], gun, bevel=0.03)
    cyl("barrel", 0.035, reach - 1.05, ((reach + 1.05) / 2, 0, 0), "X", m["dark"], gun, seg=12)
    cyl("muzzle", 0.055, 0.16, (reach - 0.08, 0, 0), "X", m["steel"], gun, seg=14)
    for k in range(3):
        box(f"muzzle_slot_{k}", (0.03, 0.12, 0.02), (reach - 0.12 + k * 0.04, 0, 0.0), m["black"], gun, lods=FINE)
    # The Kornet pair on the left arm.
    lreach = v.frame["mounts"][1]["muzzle_m"][0]
    box("kornet_arm", (0.24, 0.16, 0.28), (-0.10, -0.12, -0.12), m["dark"], launcher, bevel=0.02)
    for k, y in enumerate((0.11, -0.11)):
        cyl(f"kornet_tube_{k}", 0.09, lreach + 0.40, ((lreach - 0.40) / 2, y, 0.0), "X", m["paint"], tube, seg=16,
            bevel=0.01)
        cyl(f"kornet_cap_{k}", 0.07, 0.02, (lreach + 0.005, y, 0.0), "X", m["black"], tube, seg=14, lods=MID)
    box("kornet_sight", (0.20, 0.14, 0.16), (0.10, 0.0, 0.18), m["dark"], tube, bevel=0.015, lods=MID)


# ---------------------------------------------------------------- BMP-3
def bmp3_hull(v):
    m, hull = v.mats, v.hull
    g = Bmp3
    half = v.length / 2
    prism("hull_upper", [(-half, 0.90), (3.20, 0.90), (half, 1.04), (2.30, g.ROOF_Z), (-half, g.ROOF_Z)], 3.04,
          mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.30, 0.42), (2.85, 0.42), (half, 1.04), (-half, 0.92), (-half, 0.72)], 2.24,
          mat=m["paint"], parent=hull, bevel=0.04)
    prism("splash_board", [(2.90, 1.20), (3.02, 1.20), (2.96, 1.40), (2.84, 1.40)], 2.80, mat=m["paint"], parent=hull,
          bevel=0.015)
    for k, y in enumerate((-1.0, 0.0, 1.0)):
        VP.periscope(f"driver_periscope_{k}", (2.30, y * 0.30, g.ROOF_Z), m, hull, size=(0.12, 0.16, 0.08))
    VP.hatch("driver_hatch", (1.95, 0.0, g.ROOF_Z), m, hull, radius=0.26)
    for side, s in ((1, "L"), (-1, "R")):
        VP.hatch(f"bow_gunner_hatch_{s}", (2.05, side * 0.95, g.ROOF_Z), m, hull, radius=0.24)
        cyl(f"bow_mg_port_{s}", 0.06, 0.08, (3.12, side * 0.90, 1.12), "X", m["black"], hull, seg=12, lods=MID)
        VP.light_with_guard(f"headlight_{s}", (2.80, side * 1.30, 1.30), 0.07, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.40, side * 0.60, 0.86), m, hull, size=0.12)
        VP.tow_hook(f"rear_tow_{s}", (-3.50, side * 0.80, 0.82), m, hull, size=0.12, rot=(0, 0, math.pi))
        # The skirt over the tracks with its long bolted rail.
        VP.bolted_panel(f"skirt_{s}", (-0.15, side * 1.48, 0.85), (6.40, 0.30, 0.08), m, hull, bolts=(10, 1),
                        bevel=0.03, rot=(-side * math.pi / 2, 0, 0), lods=VP.ALL)
        box(f"skirt_rail_{s}", (6.60, 0.06, 0.06), (-0.10, side * 1.57, 0.96), m["paint"], hull, bevel=0.02,
            lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.08, side * 1.30, 1.40), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
        stencil(f"side_number_{s}", "811", 0.28, (0.40, side * 1.524, 1.32),
                (math.pi / 2, 0, math.pi if side > 0 else 0),
                m["marking"], hull)
    # The engine deck at the rear, its long side grille, the rear doors.
    VP.grille("engine_grille", (-2.55, 0, g.ROOF_Z), (1.30, 2.00), m, hull, slats=10)
    VP.grille("side_grille", (-2.70, 1.522, 1.30), (1.40, 0.12), m, hull, slats=3, rot=(-math.pi / 2, 0, 0))
    for side in (-1, 1):
        VP.bolted_panel(f"rear_door_{side}", (-half - 0.01, side * 0.42, 0.62), (0.04, 0.70, 0.70), m, hull,
                        bolts=(1, 1), bevel=0.02, lods=VP.ALL)


def bmp3_turret(v, turret, gun, coax_gun):
    m = v.mats

    def ring(r, n=18, shift=0.0, stretch=1.15):
        return [(shift + r * stretch * math.cos(k * math.tau / n), r * math.sin(k * math.tau / n)) for k in range(n)]

    loft("turret_shell", [(-0.02, ring(1.05)), (0.30, ring(1.00)), (0.50, ring(0.80)), (0.56, ring(0.55))],
         mat=m["paint"], parent=turret, bevel=0.03)
    cyl("turret_ring_guard", 1.00, 0.06, (0, 0, -0.04), "Z", m["dark"], turret, seg=32, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 0.72, 0.30), m, turret, count=3, tube_radius=0.05,
                                 tube_length=0.24, elevation=0.4, spread=0.3, rot=(0, 0, side * 0.6))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-0.85, side * 0.55, 0.50), m, whip, height=2.0)
    VP.sight_housing("gunner_sight", (0.35, -0.40, 0.48), m, turret, size=(0.34, 0.28, 0.26))
    VP.cupola("commander_cupola", (-0.20, 0.45, 0.50), m, turret, radius=0.28, periscopes=4)
    cyl("commander_lamp", 0.07, 0.10, (0.15, 0.45, 0.66), "X", m["lamp"], turret, seg=12, lods=NEAR)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.40, 0.34, 0.30), (0.95, 0.0, 0.0), m["paint"], gun, bevel=0.04)
    cyl("barrel_root", 0.10, 0.50, (1.35, 0, 0), "X", m["paint"], gun, seg=20, bevel=0.012)
    cyl("barrel", 0.075, reach - 1.85, ((reach + 1.60) / 2, 0, 0), "X", m["paint"], gun, seg=18)
    cyl("fume_extractor", 0.10, 0.36, (2.40, 0, 0), "X", m["paint"], gun, seg=20, bevel=0.015)
    cyl("muzzle", 0.085, 0.16, (reach - 0.08, 0, 0), "X", m["steel"], gun, seg=18)
    cyl("muzzle_bore", 0.05, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=14, lods=MID)
    creach = v.frame["mounts"][1]["muzzle_m"][0]
    cyl("coax_barrel", 0.035, creach - 1.10, ((creach + 1.10) / 2, 0, 0), "X", m["dark"], coax_gun, seg=12)
    cyl("coax_muzzle", 0.05, 0.12, (creach - 0.06, 0, 0), "X", m["steel"], coax_gun, seg=12)
    box("coax_sleeve", (0.30, 0.10, 0.10), (1.05, 0, 0), m["paint"], coax_gun, bevel=0.015, lods=MID)


# ---------------------------------------------------------------- export
def build(variant, v):
    three = variant["id"].endswith("bmp_3")
    g = Bmp3 if three else Bmp2
    (bmp3_hull if three else bmp2_hull)(v)
    VP.tracked_running_gear(v.mats, v.hull, g.TRACK_Y, g.TRACK_W, g.ROAD_X, g.ROAD_Z, g.ROAD_R, 0.24, g.SPROCKET,
                            g.IDLER, g.RETURNS, dual=False, bolts=6, ribs=6, teeth=12, arm=(0.40, 0.40), pitch=0.15)
    mounts = rig(v.frame, v.root)
    if three:
        turret, gun, _, pivot = mounts["main_gun"]
        _, coax_gun, _, _ = mounts["autocannon"]
        bmp3_turret(v, turret, gun, coax_gun)
        # In the cupola (turret frame (-0.20, 0.45), its roof 0.56 up).
        v.head_out("commander", turret, pivot.x - 0.20, pivot.y + 0.45, pivot.z + 0.56)
    else:
        turret, gun, _, _ = mounts["autocannon"]
        launcher, tube, _, _ = mounts["launcher"]
        bmp2_turret(v, turret, gun, launcher, tube)


def wreck(variant, v):
    """The vehicle after its fire (an aluminium-and-steel hull burns hot):
    the right track off along its side, two road wheels gone, the rear doors
    blown off, the bins burnt away, plates warped and the roof sagging.
    `wreckage.burn` heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_2_", "wheel_R_4_", "rear_door_", "side_bin_", "troop_hatch_", "side_number_R",
           "skirt_R")
    three = variant["id"].endswith("bmp_3")
    g = Bmp3 if three else Bmp2
    thrown = solid("thrown_track", (3.4, g.TRACK_W, 0.05), (-0.3, -g.TRACK_Y - 0.25, 0.03), m["track"], v.hull,
                   rot=(0, 0, 0), lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=17.0))
    shell = parts("hull_upper", "hull_lower", "turret_shell", "glacis_rib_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.03, 0.9, seed=18.0), dent((-1.0, 0.0, g.ROOF_Z), 1.0, 0.12, (0, 0, -1)),
         dent((2.90, 0.40, 1.10), 0.4, 0.08, (-0.6, 0, -1)))
    rest_on_ground(0.004)


run("bmp", "russian_green", build, wreck, chip=1.0)
