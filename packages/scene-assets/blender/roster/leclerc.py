"""Leclerc XLR, from assets/references/leclerc/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/leclerc.py -- [--variant=<id>] [--wreck]

What the photos settle: one XLR photo (Eurosatory 2022, front left) and
pre-XLR Leclercs for the shared hull, running gear and turret core. Six large
evenly spaced road wheels a side with eight-bolt hubs, the sprocket raised at
the rear and the idler raised at the front, thin skirts with a top band; a
shallow upper glacis, a short nose plate and a steep lower glacis, the lamp
clusters in boxes on the fenders' front corners and shackles on the lower
glacis. The turret is low and flat-faced, its armour modules either
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
IDLER = (2.88, 0.70, 0.29)  # its top level with the return rollers' tops
RETURNS = [(1.15, 0.90, 0.09), (-0.55, 0.91, 0.09), (-2.05, 0.90, 0.09)]
DECK = 1.50
SIDE_Y = 1.82  # the skirts' outer face, clear of the tracks (out to 1.77)
SKIRT_T = 0.04
SKIRT_TOP = DECK - 0.02
HULL_Y = 1.65  # the hull box's side
RISE_X = 2.52  # where the front skirt's lower edge starts to climb
BOX_BACK, BOX_FRONT, BOX_FOOT = 2.75, 3.40, 1.20  # the fenders over the tracks' front
FOOT = -0.02
ROOF = 0.64
TRUNNION = 1.30
MODULE_D = 0.065


# The upper glacis: from its head at GLACIS_X it falls 0.24 m over 1.24 m to
# the nose plate, about 11 degrees side-on.
GLACIS_X, GLACIS_DROP = 2.20, 0.24
NOSE_X = 3.44
NOSE_TOP, NOSE_FOOT = DECK - GLACIS_DROP, 1.14  # the nose plate's short upright face


def glacis_z(x):
    """The hull top's height at x: the deck, then the glacis falling to the
    nose plate."""
    return DECK - max(0.0, x - GLACIS_X) / (NOSE_X - GLACIS_X) * GLACIS_DROP


def band_top(x):
    """The skirt band's and the fenders' top at x: the deck's line, falling
    with the glacis toward the nose."""
    return min(DECK, glacis_z(x))


def fender_foot(x):
    """The fenders' underside: level over the track's top run, then stepping
    down ahead of the idler round the track's front."""
    return BOX_FOOT - max(0.0, min(1.0, (x - 3.05) / 0.25)) * (BOX_FOOT - 1.00)


def lower_glacis_x(z):
    """The steep lower glacis's x at height `z`: from the belly at 0.48 m,
    x 2.85, up and forward to the nose plate's foot at the nose."""
    return 2.85 + (z - 0.48) * (NOSE_X - 2.85) / (NOSE_FOOT - 0.48)


def skirt_foot(x):
    """The skirt's lower edge: level to RISE_X, then climbing with the track
    toward the idler."""
    return 0.82 + max(0.0, x - RISE_X) * (1.14 - 0.82) / (3.38 - RISE_X)


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
    # The deck falls over a shallow upper glacis, then breaks over a short
    # nose plate onto the steep lower glacis.
    g = lower_glacis_x
    slope = math.atan(GLACIS_DROP / (NOSE_X - GLACIS_X))
    prism("hull_upper", [(-half, 1.00), (g(1.00), 1.00), (half, NOSE_FOOT), (half, NOSE_TOP), (GLACIS_X, DECK),
                         (-3.40, DECK), (-half, 1.44)], 3.30, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.20, 0.48), (g(0.48), 0.48), (g(1.02), 1.02), (-half, 1.02), (-half, 0.84)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    VP.hatch("driver_hatch", (2.30, 0.55, glacis_z(2.30)), m, hull, size=(0.56, 0.50), rot=(0, slope, 0))
    for k, y in enumerate((0.35, 0.55, 0.75)):
        VP.periscope(f"driver_periscope_{k}", (2.66, y, glacis_z(2.66)), m, hull, size=(0.14, 0.16, 0.08),
                     rot=(0, slope, 0))
    VP.weld_line("glacis_weld", [(GLACIS_X + 0.02, -1.6, DECK + 0.004), (GLACIS_X + 0.02, 1.6, DECK + 0.004)], m,
                 hull)
    for side, s in ((1, "L"), (-1, "R")):
        # The lamp clusters in their boxes on the fenders' front corners,
        # lying on the fenders' falling top.
        lamp = (3.12, side * 1.42, band_top(3.12) - 0.01)
        VP.bolted_panel(f"lamp_box_{s}", lamp, (0.30, 0.42, 0.13), m, hull, bolts=(1, 2), bevel=0.03,
                        rot=(0, slope, 0), lods=VP.ALL)
        for k, dy in enumerate((-0.10, 0.10)):
            cyl(f"lamp_{s}_{k}", 0.055, 0.03, (lamp[0] + 0.16, lamp[1] + dy, lamp[2] + 0.06), "X", m["lamp"], hull,
                seg=14, lods=MID)
        VP.shackle(f"front_shackle_{s}", (g(0.92) + 0.01, side * 0.62, 0.92), m, hull, size=0.16,
                   rot=(0, 0, math.pi / 2))
        VP.tow_hook(f"rear_tow_{s}", (-3.32, side * 0.78, 0.86), m, hull, size=0.13, rot=(0, 0, math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-3.36, side * 1.45, 1.36), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-3.20, side * 1.54, DECK + 0.025), (-1.6, side * 1.56, DECK + 0.025),
                                    (0.4, side * 1.56, DECK + 0.02), (2.1, side * 1.54, DECK + 0.02)], m, hull,
                 radius=0.022)
    # The number plate high on the lower glacis, which leans forward and
    # down from the nose plate.
    lean = math.atan2(NOSE_X - 2.85, NOSE_FOOT - 0.48)
    stencil("nose_number", "6044-0116", 0.09, (g(1.06) + 0.004 * math.cos(lean), 0.0, 1.06 - 0.004 * math.sin(lean)),
            (math.pi / 2 + lean, 0, math.pi / 2), m["marking"], hull)
    for k, y in enumerate((0.72, -0.72)):
        VP.grille(f"engine_grille_{k}", (-2.70, y, DECK), (1.10, 1.15), m, hull, slats=9)
        VP.bolted_panel(f"engine_door_{k}", (-1.75, y, DECK), (0.70, 1.10, 0.03), m, hull, bolts=(2, 2), bevel=0.01,
                        lods=MID)
    VP.grille("rear_louvre", (-half, 0, 1.18), (0.34, 2.40), m, hull, slats=7, rot=(0, -math.pi / 2, 0))
    VP.slat_armour("rear_bars", (-half - 0.06, 0, 0.66), (3.10, 0.70), m, hull, rot=(0, 0, math.pi / 2))


def hull_sides(v):
    """The thin skirt with its top band; XLR armour modules stacked over the
    front, bar armour standing off the rear. Skirt, band, fenders and modules
    fall with the glacis toward the nose."""
    m, hull = v.mats, v.hull

    def topline(top, a, b, drop=0.0):
        """The top line (x, z) from b back to a, with the glacis' knee."""
        return [(x, top(x) - drop) for x in [b] + ([GLACIS_X] if a < GLACIS_X < b else []) + [a]]
    for side, s in ((1, "L"), (-1, "R")):
        # The thin skirt, clear of the track, its top under the fender plate
        # that carries the deck out to it; its lower edge climbs at the front
        # with the track toward the idler and runs on, as a tongue, under the
        # fender over the track's front.
        y = side * (SIDE_Y - SKIRT_T / 2)
        prism(f"skirt_{s}", [(-3.30, 0.82), (RISE_X, 0.82), (BOX_BACK, skirt_foot(BOX_BACK))]
              + topline(band_top, -3.30, BOX_BACK, 0.02), SKIRT_T, loc=(0, y, 0), mat=m["paint"], parent=hull,
              bevel=0.012)
        prism(f"skirt_{s}_tongue", [(BOX_BACK - 0.02, skirt_foot(BOX_BACK - 0.02)), (3.05, skirt_foot(3.05)),
                                    (3.05, BOX_FOOT + 0.01), (BOX_BACK - 0.02, BOX_FOOT + 0.01)], SKIRT_T,
              loc=(0, y, 0), mat=m["paint"], parent=hull, bevel=0.01)
        # The top band: the fender plate from the hull side out over the
        # skirt's top edge.
        prism(f"skirt_band_{s}",
              topline(band_top, -3.30, BOX_BACK, 0.07)[::-1] + topline(band_top, -3.30, BOX_BACK),
              SIDE_Y + 0.02 - (HULL_Y - 0.05), loc=(0, side * (SIDE_Y + 0.02 + HULL_Y - 0.05) / 2, 0),
              mat=m["paint"], parent=hull, bevel=0.015, lods=MID)
        # The fenders over the tracks' front, their top falling with the
        # glacis, their underside stepping down round the track's front, the
        # lamp boxes on their corners.
        lo, hi = TRACK_Y - TRACK_W / 2 - 0.01, SIDE_Y + 0.02
        prism(f"front_fender_{s}", [(BOX_BACK - 0.05, BOX_FOOT), (3.05, BOX_FOOT), (3.30, fender_foot(3.30)),
                                    (BOX_FRONT, fender_foot(BOX_FRONT)), (BOX_FRONT, band_top(BOX_FRONT)),
                                    (BOX_BACK - 0.05, band_top(BOX_BACK - 0.05))], hi - lo,
              loc=(0, side * (lo + hi) / 2, 0), mat=m["paint"], parent=hull, bevel=0.025, lods=VP.ALL)
        # The XLR's stacked armour modules over the front of the skirt, from
        # its foot to the band, their tops falling with it; the front
        # column's lower module chamfered along the skirt's climbing edge.
        foot, top, gap = 0.85, SKIRT_TOP - 0.01, 0.025
        col = ((RISE_X + 0.02) - (-1.15)) / 5
        mid = (foot + top) / 2
        cells = []
        for i in range(5):
            a, b = -1.15 + col * i + gap / 2, -1.15 + col * (i + 1) - gap / 2
            cells += [(f"side_modules_{s}_{i}_0", a, b, [(a, foot + gap / 2), (b, foot + gap / 2)], mid - gap / 2),
                      (f"side_modules_{s}_{i}_1", a, b, [(a, mid + gap / 2), (b, mid + gap / 2)], top - gap / 2)]
        a, b = RISE_X + 0.02 + gap / 2, 3.04
        cells += [(f"side_modules_{s}_5_0", a, b, [(a, skirt_foot(a) + 0.02), (b, skirt_foot(b) + 0.02)],
                   mid - gap / 2),
                  (f"side_modules_{s}_5_1", a, b, [(a, mid + gap / 2), (b, mid + gap / 2)], top - gap / 2)]
        for name, a, b, feet, hi_z in cells:
            def cap(x, hi_z=hi_z):
                return min(hi_z, band_top(x) - 0.03)
            prism(name, feet + topline(cap, a, b), MODULE_D, loc=(0, side * (SIDE_Y + MODULE_D / 2), 0),
                  mat=m["paint"], parent=hull, bevel=0.02, lods=VP.ALL)
            lo_z = max(z for _, z in feet)
            for k in range(4):
                bx = a + 0.06 if k % 2 == 0 else b - 0.06
                bz = lo_z + 0.05 if k < 2 else cap(bx) - 0.05
                cyl(f"{name}_bolt_{k}", 0.018, 0.016, (bx, side * (SIDE_Y + MODULE_D + 0.006), bz), "Y",
                    m["steel"], hull, seg=6, lods=FINE)
        VP.slat_armour(f"side_bars_{s}", (-2.25, side * (SIDE_Y + 0.03), 0.82), (2.10, 0.66), m, hull)
        stencil(f"side_number_{s}", "52", 0.22, (0.40, side * (SIDE_Y + MODULE_D + 0.008), 1.12),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- turret
def turret_rings():
    """A low flat-faced turret: wide front modules, sides angling in, the
    long narrower autoloader bustle."""
    half = [(1.72, 0.44), (1.72, 1.10), (1.30, 1.36), (-0.70, 1.36), (-1.10, 1.08), (-2.30, 1.02), (-2.40, 0.90)]

    def ring(pts, lean):
        left = [(x, y - lean) for x, y in pts]
        return VP.mirror_plan(left)

    return [(FOOT, ring(half, 0.0)), (ROOF, ring([(x - (0.10 if x > 1.0 else 0.0), y) for x, y in half], 0.06))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_rings(), mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.08, 0.10, (0, 0, FOOT - 0.04), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # The front armour modules either side of the mantlet.
        VP.bolted_panel(f"front_module_{s}", (1.82, side * 0.80, 0.05), (0.22, 0.66, 0.52), m, turret,
                        bolts=(1, 2), bevel=0.04, lods=VP.ALL)
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
    VP.bolted_panel("roof_plate", (0.55, -0.20, ROOF), (0.70, 0.55, 0.03), m, turret, bolts=(2, 2), bevel=0.01,
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
    remove("track_L_band", "wheel_L_2_", "wheel_L_5_", "side_modules_L_4_", "side_modules_L_5", "bustle_hatch",
           "side_number_L")
    thrown = solid("thrown_track", (3.8, TRACK_W, 0.05), (-0.6, 2.25, 0.03), m["track"], v.hull, rot=(0, 0, -0.07),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=2.0))
    for k, (loc, rot) in enumerate((((2.4, 2.45, 0.08), (1.4, 0.2, 0.6)), ((1.6, 2.7, 0.08), (1.5, -0.1, -0.4)))):
        solid(f"fallen_module_{k}", (0.70, 0.29, 0.12), loc, m["paint"], v.hull, rot=rot, bevel=0.02)
    bend(parts("side_bars_R"), (-2.25, -(SIDE_Y + 0.03), 1.48), (1, 0, 0), (0, 0, -1), -0.5)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=8.0), dent((3.10, -0.40, glacis_z(3.10)), 0.5, 0.10, (-0.3, 0, -1)))
    plate("bustle_lid", [(-0.36, -0.3), (0.36, -0.32), (0.36, 0.3), (-0.34, 0.32)], 0.05, (-2.9, -0.7, DECK + 0.05),
          (0.06, -0.1, 0.4), m["paint"], v.hull, seed=19)
    for k, (loc, rot, size) in enumerate((((2.2, -2.2, 0.03), (0.04, 0.02, 0.3), 0.40),
                                          ((-2.8, 2.0, 0.03), (-0.03, 0.05, 1.8), 0.32),
                                          ((3.9, 0.8, 0.03), (0.0, 0.06, 2.6), 0.28))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=51 + k)
    rest_on_ground(0.004)


run("leclerc", "french_three_tone", build, wreck, chip=1.0)
