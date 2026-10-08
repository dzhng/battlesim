"""T-14 Armata and T-15 Armata (disabled cards), from assets/references/t14/ and t15/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/armata.py -- [--variant=<card id>] [--wreck]

What the photos settle (Moscow parades 2015-2016, Alabino, Army-2018): the
one Armata platform under both. Seven large road wheels a side with dished
discs, the drive sprocket at the engine end and the idler at the other; deep
side skirts in bolted sections with a rubber lip, the Victory stripe on the
parade cars, their tops falling with the glacis toward the nose and a band
leaning in from them to the deck's edge; a long shallow glacis rising to a
flat deck; headlights in boxes
at the nose corners; a slat screen round the tail.

- T-14: engine at the rear. The crew capsule's three hatches sit in a row on
  the deck ahead of the turret; the unmanned turret is a low faceted shell
  under a sloped sensor housing, the gunner's sight box on its right front,
  the commander's panoramic sight on a mast over its left, a remote Kord on
  the roof, the Afganit launcher racks low on each flank, a slatted cage
  round the bustle. The 125 mm 2A82-1M has a thermal sleeve and no fume
  extractor.
- T-15: engine at the front, so the glacis is longer and the troop
  compartment is behind; the sprocket is at the front. The Bumerang-BM
  module: a low box turret with its 30 mm 2A42 on the centre line and a pair
  of Kornet tubes on the left; big slab applique modules lie on the deep
  leaning band beside the glacis, tall upright modules stand on the rear
  half; a rear door with stowage either side.

Frames (`T14_DIMENSIONS`, `T15_DIMENSIONS` and their mounts):
T-14 8.7 x 3.5 x 3.3 m, gun pivot 1.98 m; T-15 9.5 x 3.5 x 3.5 m, gun pivot
1.82 m. The T-15's frame puts the 30 mm's axis at 2.32 m, about 0.4 m under
the real module's; the deck is drawn at the T-14's height and the module
sits low on it (specs/done/unit-models/choices.md).
"""
import math
import os
from functools import partial
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

T14 = "eastern_t_14_armata_main_battle_tank"
T15 = "eastern_t_15_armata_heavy_ifv"
# The frame its references give (a disabled card has no unit type): the
# hull box, and the mounts its turret and guns are rigged on for the art.
T14_DIMENSIONS = (8.7, 3.5, 3.3)
T14_MOUNTS = [
    dict(name="cannon", role="gun", on=None, pivot_m=[0.0, 0.0, 1.98], muzzle_m=[5.36, 0.0, 0.594]),
    dict(name="HMG", role="hmg", on="cannon", pivot_m=[-0.25, -0.58, 3.102], muzzle_m=[1.43, 0.0, 0.32]),
]
T15_DIMENSIONS = (9.5, 3.5, 3.5)
T15_MOUNTS = [
    dict(name="autocannon", role="gun", on=None, pivot_m=[0.0, 0.0, 1.82], muzzle_m=[2.94, 0.0, 0.504]),
    dict(name="launcher", role="hmg", on="autocannon", pivot_m=[0.0, 0.81, 2.296], muzzle_m=[1.05, 0.0, 0.0]),
]
TRACK_Y = 1.36
TRACK_W = 0.58
ROAD_R = 0.37
ROAD_Z = 0.44
DECK = 1.92
SKIRT_Y = 1.72
NOSE_Z = 1.22
# The upper hull's side: vertical skirts up to the shoulder, then a band
# leaning in to the deck's edge (photos: three-quarter front, front). The
# T-14's is a narrow steep strip; the T-15's is deep and carries the slab
# modules on its front half.
SHOULDER = {T14: 1.70, T15: 1.40}
BAND_LEAN = {T14: math.radians(45), T15: math.radians(40)}
SHOULDER_Y = 1.70
# Toward the nose the skirts' tops follow the glacis down, this far under
# it: the T-14's stand just proud of it, the T-15's leave the slab band
# room above them.
UNDER_GLACIS = {T14: -0.06, T15: 0.42}
SLAB = 0.12


def side_face(v):
    """The leaning band as (y, z) at the shoulder and at the deck's edge
    (`vehicle_parts.on_side`)."""
    ident = v.variant["id"]
    shoulder = SHOULDER[ident]
    return (SHOULDER_Y, shoulder), (SHOULDER_Y - (DECK - shoulder) * math.tan(BAND_LEAN[ident]), DECK)


def glacis_top(v):
    return v.length / 2 - (3.20 if front_engine(v) else 2.40)


def glacis_z(v, x):
    """The glacis' height at `x`, from the nose to where it meets the deck."""
    half = v.length / 2
    return NOSE_Z + (DECK - NOSE_Z) * (half - x) / (half - glacis_top(v))


def glacis_x(v, z):
    """Where the glacis is at height `z` (x)."""
    half = v.length / 2
    return half - (z - NOSE_Z) * (half - glacis_top(v)) / (DECK - NOSE_Z)


def skirt_top(v, x):
    """The skirts' top at `x`: the shoulder, or under the glacis near the nose."""
    ident = v.variant["id"]
    return min(SHOULDER[ident], glacis_z(v, x) - UNDER_GLACIS[ident])


def front_engine(v):
    return v.variant["id"] == T15


# ---------------------------------------------------------------- hull
def hull(v):
    """The Armata hull: a lower tub between the tracks, the wide upper hull
    over them with its long glacis and flat deck, skirts, lights, tow hooks,
    the rear slat screen. Returns the glacis' top (x)."""
    m, h = v.mats, v.hull
    half = v.length / 2
    top = glacis_top(v)
    nose_z = NOSE_Z
    (_, shoulder), (edge, _) = side_face(v)
    front = partial(glacis_x, v)

    def band(z):
        return SHOULDER_Y + (edge - SHOULDER_Y) * (z - shoulder) / (DECK - shoulder)

    # In horizontal rings: the hull between the skirts, stepping out to the
    # shoulder over them, then the band leaning in to the deck; the glacis
    # cuts each ring's bow.
    plan = VP.hull_plan
    loft("hull_upper", [(1.00, plan(-half, half - 0.20, 1.62, 0.30)),
                        (nose_z, plan(-half, half, 1.62, 0.30)),
                        (shoulder - 0.05, plan(-half, front(shoulder - 0.05), 1.62, 0.30)),
                        (shoulder, plan(-half, front(shoulder), SHOULDER_Y, 0.38)),
                        (DECK - 0.12, plan(-half, front(DECK - 0.12), band(DECK - 0.12), 0.38)),
                        (DECK, plan(-half + 0.10, top, edge, 0.38))], mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.40, 0.44), (half - 0.75, 0.44), (half - 0.10, 1.02), (-half, 1.02),
                         (-half, 0.76)], 2.20, mat=m["paint"], parent=h, bevel=0.04)
    slope = math.atan((DECK - nose_z) / (half - top))

    def glacis(x):
        return glacis_z(v, x)

    # A raised armour wedge down the glacis' centre, the T-14's "chin".
    prism("glacis_wedge", [(half - 0.05, nose_z + 0.04), (top + 0.30, glacis(top + 0.30) + 0.10),
                           (top + 0.30, glacis(top + 0.30) - 0.05), (half - 0.05, nose_z - 0.06)], 1.6,
          mat=m["paint"], parent=h, bevel=0.03)
    for k, y in enumerate((-1.05, 1.05)):
        VP.bolted_panel(f"glacis_plate_{k}", (half - 1.10, y, glacis(half - 1.10) - 0.01), (1.70, 0.95, 0.06), m, h,
                        bolts=(3, 2), bevel=0.02, rot=(0, slope, 0), lods=VP.ALL)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"headlight_box_{s}", (0.28, 0.42, 0.24), (half - 0.55, side * 1.30, glacis(half - 0.55) + 0.10),
            m["paint"], h, bevel=0.03, rot=(0, slope, 0))
        for j in range(2):
            VP.light_with_guard(f"headlight_{s}_{j}", (half - 0.40, side * (1.20 + 0.18 * j),
                                                       glacis(half - 0.40) + 0.13), 0.055, m, h)
        VP.tow_hook(f"front_tow_{s}", (half - 0.16, side * 0.70, 0.95), m, h, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.15, side * 0.80, 0.90), m, h, size=0.13, rot=(0, 0, math.pi))
        y = side * (edge - 0.12)
        VP.cable(f"tow_cable_{s}", [(-half + 0.5, y, DECK + 0.02), (-1.5, y, DECK + 0.02), (0.4, y, DECK + 0.02),
                                    (top - 0.2, y, DECK + 0.02)], m, h, radius=0.022)
    VP.slat_armour("rear_screen", (-half + 0.03, 0, 1.05), (3.10, 0.85), m, h, rot=(0, 0, math.pi / 2))
    return top
    # Tail lights in their guards on the rear plate.
    for side, sd in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{sd}", (-v.length / 2 - 0.02, side * (v.width / 2 - 0.55), 1.30), 0.05,
                            dict(v.mats, lamp=v.mats["tail"]), v.hull, rot=(0, 0, math.pi))

def running_gear(v):
    half = v.length / 2
    span = v.length - 2.0
    road_x = [half - 1.25 - span * k / 6 * 0.93 for k in range(7)]
    rear = (-half + 0.45, 0.62, 0.33)
    front = (half - 0.50, 0.66, 0.31)
    sprocket, idler = (front, rear) if front_engine(v) else (rear, front)
    returns = [(road_x[1] + 0.3, 1.10, 0.10), (road_x[3], 1.12, 0.10), (road_x[5] - 0.3, 1.10, 0.10)]
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, road_x, ROAD_Z, ROAD_R, 0.24, sprocket, idler,
                            returns, bolts=8, ribs=0, teeth=13, arm=(0.46, -0.30), pitch=0.16, dual=True)


def skirts(v, slab=False):
    """Deep skirts in bolted sections up to the shoulder, their tops
    following the glacis down to the nose, with a rubber lip over the road
    wheels. The T-15 hangs its slab modules angled on the band beside the
    glacis, and tall upright modules on its rear half."""
    m, h = v.mats, v.hull
    half = v.length / 2
    face = side_face(v)
    (_, shoulder), (edge, _) = face
    ident = v.variant["id"]
    sections = 7
    length = (v.length - 0.6) / sections
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(sections):
            x0, x1 = half - 0.30 - length * (k + 1) + 0.015, half - 0.30 - length * k - 0.015
            # In side view: its top at the shoulder, breaking at the knee
            # where it starts to follow the glacis down.
            knee = glacis_x(v, shoulder + UNDER_GLACIS[ident])
            tops = [x1] + ([knee] if x0 + 0.01 < knee < x1 - 0.01 else []) + [x0]
            outline = [(x0, 0.41), (x1, 0.41)] + [(x, skirt_top(v, x)) for x in tops]
            prism(f"skirt_{s}_{k}", outline, 0.08, loc=(0, side * (SKIRT_Y - 0.05), 0), mat=m["paint"], parent=h,
                  bevel=0.025, lods=VP.ALL)
            for j, x in enumerate((x0 + 0.10, x1 - 0.10)):
                cyl(f"skirt_{s}_{k}_bolt_{j}", 0.026, 0.03, (x, side * (SKIRT_Y - 0.002), 0.52), "Y", m["steel"], h,
                    seg=6, lods=FINE)
            box(f"skirt_lip_{s}_{k}", (length - 0.05, 0.03, 0.18), ((x0 + x1) / 2, side * (SKIRT_Y - 0.08), 0.82),
                m["rubber"], h, lods=MID)
        if slab:
            # Each slab stands on the skirt's top, leaning in with the band up
            # to the deck or the glacis; where the skirt's top follows the
            # glacis down, the slab falls with both, so the band runs on down
            # to the nose.
            lean = BAND_LEAN[ident]
            knee = glacis_x(v, shoulder + UNDER_GLACIS[ident])
            fall = (DECK - NOSE_Z) / (half - glacis_top(v))
            for k in range(5):
                x = 3.60 - k * 0.66
                sloped = x > knee
                foot = skirt_top(v, x if sloped else x + 0.31)
                rise = UNDER_GLACIS[ident] if sloped else min(DECK, glacis_z(v, x + 0.31)) - foot
                low = (SHOULDER_Y, foot)
                loc, rot = VP.on_side(x, foot + rise / 2, side, low, (SHOULDER_Y - math.tan(lean), foot + 1.0),
                                      fall=fall if sloped else 0.0)
                VP.bolted_panel(f"slab_module_{s}_{k}", loc, (0.62, rise / math.cos(lean) - 0.04, SLAB), m, h,
                                bolts=(2, 2), rot=rot, bevel=0.03, lods=VP.ALL)
            for k in range(4):
                tall = DECK + 0.30 - shoulder
                VP.bolted_panel(f"rear_module_{s}_{k}", (0.05 - k * 1.20, side * edge, shoulder + tall / 2),
                                (1.16, tall, SKIRT_Y + 0.01 - edge), m, h, bolts=(3, 2),
                                rot=(-side * math.pi / 2, 0, 0), bevel=0.03, lods=VP.ALL)
        # The parade stripe on the skirts' front half, the number behind it.
        for j, (colour, z) in enumerate(((m["tail"], 0.16), (m["marking"], 0.08), (m["tail"], 0.0))):
            box(f"victory_stripe_{s}_{j}", (1.30, 0.008, 0.06), (0.9, side * (SKIRT_Y + 0.004), shoulder - 0.24 + z),
                colour, h, lods=NEAR)
        stencil(f"side_number_{s}", "112" if not slab else "203", 0.22,
                (-1.4, side * (SKIRT_Y + 0.004), shoulder - 0.20), (math.pi / 2, 0, math.pi if side > 0 else 0),
                m["marking"], h)


def engine_deck(v, x0, x1):
    m, h = v.mats, v.hull
    mid = (x0 + x1) / 2
    VP.grille("engine_grille", (mid, 0.55, DECK), (x1 - x0, 1.0), m, h, slats=12)
    VP.grille("radiator_grille", (mid, -0.55, DECK), (x1 - x0, 1.0), m, h, slats=12)
    VP.bolted_panel("engine_access", (mid, 0, DECK), (x1 - x0 + 0.1, 0.12, 0.03), m, h, bolts=(4, 1), bevel=0.01,
                    lods=MID)
    # The exhaust out of the right band (through the T-15's slab there).
    face = side_face(v)
    loc, _ = VP.on_side(x0 - 0.10, (face[0][1] + DECK) / 2, -1, *face, proud=(SLAB if front_engine(v) else 0) + 0.03)
    VP.exhaust("exhaust", loc, 0.11, 0.30, m, h, rot=(0, 0, -math.pi / 2))


# ---------------------------------------------------------------- T-14
def t14(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    top = hull(v)
    running_gear(v)
    skirts(v)
    engine_deck(v, -half + 0.4, -2.0)
    # The crew capsule's three hatches across the deck ahead of the turret,
    # each with its periscope block.
    for k, y in enumerate((0.75, 0.0, -0.75)):
        VP.hatch(f"crew_hatch_{k}", (top - 0.55, y, DECK), m, h, radius=0.26)
        VP.periscope(f"crew_periscope_{k}", (top - 0.18, y, DECK), m, h, size=(0.12, 0.30, 0.10))
    mounts = rig(v.frame, v.root, trunnion={"cannon": 1.05})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    t14_turret(v, turret, base)
    gun_2a82(v, gun)
    kord_station(v, hmg, hmg_gun)


def t14_turret(v, turret, base):
    """The unmanned turret: a low faceted shell, its sloped front, the
    sensor housing over it, the gunner's sight box, the commander's sight
    mast, the Afganit racks and the bustle cage."""
    m = v.mats
    z0, z1 = base + 0.04, base + 0.95
    foot = [(1.80, 0.60), (1.45, 1.45), (-1.55, 1.50), (-2.05, 1.22), (-2.05, -1.22), (-1.55, -1.50), (1.45, -1.45),
            (1.80, -0.60)]
    crown = [(0.95, 0.55), (0.75, 1.26), (-1.45, 1.32), (-1.95, 1.06), (-1.95, -1.06), (-1.45, -1.32), (0.75, -1.26),
             (0.95, -0.55)]
    cyl("turret_ring_guard", 1.15, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=40, lods=MID)
    loft("turret_shell", [(z0, foot), (z0 + 0.45, [(x * 0.98, y) for x, y in foot]), (z1, crown)], mat=m["paint"],
         parent=turret, bevel=0.04)
    VP.roof_fittings("roof", crown, z1, m, turret, rails=True)
    # The gun's slot and the gunner's sight box on the right front.
    box("gun_slot", (0.30, 0.36, 0.40), (1.45, 0, base + 0.55), m["black"], turret, lods=MID)
    # The raised sensor brow across the front of the roof.
    prism("sensor_brow", [(1.05, z1 - 0.02), (0.55, z1 + 0.16), (-0.35, z1 + 0.16), (-0.35, z1 - 0.02)], 1.9,
          mat=m["paint"], parent=turret, bevel=0.03)
    VP.sight_housing("gunner_sight", (0.45, -0.95, z1 - 0.02), m, turret, size=(0.55, 0.40, 0.36))
    # The sensor housing's sloped roof plates and radar panels on the corners.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"radar_panel_{s}", (0.06, 0.36, 0.28), (1.0, side * 1.32, base + 0.55), m["dark"], turret, bevel=0.01,
            rot=(0, 0, side * 0.75))
        box(f"radar_panel_rear_{s}", (0.36, 0.06, 0.28), (-1.55, side * 1.40, base + 0.55), m["dark"], turret,
            bevel=0.01)
        # Afganit: a rack of launcher tubes low on each flank, pointing out and up.
        for j in range(5):
            cyl(f"afganit_{s}_{j}", 0.06, 0.38, (-0.25 - j * 0.15, side * 1.48, base + 0.30), "Y", m["paint"], turret,
                seg=12, rot=(side * -0.35, 0, 0), lods=MID)
            cyl(f"afganit_bore_{s}_{j}", 0.045, 0.01, (-0.25 - j * 0.15, side * 1.67, base + 0.37), "Y", m["black"],
                turret, seg=10, rot=(side * -0.35, 0, 0), lods=FINE)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 1.18, z1 - 0.10), m, turret, count=4, tube_radius=0.045,
                                 tube_length=0.20, elevation=0.35, spread=0.4, rot=(0, 0, side * 0.9))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.75, side * 0.85, z1), m, whip, height=2.0)
    # The commander's panoramic sight on its mast, over the left: the top of
    # the frame (its eye, 3.10 m).
    cyl("panorama_mast", 0.09, 0.14, (-0.30, 0.55, z1 + 0.07), "Z", m["dark"], turret, seg=12)
    VP.sight_housing("panorama_head", (-0.30, 0.55, z1 + 0.12), m, turret, size=(0.38, 0.34, 0.30))
    # The bustle cage of slats round the rear.
    VP.slat_armour("bustle_cage", (-2.20, 0, base + 0.10), (2.20, 0.60), m, turret, rot=(0, 0, math.pi / 2))
    for side, s in ((1, "L"), (-1, "R")):
        VP.slat_armour(f"bustle_cage_{s}", (-1.80, side * 1.20, base + 0.10), (0.75, 0.60), m, turret)


def gun_2a82(v, gun):
    """The 125 mm 2A82-1M: no fume extractor, a thermal sleeve in sections
    with clamps, the muzzle reference sensor near the end."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - 1.05
    box("mantlet", (0.70, 0.46, 0.42), (0.30, 0, 0), m["paint"], gun, bevel=0.06)
    cyl("barrel_root", 0.13, 0.60, (0.85, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.015)
    cyl("thermal_sleeve", 0.095, reach - 1.0, ((reach + 0.8) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k in range(5):
        cyl(f"sleeve_clamp_{k}", 0.105, 0.05, (1.1 + k * (reach - 1.6) / 4, 0, 0), "X", m["dark"], gun, seg=20,
            lods=NEAR)
    box("muzzle_sensor", (0.12, 0.08, 0.10), (reach - 0.35, 0, 0.13), m["dark"], gun, lods=NEAR)
    cyl("muzzle_end", 0.085, 0.20, (reach - 0.10, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.063, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)


def kord_station(v, hmg, hmg_gun):
    """The roof's remote Kord station: its bearing, body, sight and gun."""
    m = v.mats
    lift = v.frame["mounts"][1]["muzzle_m"][2]
    cyl("rws_bearing", 0.22, 0.08, (0, 0, -0.04), "Z", m["dark"], hmg, seg=20, bevel=0.01)
    box("rws_body", (0.46, 0.36, lift), (-0.05, 0, lift / 2), m["paint"], hmg, bevel=0.03)
    VP.sight_housing("rws_sight", (0.05, -0.26, -0.10), m, hmg_gun, size=(0.26, 0.14, 0.20))
    VP.kord(hmg_gun, v.frame["mounts"][1]["muzzle_m"][0], m)


# ---------------------------------------------------------------- T-15
def t15(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    top = hull(v)
    running_gear(v)
    skirts(v, slab=True)
    # The engine is under the long glacis: its grilles at the glacis' head.
    engine_deck(v, top - 0.2, top + 1.3)
    VP.hatch("driver_hatch", (top - 0.45, 0.65, DECK), m, h, radius=0.28)
    VP.periscope("driver_periscope", (top - 0.05, 0.65, DECK), m, h, size=(0.12, 0.30, 0.10))
    # Troop hatches on the rear deck, the rear door between stowage boxes.
    for k, y in enumerate((0.62, -0.62)):
        VP.hatch(f"troop_hatch_{k}", (-2.95, y, DECK), m, h, size=(0.90, 0.62))
    VP.bolted_panel("rear_door", (-half - 0.03, 0, 1.05), (0.06, 1.10, 0.80), m, h, bolts=(1, 3), bevel=0.015,
                    rot=(0, 0, 0), lods=VP.ALL)
    for side in (-1, 1):
        VP.stowage_box(f"rear_box_{side}", (-half + 0.14, side * 0.85, DECK), (0.25, 0.70, 0.45), m, h)
    mounts = rig(v.frame, v.root)
    turret, gun, _, _ = mounts["autocannon"]
    launcher, launcher_pitch, _, _ = mounts["launcher"]
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    bumerang_module(v, turret, gun, base)
    kornet_pair(v, launcher, launcher_pitch)


def bumerang_module(v, turret, gun, base):
    """The Bumerang-BM unmanned module: a low faceted box on a wide ring,
    sights on its roof, smoke banks, the 30 mm 2A42 on the centre line."""
    m = v.mats
    z0, z1 = base + 0.02, base + 0.62
    foot = [(1.05, 0.45), (0.85, 0.95), (-1.05, 1.00), (-1.25, 0.70), (-1.25, -0.70), (-1.05, -1.00), (0.85, -0.95),
            (1.05, -0.45)]
    crown = [(0.65, 0.40), (0.50, 0.85), (-0.95, 0.90), (-1.15, 0.62), (-1.15, -0.62), (-0.95, -0.90), (0.50, -0.85),
             (0.65, -0.40)]
    cyl("module_ring", 1.05, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=40, lods=MID)
    loft("module_shell", [(z0, foot), (z1, crown)], mat=m["paint"], parent=turret, bevel=0.035)
    VP.roof_fittings("roof", crown, z1, m, turret, periscopes=((-0.60, 0.55, 0.4),))
    VP.sight_housing("gunner_sight", (0.20, -0.55, z1 - 0.02), m, turret, size=(0.42, 0.32, 0.30))
    # The commander's panoramic sight stands on its pedestal over the
    # module: the top of the frame.
    cyl("panorama_post", 0.08, 0.62, (-0.55, -0.30, z1 + 0.31), "Z", m["dark"], turret, seg=12)
    VP.sight_housing("panorama_head", (-0.55, -0.30, z1 + 0.60), m, turret, size=(0.34, 0.32, 0.30))
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (-0.55, side * 0.95, z0 + 0.30), m, turret, count=3,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.4, spread=0.3,
                                 rot=(0, 0, side * 1.4))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.05, side * 0.55, z1), m, whip, height=2.0)
    VP.stowage_box("module_box", (-1.32, 0, z0), (0.30, 1.30, 0.40), m, turret, rot=(0, 0, math.pi / 2))
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("cannon_cradle", (0.60, 0.34, 0.30), (0.55, 0, 0), m["paint"], gun, bevel=0.04)
    cyl("cannon_sleeve", 0.07, 0.65, (1.10, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("cannon_barrel", 0.040, reach - 1.40, ((reach + 1.40) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("cannon_muzzle", 0.058, 0.22, (reach - 0.11, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("cannon_bore", 0.025, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.018, 0.40, (0.95, -0.20, 0.04), "X", m["steel"], gun, seg=8, lods=NEAR)


def kornet_pair(v, launcher, pitch):
    """Two Kornet tubes in their cradle on the module's left."""
    m = v.mats
    reach = v.frame["mounts"][1]["muzzle_m"][0]
    box("kornet_arm", (0.30, 0.16, 0.30), (-0.20, 0, -0.12), m["dark"], launcher, bevel=0.02)
    for k, z in enumerate((0.0, 0.20)):
        cyl(f"kornet_tube_{k}", 0.09, reach + 0.55, ((reach - 0.55) / 2, 0, z), "X", m["paint"], pitch, seg=16,
            bevel=0.01)
        cyl(f"kornet_cap_{k}", 0.075, 0.01, (reach, 0, z), "X", m["black"], pitch, seg=14, lods=MID)
        for j, x in enumerate((-0.30, reach - 0.20)):
            cyl(f"kornet_band_{k}_{j}", 0.098, 0.05, (x, 0, z), "X", m["dark"], pitch, seg=16, lods=NEAR)


def build(variant, v):
    (t15 if front_engine(v) else t14)(v)


# ---------------------------------------------------------------- wrecks
def wreck(variant, v):
    """An Armata after its fire: the left track run off with two road wheels
    gone, three skirt sections blown away (thrown by `wreckage.scatter`), one bent out,
    the rear screen crushed, plates warped and the glacis dented by the hit.
    Each throws what rides its gun mount, the T-14's turret and the T-15's
    module (`wreckage.burn`)."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    m = v.mats
    half = v.length / 2
    remove("track_L_band", "wheel_L_3_", "wheel_L_4_", "skirt_L_2_", "skirt_L_3_", "skirt_L_4_", "skirt_lip_L_2",
           "skirt_lip_L_3", "skirt_lip_L_4", "victory_stripe_L", "side_number_L", "slab_module_L_1", "crew_hatch_1",
           "troop_hatch_0")
    thrown = solid("thrown_track", (4.4, TRACK_W, 0.05), (-0.6, 2.25, 0.03), m["track"], v.hull, rot=(0, 0, -0.06))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=1.5))
    for k, (loc, rot) in enumerate((((1.5, 2.75, 0.10), (1.45, 0.15, 0.4)), ((0.2, 2.95, 0.10), (1.5, -0.2, -0.3)))):
        solid(f"fallen_skirt_{k}", (1.10, 0.08, 0.95), loc, m["paint"], v.hull, rot=rot, bevel=0.02)
    bend(parts("skirt_L_1_"), (half - 1.5, 1.70, skirt_top(v, half - 1.5)), (1, 0, 0), (0, 0, -1), 0.6)
    bend(parts("rear_screen"), (-half + 0.03, 0, 1.6), (0, 1, 0), (1, 0, 0), 0.35)
    shell = parts("hull_upper", "hull_lower", "glacis_wedge", "skirt_", "turret_shell", "module_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=13.0), dent((half - 1.0, 0.6, 1.45), 0.55, 0.12, (-0.6, 0, -1)))
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("armata", {T14: T14_DIMENSIONS, T15: T15_DIMENSIONS}, "russian_green", build, wreck,
                 mounts={T14: T14_MOUNTS, T15: T15_MOUNTS}, skip=("dressing_", "gun", "hmg"), chip=1.0)
