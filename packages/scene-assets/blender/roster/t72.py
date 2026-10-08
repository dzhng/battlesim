"""T-72B3 (mod. 2016), from assets/references/t72/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t72.py -- [--variant=<id>] [--wreck]

What the photos settle (Alabino 2017, Moscow 2018): six large road wheels a
side with ribbed discs, three small return rollers mostly hidden, the
sprocket at the rear and the idler at the front; a steep glacis in rows of
explosive reactive armour under a V splash board; the hull sides in two rows
of bolted Relikt panels over the front two-thirds above a scalloped rubber
skirt, slat panels over the rear third and a slat cage round the engine deck.
The cast dome turret wears a fan of sloped Relikt blocks round each front
cheek, a row of rounded side modules wrapping its flanks, two rows of smaller
blocks on the roof front, four smoke tubes a side, the gunner's Sosna-U sight
box on the left, the commander's cupola with its heavy machine gun on the
right, a stowage box and snorkel on the rear and a slat frame round the
bustle. The commander rides head out.

The T-72 hull, running gear, skirts, gun and cupola are the T-90M's too
(`roster/t90.py` imports them): the T-90 is a T-72 derivative.

Built to the catalog frame (hull 6.86 x 3.59 x 2.23 m, turret pivot 1.338 m,
cannon muzzle 6.3 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

TRACK_Y = 1.40
TRACK_W = 0.58
ROAD_R = 0.375
ROAD_Z = 0.455
ROAD_X = [2.05 - 0.81 * k for k in range(6)]
SPROCKET = (-2.92, 0.52, 0.32)
IDLER = (2.95, 0.50, 0.30)
RETURNS = [(1.62, 0.86, 0.10), (0.02, 0.87, 0.10), (-1.56, 0.86, 0.10)]
DECK = 1.32
SKIRT_Y = 1.74  # the side armour's outer face
TRUNNION = 0.95


def glacis_z(x):
    if x >= 2.05:
        return DECK - (x - 2.05) / 1.38 * 0.40
    return DECK


# ---------------------------------------------------------------- hull (shared with the T-90M)
def soviet_hull(v, rear_cage=True):
    """The T-72 hull: the steep ERA-covered glacis under its V splash board,
    the driver's hatch at its head, fenders, the engine deck's grilles and a
    slat cage round the rear. Roles: paint, dark, steel, black, glass, lamp."""
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.92), (3.30, 0.92), (half, 0.94), (2.05, DECK), (-3.36, DECK), (-half, 1.24)],
          3.30, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.20, 0.44), (2.90, 0.44), (3.42, 0.94), (-half, 0.94), (-half, 0.78)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan(0.40 / 1.38)
    VP.armour_tiles("glacis_era", (2.78, 0, glacis_z(2.78) + 0.005), (1.10, 2.70), (2, 6), 0.09, m, hull,
                    rot=(0, slope, 0))
    prism("splash_board", [(2.16, DECK - 0.02), (2.26, DECK - 0.02), (2.20, DECK + 0.12), (2.10, DECK + 0.12)], 2.60,
          mat=m["paint"], parent=hull, bevel=0.015)
    VP.hatch("driver_hatch", (1.80, 0, DECK), m, hull, radius=0.30)
    VP.periscope("driver_periscope", (2.06, 0, DECK), m, hull, size=(0.12, 0.26, 0.09))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (2.0, side * 1.30, DECK + 0.10), 0.07, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.30, side * 0.62, 0.80), m, hull, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-3.30, side * 0.80, 0.82), m, hull, size=0.13, rot=(0, 0, math.pi))
        # The fenders over the tracks, their front lips raised over the idler.
        box(f"front_fender_{s}", (0.45, 0.60, 0.04), (3.18, side * TRACK_Y, 1.02), m["paint"], hull, bevel=0.012,
            rot=(0, 0.35, 0), lods=MID)
        VP.cable(f"tow_cable_{s}", [(-3.0, side * 1.52, DECK + 0.02), (-1.6, side * 1.54, DECK + 0.02),
                                    (0.0, side * 1.54, DECK + 0.02), (1.4, side * 1.52, DECK + 0.02)], m, hull,
                 radius=0.022)
        VP.stowage_box(f"fender_box_{s}", (0.35, side * 1.50, DECK), (0.80, 0.36, 0.30), m, hull,
                       rot=(0, 0, 0 if side > 0 else math.pi))
    VP.grille("engine_grille", (-2.30, 0, DECK), (1.40, 1.70), m, hull, slats=12)
    VP.bolted_panel("engine_door", (-1.20, 0.0, DECK), (0.65, 1.90, 0.03), m, hull, bolts=(2, 3), bevel=0.01,
                    lods=MID)
    VP.exhaust("exhaust_L", (-1.70, 1.66, 1.10), 0.10, 0.30, m, hull, rot=(0, 0, math.pi / 2))
    if rear_cage:
        VP.slat_armour("rear_cage", (-half - 0.05, 0, 0.66), (3.20, 0.78), m, hull, rot=(0, 0, math.pi / 2))


def soviet_gear(v):
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.24, SPROCKET, IDLER,
                            RETURNS, bolts=6, ribs=8, teeth=12, arm=(0.45, -0.35), pitch=0.15, dual=True)


def soviet_skirts(v, label="21", outer=SKIRT_Y, front=2.95, bars=(-2.28, 2.20)):
    """Two rows of bolted Relikt panels over the front two-thirds of each
    side (their outer face `outer` from the centre line, the first panel's
    centre at `front`), a scalloped rubber skirt under them, slat panels over
    the rear (`bars`: centre and length)."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        rows = ((1.06, 0.30), (0.80, 0.24))
        for r, (z, h) in enumerate(rows):
            for k in range(6):
                x = front - k * 0.70
                VP.bolted_panel(f"side_era_{s}_{r}_{k}", (x, side * (outer - 0.06), z), (0.66, 0.12, h), m, hull,
                                bolts=(2, 1), bevel=0.03, lods=VP.ALL)
        for k in range(7):
            x = front + 0.10 - k * 0.70
            box(f"rubber_skirt_{s}_{k}", (0.64, 0.03, 0.22), (x, side * (outer - 0.08), 0.70), m["rubber"], hull,
                taper=(0.8, 1.0), rot=(math.pi, 0, 0), lods=MID)
        VP.slat_armour(f"side_bars_{s}", (bars[0], side * (outer - 0.02), 0.70), (bars[1], 0.64), m, hull)
        box(f"side_rail_{s}", (v.length - 0.16, 0.10, 0.06), (0.0, side * (outer - 0.10), DECK - 0.02), m["paint"],
            hull,
            bevel=0.02, lods=MID)
        stencil(f"side_number_{s}", label, 0.22, (0.90, side * (outer + 0.003), 1.20),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- weapons (shared with the T-90M)
def gun_2a46(v, gun, sleeve_cover=True):
    """The 125 mm 2A46M: the mantlet cover, the thermal sleeve in sections,
    the fume extractor a third of the way out and the plain muzzle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    if sleeve_cover:
        box("mantlet_cover", (0.62, 0.62, 0.48), (0.20, 0, 0.0), m["canvas"], gun, bevel=0.12)
    cyl("barrel_root", 0.14, 0.40, (0.65, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.015)
    evac = 2.10
    for k, (a, b) in enumerate([(0.85, evac - 0.36), (evac + 0.36, reach - 0.18)]):
        cyl(f"thermal_sleeve_{k}", 0.092, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k, x in enumerate((1.25, evac + 0.80, evac + 1.80, reach - 0.60)):
        cyl(f"sleeve_band_{k}", 0.10, 0.05, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("fume_extractor", 0.145, 0.56, (evac, 0, 0), "X", m["paint"], gun, seg=28, bevel=0.02)
    cyl("muzzle_end", 0.090, 0.18, (reach - 0.09, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.064, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)


def cupola_mg(v, hmg, hmg_gun, remote=False):
    """The commander's machine gun: on its cupola ring and post, or (the
    T-90M) in a small remote mount with its own sight."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    if remote:
        cyl("rws_bearing", 0.20, 0.10, (0, 0, -0.06), "Z", m["dark"], hmg, seg=20, bevel=0.012)
        box("rws_body", (0.40, 0.34, lift), (-0.04, 0, lift / 2 - 0.02), m["paint"], hmg, bevel=0.03)
        VP.sight_housing("rws_sight", (0.04, -0.24, -0.10), m, hmg_gun, size=(0.26, 0.14, 0.20))
    else:
        cyl("mg_post", 0.04, lift + 0.10, (0.15, 0, (lift + 0.10) / 2 - 0.10), "Z", m["steel"], hmg, seg=10)
        box("mg_cradle", (0.24, 0.12, 0.10), (0.12, 0, lift - 0.07), m["dark"], hmg, lods=MID)
        box("mg_shield", (0.04, 0.34, 0.24), (0.42, 0, -0.04), m["paint"], hmg_gun, bevel=0.01, lods=MID)
    VP.kord(hmg_gun, muzzle[0], m)


# ---------------------------------------------------------------- turret
def dome_rings():
    """The cast dome: a rounded plan, longer at the front, its walls rolling
    in to a domed roof."""
    def ring(rx_front, rx_rear, ry, n=20, shift=0.05):
        out = []
        for k in range(n):
            a = k * math.tau / n
            rx = rx_front if math.cos(a) > 0 else rx_rear
            out.append((shift + rx * math.cos(a), ry * math.sin(a)))
        return out

    return [(-0.04, ring(1.55, 1.40, 1.32)), (0.30, ring(1.52, 1.40, 1.30)), (0.56, ring(1.30, 1.22, 1.12)),
            (0.76, ring(0.95, 0.95, 0.88)), (0.82, ring(0.60, 0.60, 0.56))]


def relikt_fan(v, turret, side, s, blocks=5, radius=1.28, z=0.50):
    """A fan of sloped Relikt blocks round one front cheek, each a chunky
    bolted box leaning out and down from the dome."""
    m = v.mats
    for k in range(blocks):
        a = side * (0.30 + k * 0.24)
        x, y = 0.05 + radius * math.cos(a), radius * math.sin(a)
        box(f"cheek_era_{s}_{k}", (0.60, 0.25, 0.20), (x, y, z), m["paint"], turret, bevel=0.035,
            rot=(0, 0.55, a))
        for j in range(2):
            cyl(f"cheek_era_bolt_{s}_{k}_{j}", 0.018, 0.02, (x + 0.18 * math.cos(a), y + 0.18 * math.sin(a),
                                                             z + 0.06 - j * 0.12), "Z", m["steel"], turret, seg=6,
                lods=FINE)


def side_modules(v, turret, side, s, count=5, start=1.20, radius=1.36):
    """Rounded Relikt modules wrapping the turret's flank behind the cheek."""
    m = v.mats
    for k in range(count):
        a = side * (start + k * 0.26)
        x, y = 0.05 + radius * math.cos(a), radius * math.sin(a)
        VP.bolted_panel(f"side_module_{s}_{k}", (x, y, 0.08), (0.34, 0.24, 0.42), m, turret, bolts=(1, 2),
                        bevel=0.06, rot=(0, 0, a), lods=VP.ALL)


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", dome_rings(), mat=m["paint"], parent=turret, bevel=0.03)
    cyl("turret_ring_guard", 1.12, 0.10, (0, 0, -0.07), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        relikt_fan(v, turret, side, s)
        side_modules(v, turret, side, s)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 1.02, 0.62), m, turret, count=4, tube_radius=0.05,
                                 tube_length=0.22, elevation=0.30, spread=0.5, rot=(0, 0, side * 0.80))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.05, side * 0.70, 0.72), m, whip, height=2.2)
    # Roof ERA: two rows of small blocks across the front.
    for row in range(2):
        for k in range(4):
            y = (k - 1.5) * 0.36
            if abs(y) < 0.2:
                continue
            box(f"roof_era_{row}_{k}", (0.30, 0.30, 0.10), (0.88 - row * 0.32, y, 0.76 + row * 0.05), m["paint"],
                turret, bevel=0.02, rot=(0, 0.18, 0))
    # The gunner's Sosna-U sight box on the left, the commander's cupola on
    # the right, a loader-less roof hatch on the left rear.
    VP.sight_housing("sosna_sight", (0.55, 0.58, 0.66), m, turret, size=(0.42, 0.34, 0.30))
    VP.cupola("commander_cupola", (-0.25, -0.58, 0.72), m, turret, radius=0.36, periscopes=4)
    VP.hatch("gunner_hatch", (-0.25, 0.58, 0.74), m, turret, radius=0.30)
    # The rear: stowage box, snorkel tube, a slat frame round the bustle.
    VP.stowage_box("rear_box", (-1.58, 0, 0.12), (0.40, 1.20, 0.38), m, turret, rot=(0, 0, math.pi / 2))
    cyl("snorkel", 0.11, 1.30, (-1.30, -0.78, 0.48), "Y", m["paint"], turret, seg=16, rot=(0.2, 0, 0.45), lods=MID)
    VP.slat_armour("bustle_bars", (-1.86, 0, 0.0), (1.90, 0.46), m, turret, rot=(0, 0, math.pi / 2))


# ---------------------------------------------------------------- wreck
def soviet_wreck(v, extra_remove=(), seed=0):
    """A Soviet-pattern tank after its cook-off (the ammunition carousel's
    blast): the left track run off with two road wheels gone, side panels
    blown off and lying by it, the rear slats bent, plates warped, the glacis
    dented. `wreckage.burn` then throws the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_L_band", "wheel_L_3_", "wheel_L_4_", "side_era_L_0_2", "side_era_L_0_3", "side_era_L_1_2",
           "rubber_skirt_L_3", "rubber_skirt_L_4", "side_number_L", "fender_box_L", *extra_remove)
    thrown = solid("thrown_track", (3.9, TRACK_W, 0.05), (-0.4, 2.15, 0.03), m["track"], v.hull, rot=(0, 0, -0.07),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=1.0 + seed))
    for k, (loc, rot) in enumerate((((1.3, 2.55, 0.07), (1.5, 0.2, 0.5)), ((0.5, 2.70, 0.07), (1.5, -0.3, -0.2)))):
        solid(f"fallen_era_{k}", (0.66, 0.12, 0.30), loc, m["paint"], v.hull, rot=rot, bevel=0.02)
    bend(parts("side_bars_R"), (-2.28, -1.72, 1.34), (1, 0, 0), (0, 0, -1), -0.5)
    shell = parts("hull_upper", "hull_lower", "side_era_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=11.0 + seed), dent((2.90, 0.50, 1.10), 0.5, 0.10, (-0.6, 0, -1)))
    for k, (loc, rot, size) in enumerate((((2.2, -2.1, 0.03), (0.04, 0.02, 0.4), 0.40),
                                          ((-2.7, -2.0, 0.03), (-0.03, 0.05, 1.8), 0.32),
                                          ((3.8, 0.7, 0.03), (0.0, 0.06, 2.6), 0.26))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15,
              seed=71 + k + seed)
    rest_on_ground(0.004)


def build(variant, v):
    soviet_hull(v)
    soviet_gear(v)
    soviet_skirts(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    gun_2a46(v, gun)
    cupola_mg(v, hmg, hmg_gun)
    v.head_out("commander", turret, -0.25, -0.58, 2.13)


def wreck(variant, v):
    soviet_wreck(v, ("gunner_hatch", "rear_box", "snorkel"))


if __name__ == "__main__":
    run("t72", "russian_green", build, wreck, chip=1.0)
