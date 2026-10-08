"""Challenger 2 TES, from assets/references/challenger/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/challenger.py -- [--variant=<id>] [--wreck]

What the photos settle (TES "Megatron", Tankfest 2023, and the Op Telic TES):
six large road wheels a side on hydrogas arms, the drive sprocket raised at the
rear and the idler raised at the front; the side armour packs over the front
two-thirds of the hull and bar armour over the rear third and across the rear
plate; heavy armoured boxes on the nose corners over the tracks with the lamps
in them; a long sloped glacis with the driver's hatch at its head on the centre
line. The turret's front faces slope back steeply above the mantlet, its flat
cheeks angle in, five smoke tubes a side sit on the front corners, deep
stowage bins run along the bustle sides with bar armour round its rear; the
gunner's sight barbette on the right front roof, the commander's sight and
cupola behind it, the loader's hatch on the left, the remote weapon station
on the commander's side and the electronic countermeasures table on posts at
the rear of the roof. Commander and loader stand in their hatches.

Built to the catalog frame (hull 8.3 x 4.2 x 2.49 m, turret pivot 1.494 m,
cannon muzzle 5.561 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

TRACK_Y = 1.45
TRACK_W = 0.64
ROAD_R = 0.37
ROAD_Z = 0.45
ROAD_X = [2.62 - 0.96 * k for k in range(6)]
SPROCKET = (-3.20, 0.66, 0.33)
IDLER = (3.55, 0.64, 0.31)
RETURNS = [(1.70, 0.94, 0.10), (-0.20, 0.95, 0.10), (-2.10, 0.94, 0.10)]
DECK = 1.45
PACK_Y = 2.08  # the side armour's outer face
FOOT = -0.04
ROOF = 0.92
TRUNNION = 1.25


def glacis_z(x):
    if x >= 2.45:
        return DECK - (x - 2.45) / 1.70 * 0.42
    return DECK


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.24, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.55, -0.35), pitch=0.17)
    side_armour(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    main_gun(v, gun)
    weapon_station(v, hmg, hmg_gun)
    v.head_out("commander", turret, -0.95, -0.72, 1.494 + ROOF - 0.04)
    v.head_out("loader", turret, -0.45, 0.68, 1.494 + ROOF - 0.04, facing=0.4)


# ---------------------------------------------------------------- hull
def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 1.00), (3.95, 1.00), (half, 1.03), (2.45, DECK), (-4.08, DECK), (-half, 1.40)],
          3.50, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.85, 0.50), (3.55, 0.50), (4.10, 1.02), (-half, 1.02), (-half, 0.82)], 2.30,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan(0.42 / 1.70)
    VP.hatch("driver_hatch", (2.62, 0, glacis_z(2.62)), m, hull, size=(0.60, 0.70), rot=(0, slope, 0))
    VP.periscope("driver_periscope", (3.02, 0, glacis_z(3.02)), m, hull, size=(0.14, 0.30, 0.08), rot=(0, slope, 0))
    VP.armour_tiles("glacis_tiles", (3.55, 0, glacis_z(3.55) + 0.01), (0.70, 2.00), (1, 4), 0.06, m, hull,
                    rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        # The armoured boxes on the nose corners over the tracks, lamps in them.
        VP.bolted_panel(f"nose_box_{s}", (3.82, side * 1.52, 0.86), (0.62, 0.80, 0.52), m, hull, bolts=(2, 2),
                        bevel=0.05, lods=VP.ALL)
        box(f"nose_lamp_{s}", (0.03, 0.22, 0.12), (4.14, side * 1.70, 1.20), m["lamp"], hull, lods=MID)
        box(f"nose_lamp_hood_{s}", (0.10, 0.28, 0.03), (4.11, side * 1.70, 1.28), m["paint"], hull, lods=NEAR)
        VP.tow_hook(f"front_tow_{s}", (4.02, side * 0.62, 0.82), m, hull, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-4.02, side * 0.80, 0.86), m, hull, size=0.14, rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-3.9, side * 1.62, DECK + 0.025), (-2.2, side * 1.64, DECK + 0.025),
                                    (-0.6, side * 1.64, DECK + 0.02), (1.2, side * 1.62, DECK + 0.02)], m, hull,
                 radius=0.024)
        VP.jerrycan(f"rear_jerrycan_{s}", (-3.70, side * 1.30, DECK), dict(m, paint=m["dark"]), hull,
                    rot=(0, 0, math.pi / 2))
    # Engine deck: grilles and access doors; the rear plate under bar armour.
    for k, y in enumerate((0.75, -0.75)):
        VP.grille(f"engine_grille_{k}", (-3.15, y, DECK), (1.30, 1.20), m, hull, slats=10)
        VP.bolted_panel(f"engine_door_{k}", (-1.95, y, DECK), (0.90, 1.15, 0.03), m, hull, bolts=(3, 2), bevel=0.01,
                        lods=MID)
    VP.grille("rear_louvre", (-half, 0, 1.20), (0.36, 2.40), m, hull, slats=7, rot=(0, -math.pi / 2, 0))
    VP.slat_armour("rear_bars", (-half - 0.06, 0, 0.72), (3.20, 0.78), m, hull, rot=(0, 0, math.pi / 2))
    stencil("rear_number", "BUSTER", 0.10, (-half - 0.003, 0.9, 1.05), (math.pi / 2, 0, -math.pi / 2), m["marking"],
            hull)


def side_armour(v):
    """The side armour packs over the front two-thirds, as bolted modules,
    and the bar armour cage over the rear third, standing off the hull."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(5):
            x = 3.20 - k * 0.86
            VP.bolted_panel(f"side_pack_{s}_{k}", (x, side * (PACK_Y - 0.13), 0.66), (0.82, 0.26, 0.86), m, hull,
                            bolts=(2, 1), bevel=0.05, rot=(0, 0, 0), lods=VP.ALL)
            box(f"side_pack_handle_{s}_{k}", (0.16, 0.04, 0.05), (x, side * (PACK_Y + 0.01), 1.30), m["dark"], hull,
                lods=NEAR)
        # The front pack's sloped nose.
        prism(f"side_pack_nose_{s}", [(3.62, 0.70), (3.92, 0.95), (3.92, 1.52), (3.62, 1.52)], 0.26,
              loc=(0, side * (PACK_Y - 0.13), 0), mat=m["paint"], parent=hull, bevel=0.04)
        VP.slat_armour(f"side_bars_{s}", (-2.42, side * (PACK_Y - 0.05), 0.68), (2.80, 0.84), m, hull)
        for k, x in enumerate((-1.30, -2.40, -3.50)):
            box(f"bar_bracket_{s}_{k}", (0.06, 0.40, 0.06), (x, side * 1.90, 1.40), m["dark"], hull, lods=MID)
        box(f"sponson_lip_{s}", (8.0, 0.30, 0.05), (0.0, side * 1.85, DECK - 0.02), m["paint"], hull, bevel=0.015,
            lods=MID)
        stencil(f"pack_number_{s}", "11B", 0.24, (2.34, side * (PACK_Y + 0.003), 1.16),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- turret
def turret_rings():
    """The turret's foot and roof: its front faces slope back steeply over
    the mantlet, the cheeks angle in, the bustle squares off."""
    foot = [(1.78, 0.46), (1.34, 1.42), (-1.55, 1.48), (-2.18, 1.30)]
    roof = [(1.10, 0.46), (0.86, 1.22), (-1.55, 1.36), (-2.12, 1.22)]

    def ring(half):
        left = half
        right = [(x, -y) for x, y in reversed(half)]
        return left + right

    return [(FOOT, ring(foot)), (FOOT + 0.30, ring([(x - 0.02, y) for x, y in foot])), (ROOF, ring(roof))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_rings(), mat=m["paint"], parent=turret, bevel=0.06)
    cyl("turret_ring_guard", 1.15, 0.10, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # Cheek armour plates, bolted.
        VP.bolted_panel(f"cheek_plate_{s}", (0.95, side * 1.30, 0.70), (0.80, 0.50, 0.05), m, turret, bolts=(3, 2),
                        bevel=0.015, rot=(0, -0.55, side * -0.30), lods=MID)
        VP.smoke_discharger_bank(f"smoke_{s}", (1.05, side * 1.10, 0.66), m, turret, count=5, tube_radius=0.05,
                                 tube_length=0.26, elevation=0.45, spread=0.45, rot=(0, 0, side * 0.35))
        # Deep stowage bins along the bustle sides, nets over them.
        VP.stowage_box(f"side_bin_{s}", (-1.05, side * 1.62, 0.02), (1.60, 0.40, 0.66), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        box(f"side_net_{s}", (1.40, 0.10, 0.20), (-1.05, side * 1.84, 0.60), m["canvas"], turret, bevel=0.05,
            lods=MID)
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.95, side * 0.95, ROOF), m, whip, height=2.2)
    # The bar armour round the bustle's rear.
    VP.slat_armour("bustle_bars", (-2.40, 0, 0.05), (2.60, 0.62), m, turret, rot=(0, 0, math.pi / 2))
    for side in (-1, 1):
        box(f"bustle_bar_arm_{side}", (0.30, 0.05, 0.05), (-2.25, side * 1.10, 0.62), m["dark"], turret, lods=MID)
    # Roof: the gunner's sight barbette at the front right, the commander's
    # sight behind it, the cupolas, the countermeasures table at the rear.
    VP.sight_housing("gunner_sight", (0.85, -0.78, ROOF - 0.08), m, turret, size=(0.56, 0.44, 0.20))
    sight = empty("dressing_commander_sight", parent=turret)
    cyl("commander_sight_base", 0.17, 0.08, (0.05, -1.02, ROOF + 0.04), "Z", m["paint"], sight, seg=20,
        bevel=0.02, lods=MID)
    VP.sight_housing("commander_sight", (0.05, -1.02, ROOF + 0.06), m, sight, size=(0.42, 0.40, 0.26))
    VP.cupola("commander_cupola", (-0.95, -0.72, ROOF), m, turret, radius=0.38)
    VP.cupola("loader_cupola", (-0.45, 0.68, ROOF), m, turret, radius=0.34, periscopes=3, rot=(0, 0, 0.4))
    table = empty("dressing_ecm_table", parent=turret)
    box("ecm_plate", (0.80, 0.90, 0.04), (-1.70, 0.10, ROOF + 0.18), m["paint"], table, bevel=0.012)
    for k in range(4):
        box(f"ecm_post_{k}", (0.04, 0.04, 0.18), (-1.70 + (k % 2 - 0.5) * 0.70, 0.10 + (k // 2 - 0.5) * 0.80,
                                                    ROOF + 0.09), m["dark"], table, lods=MID)
    for k, (x, y) in enumerate(((-1.85, -0.15), (-1.55, -0.20))):
        cyl(f"ecm_rod_{k}", 0.03, 0.14, (x, y, ROOF + 0.27), "Z", m["dark"], table, seg=8, lods=NEAR)
    mast = empty("dressing_ecm_mast", parent=turret)
    cyl("ecm_mast", 0.03, 0.60, (-1.55, 0.35, ROOF + 0.50), "Z", m["dark"], mast, seg=8, lods=NEAR)


# ---------------------------------------------------------------- weapons
def main_gun(v, gun):
    """The 120 mm L30A1 rifled gun: a deep mantlet, the thermal sleeve in
    sections, the fume extractor a third of the way out, the muzzle reference
    box over the muzzle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.55, 0.80, 0.56), (0.20, 0, -0.02), m["paint"], gun, bevel=0.06)
    box("gun_shield_collar", (0.18, 0.46, 0.42), (0.55, 0, -0.02), m["paint"], gun, bevel=0.04, lods=MID)
    cyl("coax_port", 0.035, 0.05, (0.50, 0.30, 0.10), "X", m["black"], gun, seg=10, lods=FINE)
    evac = 1.70
    for k, (a, b) in enumerate([(0.64, evac - 0.36), (evac + 0.36, reach - 0.24)]):
        cyl(f"thermal_sleeve_{k}", 0.10, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k, x in enumerate((1.00, evac + 0.90, reach - 0.80)):
        cyl(f"sleeve_band_{k}", 0.11, 0.06, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("fume_extractor", 0.165, 0.60, (evac, 0, 0), "X", m["paint"], gun, seg=28, bevel=0.02)
    cyl("muzzle_end", 0.105, 0.24, (reach - 0.12, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.064, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)
    box("muzzle_reference", (0.14, 0.10, 0.09), (reach - 0.30, 0, 0.16), m["dark"], gun, lods=NEAR)


def weapon_station(v, hmg, hmg_gun):
    """The remote weapon station on the commander's side: a bearing, the
    cradle, its sight block and ammunition box, and a heavy machine gun."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    cyl("rws_bearing", 0.25, 0.10, (0, 0, -0.08), "Z", m["dark"], hmg, seg=24, bevel=0.012)
    box("rws_base", (0.48, 0.40, 0.12), (-0.02, 0, 0.02), m["paint"], hmg, bevel=0.025)
    for side in (-1, 1):
        box(f"rws_cradle_{side}", (0.30, 0.06, lift), (0.02, side * 0.17, lift / 2 + 0.06), m["paint"], hmg,
            bevel=0.015)
    VP.sight_housing("rws_sight", (0.10, -0.30, -0.12), m, hmg_gun, size=(0.32, 0.18, 0.26))
    box("rws_ammo", (0.30, 0.16, 0.26), (-0.05, 0.28, -0.08), m["paint"], hmg_gun, bevel=0.02, lods=MID)
    VP.browning_m2(hmg_gun, muzzle[0], m)


# ---------------------------------------------------------------- wreck
def wreck(variant, v):
    """The tank after its cook-off: the right track run off beside it with
    two road wheels gone, two right armour packs blown off and lying by the
    hull, the rear bar cage bent out, plates warped, the glacis dented, the
    loader's lid on the deck. `wreckage.burn` then heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_3_", "wheel_R_4_", "side_pack_R_2", "side_pack_R_3", "side_pack_handle_R_2",
           "side_pack_handle_R_3", "loader_cupola_lid", "side_net_", "rear_jerrycan_", "pack_number_R")
    thrown = solid("thrown_track", (4.6, TRACK_W, 0.05), (0.2, -2.40, 0.03), m["track"], v.hull, rot=(0, 0, 0.05),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=4.0))
    for k, (loc, rot) in enumerate((((3.1, -2.35, 0.14), (1.45, 0.1, 0.2)), ((-2.9, -2.35, 0.14), (1.5, -0.2, -0.15)))):
        solid(f"fallen_pack_{k}", (0.82, 0.26, 0.86), loc, m["paint"], v.hull, rot=rot, bevel=0.04)
    bend(parts("side_bars_L"), (-2.42, 2.03, 1.52), (1, 0, 0), (0, 0, -1), 0.5)
    shell = parts("hull_upper", "hull_lower", "side_pack_", "turret_shell", "nose_box_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=6.0), dent((3.70, 0.50, 1.10), 0.55, 0.10, (-0.6, 0, -1)))
    plate("loader_lid", [(-0.3, -0.28), (0.32, -0.3), (0.32, 0.28), (-0.3, 0.3)], 0.05, (-2.6, 0.9, DECK + 0.05),
          (0.05, -0.1, 0.5), m["paint"], v.hull, seed=17)
    for k, (loc, rot, size) in enumerate((((3.0, 2.5, 0.03), (0.04, 0.02, 0.5), 0.42),
                                          ((-3.3, 2.3, 0.03), (-0.03, 0.05, 1.7), 0.34),
                                          ((4.5, -1.0, 0.03), (0.0, 0.06, 2.2), 0.28))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=41 + k)
    rest_on_ground(0.004)


run("challenger", "british_green", build, wreck, chip=1.0)
