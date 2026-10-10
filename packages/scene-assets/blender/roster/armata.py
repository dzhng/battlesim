"""The Armata platform under the T-14 and T-15 (`roster/t14.py`,
`roster/t15.py`), from assets/references/t14/ and t15/.

What the photos settle (Moscow parades 2015-2016, Alabino, Army-2018): the
one Armata platform under both. Seven large road wheels a side with dished
discs, the drive sprocket at the engine end and the idler at the other; deep
side skirts in bolted sections with a rubber lip, the Victory stripe on the
parade cars, their tops falling with the glacis toward the nose and a band
leaning in from them to the deck's edge; a long shallow glacis rising to a
flat deck; headlights in boxes at the nose corners; a slat screen round the
tail. The T-14 has its engine at the rear, the T-15 at the front, so the
T-15's glacis is longer and its sprocket at the front.

Each family's frame is its roster record's (fixtures/units/roster/eastern.json).
"""
import math
from functools import partial
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, loft, prism, stencil  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

T14 = "eastern_t_14_armata_main_battle_tank"
T15 = "eastern_t_15_armata_heavy_ifv"
TRACK_Y = 1.36
TRACK_W = 0.58
ROAD_R = 0.37
ROAD_Z = 0.44
DECK = 1.92
# The T-15's troop compartment: a raised box behind the crew's deck, up to
# the top of its side modules; the module sits on its roof.
TROOP_ROOF = DECK + 0.30
TROOP_FRONT = 0.65
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
        start = TROOP_FRONT + 0.4 if front_engine(v) else -half + 0.5
        VP.cable(f"tow_cable_{s}", [(start + (top - 0.2 - start) * k / 3, y, DECK + 0.02) for k in range(4)], m, h,
                 radius=0.022)
    VP.slat_armour("rear_screen", (-half + 0.03, 0, 1.05), (3.10, 0.85), m, h, rot=(0, 0, math.pi / 2))
    return top

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
            # Tall modules from the compartment's front to the tail, their
            # tops level with its roof.
            tall = TROOP_ROOF - shoulder
            pitch = (TROOP_FRONT + half) / 4
            for k in range(4):
                VP.bolted_panel(f"rear_module_{s}_{k}", (TROOP_FRONT - pitch * (k + 0.5), side * edge,
                                                         shoulder + tall / 2),
                                (pitch - 0.04, tall, SKIRT_Y + 0.01 - edge), m, h, bolts=(3, 2),
                                rot=(-side * math.pi / 2, 0, 0), bevel=0.03, lods=VP.ALL)
        # The parade stripe on the skirts' front half, the number behind it.
        for j, (colour, z) in enumerate(((m["tail"], 0.16), (m["marking"], 0.08), (m["tail"], 0.0))):
            box(f"victory_stripe_{s}_{j}", (1.30, 0.008, 0.06), (0.9, side * (SKIRT_Y + 0.004), shoulder - 0.24 + z),
                colour, h, lods=NEAR)
        stencil(f"side_number_{s}", "112" if not slab else "203", 0.22,
                (-1.4, side * (SKIRT_Y + 0.004), shoulder - 0.20), (math.pi / 2, 0, math.pi if side > 0 else 0),
                m["marking"], h)


def armour_kit(v):
    """What the photos show bolted onto both Armatas beyond their hull and
    skirts: the Malachit tile field over the upper glacis, the heavier boxes
    on the skirts' front sections, the T-14's stowage and jerrycans on its
    rear deck, track links carried on the glacis' foot."""
    m, h = v.mats, v.hull
    half = v.length / 2
    top = glacis_top(v)
    run, rise = half - top, DECK - NOSE_Z
    slope = math.atan2(rise, run)
    x = top + run * 0.52
    VP.armour_tiles("glacis_era", (x, 0, glacis_z(v, x) + 0.005), (math.hypot(run, rise) * 0.72, 2.50),
                    (3 if front_engine(v) else 2, 6), 0.08, m, h, rot=(0, slope, 0))
    for k in range(4):
        cyl(f"spare_link_{k}", 0.05, 0.62, (half - 0.30, -0.95 + k * 0.63, NOSE_Z + 0.05), "Y", m["track"], h, seg=8,
            lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"skirt_era_{s}", (half - 1.75, side * (SKIRT_Y - 0.01), 0.95), (2.40, 0.62), (4, 2), 0.07,
                        m, h, rot=(-side * math.pi / 2, 0, 0))
    if not front_engine(v):
        for side, s in ((1, "L"), (-1, "R")):
            VP.jerrycan(f"jerrycan_{s}", (-half + 0.45, side * 1.25, DECK), dict(m, paint=m["dark"]), h,
                        rot=(0, 0, math.pi / 2))
        VP.stowage_box("deck_box", (-half + 0.45, 0, DECK), (0.40, 1.40, 0.40), m, h)


def turret_kit(v, turret, crown, low, high, x0, x1, z0, z1):
    """The turret's own armour and kit, from the photos: a band of armour
    modules along each leaning flank (`low`, `high` its face), laser-warning
    sensors on the roof's corners, and the stowage in the bustle cage."""
    m = v.mats
    for side, s in ((1, "L"), (-1, "R")):
        loc, rot = VP.on_side((x0 + x1) / 2, (z0 + z1) / 2, side, low, high)
        VP.armour_tiles(f"turret_era_{s}", loc, (x1 - x0, (z1 - z0) * 0.8), (max(2, round((x1 - x0) / 0.42)), 1), 0.06,
                        m, turret, rot=rot)
    VP.laser_warners("laser_warner", crown, high[1], m, turret)


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
