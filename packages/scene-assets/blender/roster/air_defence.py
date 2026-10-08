"""Air defence vehicles (disabled cards): Gepard 1A2, Buk-M3, Pantsir-SM,
Boxer Skyranger 30 and NASAMS 3, from assets/references/<family>/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/air_defence.py -- [--variant=<card id>] [--wreck]

What the photos settle:
- Gepard 1A2 (German three-tone): the Leopard 1 hull, seven road wheels with
  the sprocket at the rear; a box turret with a 35 mm Oerlikon gun outside
  each flank in its armoured cradle, the round tracking radar on the
  turret's front, the search radar dish folded flat on the rear roof.
- Buk-M3 TELAR (9A317M, Russian green): the GM-569 tracked chassis with six
  road wheels and the driver's cab at the front; the turntable behind with
  the fire-control radar's rounded housing at its front and six missile
  canisters in two rows, stowed flat to the rear.
- Pantsir-SM (Russian green): the KamAZ-6560 8x8's unarmoured cab-over with
  its tall windscreen; the combat module on the rear: the search radar's
  array on top, the tracking radar's flat array on its front, a twin 30 mm
  gun each side and six missile tubes a side, two columns of three, outside
  them.
- Skyranger 30 on Boxer (German three-tone): the Boxer's own hull
  (`boxer.drive`), the turret with its 30 mm KCE gun, the four radar
  panels round its top and a missile launcher box on its side.
- NASAMS 3 (Norway's, NATO green): the canister launcher on its two-axle
  trailer, six AMRAAM canisters in two rows on the turntable, stowed flat,
  levelling legs at the corners.

Each card's frame is the published
length, width and travelling height (the Buk's measured off its side photo
against its road wheels), recorded as `references`; turrets,
guns and launchers are drawn stowed and do not articulate yet.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import boxer  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from truck_chassis import chassis  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARDS = {
    "europe_gepard_1a2": ("gepard", "german_three_tone", (6.85, 3.37, 3.29)),
    "eastern_buk_buk_m3_medium_range_air_defense": ("buk", "russian_green", (9.30, 3.25, 3.55)),
    "eastern_pantsir_pantsir_sm": ("pantsir", "russian_green", (10.40, 2.55, 4.20)),
    "europe_boxer_skyranger_30_gun_and_missile_air_defense": ("boxer_skyranger_30", "german_three_tone",
                                                             (7.93, 2.99, 3.50)),
    "europe_nasams_nasams_3": ("nasams", "german_three_tone", (6.40, 2.45, 2.90)),
}


def tracked_hull(v, deck, wheels, road_r, rear_engine):
    """A tracked hull with its running gear: an upper hull over the tracks
    with a sloped glacis and its bolted plate, the lower tub, skirts, lights,
    tow points and cables, bins, the engine's grille and exhausts. Returns the
    glacis' top x."""
    m, h = v.mats, v.hull
    half = v.length / 2
    nose = deck - 0.42
    top = half - 1.20
    prism("hull_upper", [(-half, 0.88), (half - 0.35, 0.88), (half, nose), (top, deck), (-half + 0.06, deck),
                         (-half, deck - 0.10)], v.width - 0.45, mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.35, 0.44), (half - 0.75, 0.44), (half - 0.10, 0.90), (-half, 0.90),
                         (-half, 0.70)], v.width - 1.35, mat=m["paint"], parent=h, bevel=0.04)
    span = v.length - 1.9
    road_x = [half - 1.05 - span * k / (wheels - 1) for k in range(wheels)]
    front = (half - 0.40, road_r + 0.28, road_r * 0.85)
    rear = (-half + 0.40, road_r + 0.26, road_r * 0.85)
    sprocket, idler = (rear, front) if rear_engine else (front, rear)
    returns = [(road_x[1] - 0.2, 2 * road_r + 0.18, 0.09), (road_x[-2] + 0.2, 2 * road_r + 0.18, 0.09)]
    VP.tracked_running_gear(m, h, v.width / 2 - 0.40, 0.50, road_x, road_r + 0.08, road_r, 0.20, sprocket, idler,
                            returns, bolts=6, teeth=12, arm=(0.40, -0.30), pitch=0.15, dual=True)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"skirt_{s}", (v.length - 0.40, 0.05, 0.55), (0, side * (v.width / 2 - 0.03), 0.82), m["paint"], h,
            bevel=0.01)
        VP.light_with_guard(f"headlight_{s}", (top + 0.20, side * (v.width / 2 - 0.45), deck + 0.08), 0.07, m, h)
        VP.tow_hook(f"front_tow_{s}", (half - 0.02, side * 0.70, 0.82), m, h, size=0.13)
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.80, 0.82), m, h, size=0.13, rot=(0, 0, math.pi))
        # Tow cable along the hull's top edge, a tool box and the jerrycan rack on the rear.
        VP.cable(f"tow_cable_{s}", [(-half + 0.5, side * (v.width / 2 - 0.30), deck + 0.02),
                                    (0.0, side * (v.width / 2 - 0.30), deck + 0.03),
                                    (top - 0.6, side * (v.width / 2 - 0.30), deck + 0.02)], m, h, radius=0.018)
        VP.stowage_box(f"hull_bin_{s}", (-half + 1.6, side * (v.width / 2 - 0.42), deck), (1.0, 0.36, 0.30), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-half - 0.02, side * (v.width / 2 - 0.55), deck - 0.20), 0.05,
                            dict(m, lamp=m["tail"]), h, rot=(0, 0, math.pi))
    # The upper glacis rises from (half, nose) back to (top, deck).
    VP.bolted_panel("glacis_plate", ((top + half) / 2, 0, (deck + nose) / 2),
                    (math.hypot(half - top, deck - nose) * 0.8, v.width - 1.40, 0.03), m, h, bolts=(3, 4),
                    rot=(0, math.atan2(deck - nose, half - top), 0), bevel=0.01)
    if rear_engine:
        VP.grille("engine_grille", (-half + 1.0, 0, deck), (1.4, v.width - 1.2), m, h, slats=12)
        for side in (-1, 1):
            VP.exhaust(f"exhaust_{side}", (-half - 0.01, side * 0.75, deck - 0.30), 0.08, 0.30, m, h,
                       rot=(0, 0, math.pi))
    else:
        VP.grille("engine_grille", (top - 0.6, -0.55, deck), (1.0, 1.0), m, h, slats=9)
        VP.exhaust("exhaust", (top - 0.6, -(v.width / 2 - 0.35), deck - 0.15), 0.08, 0.35, m, h,
                   rot=(0, 0, -math.pi / 2))
    return top


def gepard(v):
    m, h = v.mats, v.hull
    deck = 1.45
    top = tracked_hull(v, deck, 7, 0.34, rear_engine=True)
    VP.hatch("driver_hatch", (top - 0.35, 0.80, deck), m, h, radius=0.26)
    z0, z1 = deck, deck + 0.95
    foot = [(1.15, 1.05), (-1.45, 1.05), (-1.45, -1.05), (1.15, -1.05)]
    crown = [(0.95, 0.95), (-1.35, 0.95), (-1.35, -0.95), (0.95, -0.95)]
    cyl("turret_ring", 1.10, 0.08, (0, 0, deck + 0.04), "Z", m["dark"], h, seg=36, lods=MID)
    loft("turret_shell", [(z0, foot), (z1, crown)], mat=m["paint"], parent=h, bevel=0.05)
    # The 35 mm guns outside each flank, in their cradles.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"gun_cradle_{s}", (1.20, 0.38, 0.45), (0.20, side * 1.28, z0 + 0.62), m["paint"], h, bevel=0.04)
        cyl(f"gun_barrel_{s}", 0.040, 2.75, (2.15, side * 1.28, z0 + 0.66), "X", m["steel"], h, seg=12)
        cyl(f"gun_sleeve_{s}", 0.075, 0.55, (1.05, side * 1.28, z0 + 0.66), "X", m["dark"], h, seg=14)
        cyl(f"gun_brake_{s}", 0.060, 0.22, (3.45, side * 1.28, z0 + 0.66), "X", m["dark"], h, seg=12, lods=MID)
        VP.stowage_box(f"ammo_box_{s}", (-0.70, side * 1.22, z0 + 0.10), (0.90, 0.30, 0.55), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.smoke_discharger_bank(f"smoke_{s}", (-0.30, side * 0.95, z1 - 0.05), m, h, count=4, tube_radius=0.045,
                                 tube_length=0.20, elevation=0.4, spread=0.3, rot=(0, 0, side * 1.4))
    # The tracking radar dish on the front, the search radar folded on the rear.
    cyl("tracking_radar", 0.42, 0.18, (1.05, 0, z0 + 0.65), "X", m["dark"], h, seg=24, bevel=0.03)
    cyl("tracking_radar_face", 0.36, 0.02, (1.15, 0, z0 + 0.65), "X", m["paint"], h, seg=24, lods=MID)
    cyl("tracking_radar_feed", 0.05, 0.30, (1.30, 0, z0 + 0.65), "X", m["dark"], h, seg=10, lods=NEAR)
    box("tracking_radar_yoke", (0.25, 0.95, 0.12), (0.95, 0, z0 + 0.20), m["paint"], h, bevel=0.02, lods=MID)
    # The search radar folded on its mast over the turret's rear: the top of
    # the travelling height.
    box("search_radar_mast", (0.25, 0.25, 0.45), (-1.05, 0, z1 + 0.22), m["dark"], h)
    box("search_radar", (0.30, 1.90, 0.55), (-1.05, 0, z1 + 0.62), m["paint"], h, bevel=0.05,
        rot=(0, -1.25, 0))
    box("search_radar_face", (0.02, 1.80, 0.48), (-0.92, 0, z1 + 0.66), m["dark"], h, rot=(0, -1.25, 0), lods=MID)
    VP.sight_housing("periscope_sight", (0.40, -0.55, z1 - 0.02), m, h, size=(0.36, 0.30, 0.30))
    VP.hatch("commander_hatch", (-0.15, 0.50, z1), m, h, radius=0.27)
    VP.hatch("gunner_hatch", (-0.15, -0.55, z1), m, h, radius=0.25)
    for k in range(3):
        VP.periscope(f"turret_periscope_{k}", (0.55, 0.20 + k * 0.22, z1), m, h)
    # The guns' ammunition feeds into the turret, the laser rangefinder, the
    # turret's rear stowage basket and the hull's driver periscopes.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"ammo_feed_{s}", (0.50, 0.25, 0.30), (-0.30, side * 1.12, z0 + 0.62), m["dark"], h, bevel=0.02,
            lods=MID)
        box(f"gun_yoke_{s}", (0.20, 0.30, 0.60), (0.55, side * 1.15, z0 + 0.45), m["paint"], h, bevel=0.03)
        cyl(f"gun_v0_radar_{s}", 0.07, 0.18, (3.15, side * 1.28, z0 + 0.66), "X", m["dark"], h, seg=12, lods=NEAR)
    box("rangefinder", (0.30, 0.22, 0.22), (0.90, -0.60, z1 - 0.15), m["dark"], h, bevel=0.02, lods=MID)
    box("rangefinder_glass", (0.01, 0.16, 0.12), (1.055, -0.60, z1 - 0.15), m["glass"], h, lods=MID)
    for k, y in enumerate((-0.9, 0.9)):
        box(f"basket_rail_{k}", (0.04, 0.04, 0.35), (-1.50, y, z0 + 0.65), m["dark"], h, lods=NEAR)
    box("basket", (0.30, 1.85, 0.35), (-1.55, 0, z0 + 0.55), m["dark"], h, bevel=0.01, lods=MID)
    tarp = dict(m)
    VP.tarp_roll("basket_tarp", (-1.55, 0, z0 + 0.82), 1.6, 0.13, tarp, h)
    for k in range(3):
        VP.periscope(f"driver_periscope_{k}", (top - 0.10, 0.55 + (k - 1) * 0.22, deck), m, h)
    whip = empty("dressing_antenna_turret", parent=h)
    VP.antenna("antenna_turret", (-1.20, -0.80, z1), m, whip, height=2.0)


def buk(v):
    m, h = v.mats, v.hull
    deck = 1.55
    top = tracked_hull(v, deck, 6, 0.36, rear_engine=False)
    half = v.length / 2
    # The cab across the front: three windscreens with armoured shutters
    # raised over them, roof hatches and periscopes, a door each side.
    loft("cab_body", [(deck, [(top - 1.3, -1.30), (top, -1.30), (top, 1.30), (top - 1.3, 1.30)]),
                      (deck + 0.75, [(top - 1.25, -1.20), (top - 0.35, -1.20), (top - 0.35, 1.20),
                                     (top - 1.25, 1.20)])], mat=m["paint"], parent=h, bevel=0.04)
    for k, y in enumerate((0.6, 0.0, -0.6)):
        box(f"windscreen_{k}", (0.03, 0.48, 0.30), (top - 0.20, y, deck + 0.40), m["glass"], h, rot=(0, -0.75, 0),
            lods=MID)
        box(f"windscreen_shutter_{k}", (0.04, 0.54, 0.32), (top - 0.30, y, deck + 0.82), m["paint"], h,
            rot=(0, -1.35, 0), bevel=0.01, lods=MID)
    for k, y in enumerate((0.55, -0.55)):
        VP.hatch(f"cab_hatch_{k}", (top - 0.85, y, deck + 0.75), m, h, radius=0.24)
        VP.periscope(f"cab_periscope_{k}", (top - 0.50, y, deck + 0.75), m, h)
    for side, s in ((1, "L"), (-1, "R")):
        VP.bolted_panel(f"cab_door_{s}", (top - 0.75, side * 1.305, deck + 0.05), (0.75, 0.60, 0.03), m, h,
                        bolts=(3, 2), rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
    tx = -0.9
    cyl("turntable", 1.20, 0.25, (tx, 0, deck + 0.12), "Z", m["dark"], h, seg=32)
    # The fire-control radar's housing at the turntable's front, its flat
    # antenna face forward, the TV/laser sight beside it.
    loft("radar_housing", [(deck + 0.25, _round(tx + 1.05, 0, 0.55, 1.05)),
                           (deck + 1.70, _round(tx + 1.05, 0, 0.45, 0.95))], mat=m["paint"], parent=h, bevel=0.05)
    cyl("radar_dome", 0.95, 0.30, (tx + 1.05, 0, deck + 1.85), "Z", m["paint"], h, seg=28, r2=0.70, bevel=0.03)
    cyl("radar_face", 0.80, 0.04, (tx + 1.52, 0, deck + 1.05), "X", m["dark"], h, seg=28, lods=MID)
    VP.sight_housing("tv_sight", (tx + 1.10, -1.05, deck + 1.25), m, h, size=(0.40, 0.30, 0.32))
    for k in range(4):
        box(f"radar_rib_{k}", (0.04, 0.04, 1.30), (tx + 1.05 + 0.48 * math.cos(1.2 + k * 0.6),
                                                   0.98 * math.sin(1.2 + k * 0.6), deck + 0.95), m["paint"], h,
            lods=FINE)
    # Six canisters in two rows of three, stowed flat to the rear: ribbed,
    # their caps on the rear ends, lifting lugs on top.
    for j in range(2):
        for i in range(3):
            y = (i - 1) * 0.62
            z = deck + 1.10 + j * 0.62
            box(f"canister_{j}_{i}", (5.0, 0.56, 0.56), (tx - 1.55, y, z), m["paint"], h, bevel=0.03)
            box(f"canister_cap_{j}_{i}", (0.03, 0.48, 0.48), (tx - 4.06, y, z), m["dark"], h, lods=MID)
            for k in range(3):
                box(f"canister_rib_{j}_{i}_{k}", (0.06, 0.58, 0.58), (tx - 3.6 + k * 1.6, y, z), m["paint"], h,
                    lods=NEAR)
    box("canister_cradle", (1.0, 2.0, 0.55), (tx + 0.10, 0, deck + 0.52), m["paint"], h, bevel=0.03)
    box("canister_rest", (0.20, 1.80, 0.95), (-half + 0.30, 0, deck + 0.48), m["dark"], h, lods=MID)
    cyl("elevating_ram", 0.09, 1.20, (tx - 0.60, 0, deck + 0.65), "X", m["steel"], h, seg=12, rot=(0, 0.35, 0),
        lods=MID)
    for side in (-1, 1):
        whip = empty(f"dressing_antenna_{side}", parent=h)
        VP.antenna(f"antenna_{side}", (top - 1.15, side * 1.0, deck + 0.75), m, whip, height=2.0)


def _round(x, y, rx, ry, n=16):
    return [(x + rx * math.cos(k * math.tau / n), y + ry * math.sin(k * math.tau / n)) for k in range(n)]


def pantsir(v):
    m, h = v.mats, v.hull
    deck = chassis(v, [3.55, 2.05, -1.55, -3.00], 0.62, 0.42, 1.00, 1.10, "cabover", 3.10, 1.95, 3.00)
    box("deck_plate", (6.5, 2.4, 0.10), (-1.75, 0, deck + 0.05), m["paint"], h, bevel=0.01)
    # The generator and crew compartment behind the cab: louvred, a door aft.
    VP.stowage_box("generator_box", (1.85, 0, deck), (1.40, 2.30, 1.20), m, h, rot=(0, 0, math.pi / 2))
    for side in (-1, 1):
        VP.grille(f"generator_louvre_{side}", (1.85, side * 1.16, deck + 0.65), (0.90, 0.60), m, h, slats=8,
                  rot=(-side * math.pi / 2, 0, 0))
    for side, s in ((1, "L"), (-1, "R")):
        VP.stowage_box(f"deck_box_{s}", (0.0, side * 1.0, deck + 0.10), (1.40, 0.36, 0.45), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        box(f"jack_leg_{s}", (0.16, 0.16, 0.85), (-v.length / 2 + 0.55, side * 1.10, deck - 0.30), m["dark"], h,
            bevel=0.01)
        box(f"jack_pad_{s}", (0.36, 0.36, 0.06), (-v.length / 2 + 0.55, side * 1.10, deck - 0.75), m["dark"], h,
            lods=MID)
    mx = -1.6
    box("module_base", (2.0, 2.0, 0.40), (mx, 0, deck + 0.25), m["paint"], h, bevel=0.03)
    cyl("module_ring", 1.0, 0.15, (mx, 0, deck + 0.52), "Z", m["dark"], h, seg=32)
    loft("module_body", [(deck + 0.60, [(mx + 0.90, -0.80), (mx - 1.00, -0.80), (mx - 1.00, 0.80), (mx + 0.90, 0.80)]),
                         (deck + 1.80, [(mx + 0.70, -0.70), (mx - 0.95, -0.70), (mx - 0.95, 0.70),
                                        (mx + 0.70, 0.70)])], mat=m["paint"], parent=h, bevel=0.04)
    VP.hatch("module_hatch", (mx - 0.50, 0.30, deck + 1.80), m, h, radius=0.25)
    # The search radar's phased array on its pedestal over the module, face
    # rearward as it travels; the tracking radar's flat array on the front,
    # the electro-optical station beside it.
    box("search_radar_pedestal", (0.40, 0.40, 0.20), (mx - 0.40, 0, deck + 1.90), m["dark"], h, lods=MID)
    box("search_radar", (0.50, 1.50, 1.00), (mx - 0.40, 0, deck + 2.35), m["paint"], h, bevel=0.04)
    box("search_radar_face", (0.02, 1.40, 0.90), (mx - 0.66, 0, deck + 2.35), m["dark"], h, lods=MID)
    for k in range(5):
        box(f"search_radar_rib_{k}", (0.03, 0.03, 0.92), (mx - 0.68, -0.56 + k * 0.28, deck + 2.35), m["paint"], h,
            lods=FINE)
    box("tracking_radar", (0.20, 0.90, 0.70), (mx + 0.85, 0, deck + 1.30), m["paint"], h, bevel=0.03)
    box("tracking_radar_face", (0.02, 0.80, 0.60), (mx + 0.96, 0, deck + 1.30), m["dark"], h, lods=MID)
    VP.sight_housing("eo_station", (mx + 0.50, 0, deck + 1.80), m, h, size=(0.42, 0.40, 0.32))
    for side, s in ((1, "L"), (-1, "R")):
        # A twin 30 mm gun each side in its shroud, the ammunition feed box.
        box(f"gun_mount_{s}", (0.80, 0.30, 0.40), (mx + 0.20, side * 0.90, deck + 1.20), m["paint"], h, bevel=0.03)
        box(f"ammo_feed_{s}", (0.45, 0.28, 0.30), (mx - 0.40, side * 0.90, deck + 1.10), m["dark"], h, bevel=0.02,
            lods=MID)
        for k, dz in enumerate((-0.08, 0.08)):
            cyl(f"gun_barrel_{s}_{k}", 0.030, 2.10, (mx + 1.65, side * 0.90, deck + 1.20 + dz), "X", m["steel"], h,
                seg=10)
            cyl(f"gun_shroud_{s}_{k}", 0.055, 0.60, (mx + 0.85, side * 0.90, deck + 1.20 + dz), "X", m["dark"], h,
                seg=12, lods=MID)
            cyl(f"muzzle_{s}_{k}", 0.045, 0.14, (mx + 2.68, side * 0.90, deck + 1.20 + dz), "X", m["dark"], h,
                seg=10, lods=NEAR)
        # Six missile tubes a side outside the guns, two columns of three,
        # on their frame.
        for c in range(2):
            for i in range(3):
                y = side * (1.02 + c * 0.20)
                cyl(f"missile_tube_{s}_{c}_{i}", 0.09, 3.20, (mx + 0.10, y, deck + 0.95 + i * 0.24), "X",
                    m["paint"], h, seg=14, bevel=0.01)
                cyl(f"missile_cap_{s}_{c}_{i}", 0.075, 0.01, (mx + 1.71, y, deck + 0.95 + i * 0.24), "X",
                    m["dark"], h, seg=12, lods=MID)
        for k, x in enumerate((mx - 1.0, mx + 1.2)):
            box(f"missile_frame_{s}_{k}", (0.06, 0.40, 0.72), (x, side * 1.12, deck + 1.19), m["dark"], h, lods=MID)
    VP.slat_armour("module_ladder", (mx - 1.05, 0.0, deck + 0.10), (0.6, 1.4), m, h, spacing=0.25,
                   rot=(0, 0, math.pi / 2))


def skyranger(v):
    m, h = v.mats, v.hull
    roof = boxer.ROOF
    boxer.drive(v)
    x = -0.80
    cyl("turret_ring", 0.95, 0.10, (x, 0, roof + 0.05), "Z", m["dark"], h, seg=32, lods=MID)
    loft("turret_shell", [(roof + 0.08, _round(x, 0, 1.00, 0.95, 8)), (roof + 0.75, _round(x - 0.10, 0, 0.80, 0.78, 8))],
         mat=m["paint"], parent=h, bevel=0.04)
    # The four radar panels round its top, each in a frame on the slope.
    for k in range(4):
        a = k * math.pi / 2 + math.pi / 4
        box(f"radar_panel_{k}", (0.06, 0.55, 0.40), (x + 0.82 * math.cos(a), 0.82 * math.sin(a), roof + 0.55),
            m["dark"], h, bevel=0.01, rot=(0, 0.25, a))
        box(f"radar_frame_{k}", (0.05, 0.62, 0.47), (x + 0.79 * math.cos(a), 0.79 * math.sin(a), roof + 0.55),
            m["paint"], h, bevel=0.01, rot=(0, 0.25, a), lods=NEAR)
    # The 30 mm gun in its mantlet, the barrel's sleeve and muzzle sensor.
    box("gun_cradle", (0.80, 0.34, 0.34), (x + 0.75, 0.25, roof + 0.55), m["paint"], h, bevel=0.03)
    box("gun_mantlet", (0.12, 0.46, 0.42), (x + 1.10, 0.25, roof + 0.56), m["paint"], h, bevel=0.03)
    cyl("kce_sleeve", 0.06, 0.55, (x + 1.42, 0.25, roof + 0.58), "X", m["dark"], h, seg=14, lods=MID)
    cyl("kce_barrel", 0.035, 2.10, (x + 2.20, 0.25, roof + 0.58), "X", m["steel"], h, seg=12)
    cyl("kce_muzzle", 0.050, 0.18, (x + 3.25, 0.25, roof + 0.58), "X", m["dark"], h, seg=12, lods=NEAR)
    # The missile launcher on its arm off the turret's right flank.
    box("missile_arm", (0.30, 0.30, 0.15), (x + 0.10, -0.85, roof + 0.45), m["dark"], h, bevel=0.02, lods=MID)
    box("missile_launcher", (1.10, 0.40, 0.45), (x + 0.10, -1.05, roof + 0.55), m["paint"], h, bevel=0.03)
    for i in range(2):
        for j in range(2):
            cyl(f"missile_cap_{i}_{j}", 0.075, 0.01, (x + 0.66, -1.05 + (i - 0.5) * 0.19, roof + 0.45 + j * 0.20), "X",
                m["dark"], h, seg=12, lods=MID)
    VP.sight_housing("eo_sensor", (x - 0.20, 0.45, roof + 0.75), m, h, size=(0.34, 0.30, 0.30))
    VP.hatch("turret_hatch", (x - 0.45, -0.25, roof + 0.75), m, h, radius=0.24)
    for side in (-1, 1):
        VP.smoke_discharger_bank(f"smoke_{side}", (x - 0.55, side * 0.70, roof + 0.62), m, h, count=4,
                                 tube_radius=0.04, tube_length=0.18, elevation=0.5, spread=0.3,
                                 rot=(0, 0, math.pi + side * 0.8))
    whip = empty("dressing_antenna_turret", parent=h)
    VP.antenna("antenna_turret", (x - 0.70, 0.40, roof + 0.70), m, whip, height=1.8)


def nasams(v):
    """The launcher on its two-axle trailer."""
    m, h = v.mats, v.hull
    half = v.length / 2
    rail_z = 0.95
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate((-0.55, -1.55)):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * 0.95, 0.48), 0.48, 0.35, side, m, h, rim_radius=0.27,
                          tread="bar", hub_bolts=8)
            box(f"axle_{s}_{k}", (0.16, 0.50, 0.14), (x, side * 0.55, 0.48), m["dark"], h, lods=NEAR)
        # One mudguard over the pair, a mudflap behind it.
        box(f"fender_{s}", (2.20, 0.48, 0.04), (-1.05, side * 0.95, 1.02), m["paint"], h, bevel=0.01, lods=MID)
        box(f"fender_lip_{s}", (2.20, 0.03, 0.16), (-1.05, side * 1.19, 0.95), m["paint"], h, lods=NEAR)
        VP.mudflap(f"mudflap_{s}", (-2.20, side * 0.95, 0.95), (0.40, 0.45), m, h)
        box(f"frame_rail_{s}", (v.length - 0.9, 0.14, 0.24), (-0.35, side * 0.55, rail_z), m["dark"], h, bevel=0.01)
        # Levelling legs at the corners, folded up, their winding handles.
        for k, x in enumerate((half - 0.6, -half + 0.3)):
            box(f"leg_{s}_{k}", (0.14, 0.14, 0.80), (x, side * 1.05, rail_z + 0.05), m["dark"], h, lods=MID)
            box(f"leg_pad_{s}_{k}", (0.30, 0.30, 0.05), (x, side * 1.05, rail_z - 0.38), m["dark"], h, lods=MID)
            box(f"leg_crank_{s}_{k}", (0.03, 0.14, 0.03), (x, side * 1.05, rail_z + 0.47), m["steel"], h, lods=FINE)
    # The A-frame drawbar and its eye, the jockey wheel.
    for side in (-1, 1):
        box(f"drawbar_{side}", (1.5, 0.10, 0.10), (half - 0.85, side * 0.28, 0.80), m["dark"], h,
            rot=(0, 0.10, -side * 0.35))
    cyl("drawbar_eye", 0.10, 0.04, (half - 0.02, 0, 0.62), "Z", m["steel"], h, seg=12, lods=MID)
    jockey = VP.wheel_node("wheel_jockey", (half - 0.55, 0.15, 0.12), 0.12, h)
    cyl("jockey_tyre", 0.12, 0.08, (0, 0, 0), "Y", m["rubber"], jockey, seg=16, lods=MID)
    box("jockey_leg", (0.06, 0.06, 0.60), (half - 0.55, 0.15, 0.50), m["steel"], h, lods=MID)
    box("deck_plate", (v.length - 1.2, 2.0, 0.10), (-0.40, 0, rail_z + 0.17), m["paint"], h, bevel=0.01)
    cyl("turntable", 0.65, 0.22, (-0.30, 0, rail_z + 0.33), "Z", m["dark"], h, seg=28)
    box("launcher_cradle", (1.0, 1.4, 0.30), (-0.30, 0, rail_z + 0.58), m["paint"], h, bevel=0.03)
    for side in (-1, 1):
        box(f"launcher_arm_{side}", (2.2, 0.08, 0.45), (-0.70, side * 1.10, rail_z + 1.20), m["paint"], h,
            bevel=0.015)
    # Six canisters, two rows of three: ribbed, caps forward, lugs on top.
    for j in range(2):
        for i in range(3):
            y = (i - 1) * 0.68
            z = rail_z + 0.98 + j * 0.62
            box(f"canister_{j}_{i}", (4.3, 0.62, 0.58), (-0.70, y, z), m["paint"], h, bevel=0.03)
            box(f"canister_cap_{j}_{i}", (0.03, 0.52, 0.50), (1.46, y, z), m["dark"], h, lods=MID)
            box(f"canister_end_{j}_{i}", (0.03, 0.52, 0.50), (-2.86, y, z), m["dark"], h, lods=MID)
            for k in range(3):
                box(f"canister_rib_{j}_{i}_{k}", (0.06, 0.64, 0.60), (-2.40 + k * 1.70, y, z), m["paint"], h,
                    lods=NEAR)
    VP.stowage_box("electronics_box", (half - 1.2, 0, rail_z + 0.22), (0.70, 1.40, 0.55), m, h)
    cyl("cable_reel", 0.22, 0.30, (half - 1.2, -0.95, rail_z + 0.30), "Y", m["dark"], h, seg=16, lods=MID)
    whip = empty("dressing_antenna", parent=h)
    VP.antenna("antenna", (half - 1.40, 0.60, rail_z + 0.77), m, whip, height=1.6)
    for side in (-1, 1):
        VP.light_with_guard(f"tail_light_{side}", (-half + 0.02, side * 0.95, rail_z), 0.05, dict(m, lamp=m["tail"]),
                            h, rot=(0, 0, math.pi))


def build(variant, v):
    {"europe_gepard_1a2": gepard, "eastern_buk_buk_m3_medium_range_air_defense": buk,
     "eastern_pantsir_pantsir_sm": pantsir, "europe_boxer_skyranger_30_gun_and_missile_air_defense": skyranger,
     "europe_nasams_nasams_3": nasams}[variant["id"]](v)


def wreck(variant, v):
    """An air defence vehicle after its fire: the right side's running gear
    torn (a track off, or wheels gone and the hull down on that side), the
    radar and canisters burst, plates and the module warped; what it
    throws is `wreckage.scatter`'s."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    remove("track_R_band", "wheel_R_1_", "wheel_R_2_", "skirt_R", "search_radar", "canister_cap_", "missile_cap_",
           "missile_tube_R_1_2", "missile_tube_R_0_2", "canister_1_2", "canister_rib_1_2", "radar_dome", "side_bin_R_",
           "mudflap_R", "windscreen", "door_window_", "rear_window_", "dressing_mirror_R", "mirror_R", "mirror_arm_R",
           "basket_tarp", "radar_panel_1", "radar_frame_1")
    shell = parts("hull_upper", "hull_lower", "turret_shell", "boxer_upper", "boxer_lower", "module_body", "cab_body",
                  "radar_housing", "canister_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.025, 0.8, seed=81.0), dent((0.0, -1.2, 1.6), 0.6, 0.12, (0, 1, -0.2)))
    v.root.rotation_euler = (0.03, 0.0, 0)
    v.root.location.z -= 0.05
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("air_defence", {c: dims for c, (_, _, dims) in CARDS.items()},
                 {c: scheme for c, (_, scheme, _) in CARDS.items()}, build, wreck, skip=("dressing_", "gun_", "fume_", "muzzle_", "brake_"), chip=1.0)
