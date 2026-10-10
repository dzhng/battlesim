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
T-15's sprocket is at the front and its nose is lower and sharper, its long
glacis running in one line up to the troop compartment's roof.

Each family's frame is its roster record's (fixtures/units/roster/eastern.json).
"""
import math
from functools import partial
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402
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
# The nose's edge, where the glacis meets the lower plate: both low, under
# a glacis raked at about 23 degrees on the T-14 (photos: side) and 16 on
# the T-15's longer one.
NOSE_Z = {T14: 1.02, T15: 1.05}
# How far the T-14's glacis runs back from the nose (the T-15's runs up to
# its troop compartment's roof: `glacis_top`).
T14_GLACIS_RUN = 2.10
# How much further the nose reaches past the front wheel (sprocket or
# idler) than the base 0.19 m: the long lower plate and the skirts' rising
# front ends make a pointed beak (photos: side, three-quarter front).
OVERHANG = {T14: 0.20, T15: 0.45}
# The skirts' bottom: the T-14's hang to the road wheels' tops, the wheels
# showing below (photos: side, three-quarter front); the T-15's cover them.
SKIRT_FOOT = {T14: 0.90, T15: 0.41}
LIP_TOP = 0.91
# The lower plate's rise toward the nose, which the skirts' front ends follow.
LOWER_RISE = 0.53
# The upper hull's side: vertical skirts up to the shoulder, then a band
# leaning in to the deck's edge (photos: three-quarter front, front). The
# T-14's is a narrow steep strip; the T-15's is deep and carries the slab
# modules on its front half.
SHOULDER = {T14: 1.70, T15: 1.40}
BAND_LEAN = {T14: math.radians(45), T15: math.radians(40)}
SHOULDER_Y = 1.70
# Toward the nose the skirts' tops follow the glacis down, this far under
# it: the T-14's stand just proud of it, the T-15's leave the slab band
# room above them, the band's full depth, so the band is even from the nose
# back to the glacis' head.
UNDER_GLACIS = {T14: -0.06, T15: DECK - SHOULDER[T15]}
SLAB = 0.12


def side_face(v):
    """The leaning band as (y, z) at the shoulder and at the deck's edge
    (`vehicle_parts.on_side`)."""
    ident = v.variant["id"]
    shoulder = SHOULDER[ident]
    return (SHOULDER_Y, shoulder), (SHOULDER_Y - (DECK - shoulder) * math.tan(BAND_LEAN[ident]), DECK)


def nose_z(v):
    return NOSE_Z[v.variant["id"]]


def overhang(v):
    return OVERHANG[v.variant["id"]]


def skirt_foot(v, x):
    """The skirts' bottom at `x`: level, rising ahead of the running gear,
    and always a little under their top."""
    rise = LOWER_RISE * max(0.0, x - (v.length / 2 - 0.30 - overhang(v)))
    return min(SKIRT_FOOT[v.variant["id"]] + rise, skirt_top(v, x) - 0.12)


def glacis_top(v):
    """Where the glacis meets the deck (x). The T-15's is on the line from its
    nose to the troop compartment's roof front, which its compartment's
    leaning front carries on up (photos: side, three-quarter front)."""
    half = v.length / 2
    if not front_engine(v):
        return half - T14_GLACIS_RUN
    nose = nose_z(v)
    return half - (DECK - nose) * (half - TROOP_FRONT) / (TROOP_ROOF - nose)


def glacis_z(v, x):
    """The glacis' height at `x`, from the nose to where it meets the deck."""
    half, nose = v.length / 2, nose_z(v)
    return nose + (DECK - nose) * (half - x) / (half - glacis_top(v))


def glacis_x(v, z):
    """Where the glacis is at height `z` (x)."""
    half, nose = v.length / 2, nose_z(v)
    return half - (z - nose) * (half - glacis_top(v)) / (DECK - nose)


def glacis_slope(v):
    """The glacis' fall toward the nose (radians), as a part lying on it is turned."""
    return math.atan((DECK - nose_z(v)) / (v.length / 2 - glacis_top(v)))


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
    nose = nose_z(v)
    (_, shoulder), (edge, _) = side_face(v)
    front = partial(glacis_x, v)

    def band(z):
        return SHOULDER_Y + (edge - SHOULDER_Y) * (z - shoulder) / (DECK - shoulder)

    # In horizontal rings: the hull between the skirts, stepping out to the
    # shoulder over them, then the band leaning in to the deck; the glacis
    # cuts each ring's bow.
    plan = VP.hull_plan
    loft("hull_upper", [(1.00, plan(-half, half - 0.20, 1.62, 0.30)),
                        (nose, plan(-half, half, 1.62, 0.30)),
                        (shoulder - 0.05, plan(-half, front(shoulder - 0.05), 1.62, 0.30)),
                        (shoulder, plan(-half, front(shoulder), SHOULDER_Y, 0.38)),
                        (DECK - 0.12, plan(-half, front(DECK - 0.12), band(DECK - 0.12), 0.38)),
                        (DECK, plan(-half + 0.10, top, edge, 0.38))], mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.40, 0.44), (half - 0.75 - overhang(v), 0.44), (half - 0.10, 1.02),
                         (-half, 1.02), (-half, 0.76)], 2.20, mat=m["paint"], parent=h, bevel=0.04)
    slope = glacis_slope(v)

    def glacis(x):
        return glacis_z(v, x)

    # A raised armour wedge down the glacis' centre, the T-14's "chin".
    prism("glacis_wedge", [(half - 0.05, nose + 0.04), (top + 0.30, glacis(top + 0.30) + 0.10),
                           (top + 0.30, glacis(top + 0.30) - 0.05), (half - 0.05, nose - 0.06)], 1.6,
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
        # The tow cables along the rear deck's edge, the T-15's along its
        # troop compartment's roof.
        y = side * (edge - 0.12)
        start = -half + 0.5
        end, z = (TROOP_FRONT - 0.2, TROOP_ROOF) if front_engine(v) else (top - 0.2, DECK)
        VP.cable(f"tow_cable_{s}", [(start + (end - start) * k / 3, y, z + 0.02) for k in range(4)], m, h,
                 radius=0.022)
    VP.slat_armour("rear_screen", (-half + 0.03, 0, 1.05), (3.10, 0.85), m, h, rot=(0, 0, math.pi / 2))
    return top

def running_gear(v):
    """Seven road wheels close together, set back 1.18 m from the front
    wheel and 0.96 m from the rear one (photos: side), the drive sprocket at
    the engine end."""
    half = v.length / 2
    rear = (-half + 0.45, 0.62, 0.33)
    front = (half - 0.50 - overhang(v), 0.66, 0.31)
    pitch = (front[0] - rear[0] - 1.18 - 0.96) / 6
    road_x = [front[0] - 1.18 - pitch * k for k in range(7)]
    sprocket, idler = (front, rear) if front_engine(v) else (rear, front)
    returns = [(road_x[1] + 0.3, 1.10, 0.10), (road_x[3], 1.12, 0.10), (road_x[5] - 0.3, 1.10, 0.10)]
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, road_x, ROAD_Z, ROAD_R, 0.24, sprocket, idler,
                            returns, bolts=8, ribs=0, teeth=13, arm=(0.46, -0.30), pitch=0.16, dual=True)


def skirts(v, slab=False):
    """Skirts in bolted sections up to the shoulder, their tops following
    the glacis down to the nose, with a rubber lip over the road wheels. The
    T-15 hangs its slab modules on the band beside the glacis, leaning in
    with it and falling with the glacis to the nose, and tall upright modules
    from the glacis' head to the tail, the first one's front carrying the
    glacis' line on up to the roof (photos: side, three-quarter front)."""
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
            outline = [(x0, skirt_foot(v, x0)), (x1, skirt_foot(v, x1))] + [(x, skirt_top(v, x)) for x in tops]
            prism(f"skirt_{s}_{k}", outline, 0.08, loc=(0, side * (SKIRT_Y - 0.05), 0), mat=m["paint"], parent=h,
                  bevel=0.025, lods=VP.ALL)
            for j, x in enumerate((x0 + 0.10, x1 - 0.10)):
                bolt = (x, side * (SKIRT_Y - 0.002), max(0.52, skirt_foot(v, x) + 0.11))
                cyl(f"skirt_{s}_{k}_bolt_{j}", 0.026, 0.03, bolt, "Y", m["steel"], h, seg=6, lods=FINE)
            # The lip stops where the skirt's front end narrows under it.
            cut = x1 - min(x1, glacis_x(v, LIP_TOP + UNDER_GLACIS[ident]))
            box(f"skirt_lip_{s}_{k}", (length - 0.05 - cut, 0.03, 0.18),
                ((x0 + x1) / 2 - cut / 2, side * (SKIRT_Y - 0.08), 0.82), m["rubber"], h, lods=MID)
        if slab:
            # Each slab stands on the skirt's top, which runs the band's depth
            # under the glacis, so every slab spans the band from the skirt to
            # the glacis' edge, falling with both toward the nose.
            lean = BAND_LEAN[ident]
            top = glacis_top(v)
            fall = (DECK - nose_z(v)) / (half - top)
            rise = UNDER_GLACIS[ident]
            # The band they lie on is one plane: along the hull it falls
            # with the glacis, across it leans in.
            along = Vector((1, 0, -fall)).normalized()
            up = Vector((0, -side * math.sin(lean), math.cos(lean)))
            up = (up - along * along.dot(up)).normalized()
            rot = tuple(Matrix((along, -side * up, along.cross(-side * up))).transposed().to_euler("XYZ"))
            for k in range(4):
                x = top + 0.34 + k * 0.66
                loc = (x, side * (SHOULDER_Y - rise / 2 * math.tan(lean)), skirt_top(v, x) + rise / 2)
                size = (0.66 / along.x - 0.06, rise / math.cos(lean) - 0.04, SLAB)
                VP.bolted_panel(f"slab_module_{s}_{k}", loc, size, m, h, bolts=(2, 2), rot=rot, bevel=0.03,
                                lods=VP.ALL)
            # Tall modules from the glacis' head to the tail, their tops level
            # with the roof; the first one's front rises along the glacis'
            # line from the deck to the roof's front.
            depth = SKIRT_Y + 0.01 - edge
            pitch = (top + half) / 5
            prism(f"rear_module_{s}_0", [(top - pitch + 0.02, shoulder), (top - 0.02, shoulder), (top - 0.02, DECK),
                                          (TROOP_FRONT, TROOP_ROOF), (top - pitch + 0.02, TROOP_ROOF)], depth,
                  loc=(0, side * (edge + depth / 2), 0), mat=m["paint"], parent=h, bevel=0.03, lods=VP.ALL)
            tall = TROOP_ROOF - shoulder
            for k in range(1, 5):
                VP.bolted_panel(f"rear_module_{s}_{k}", (top - pitch * (k + 0.5), side * edge, shoulder + tall / 2),
                                (pitch - 0.04, tall, depth), m, h, bolts=(3, 2), rot=(-side * math.pi / 2, 0, 0),
                                bevel=0.03, lods=VP.ALL)
        # The parade stripe on the skirts' front half, the number behind it.
        for j, (colour, z) in enumerate(((m["tail"], 0.16), (m["marking"], 0.08), (m["tail"], 0.0))):
            box(f"victory_stripe_{s}_{j}", (1.30, 0.008, 0.06), (0.9, side * (SKIRT_Y + 0.004), shoulder - 0.24 + z),
                colour, h, lods=NEAR)
        stencil(f"side_number_{s}", "112" if not slab else "203", 0.22,
                (-1.4, side * (SKIRT_Y + 0.004), shoulder - 0.20), (math.pi / 2, 0, math.pi if side > 0 else 0),
                m["marking"], h)


def armour_kit(v):
    """What the photos show bolted onto both Armatas beyond their hull and
    skirts: the heavier boxes on the skirts' front sections, track links
    carried on the glacis' foot, and on the T-14 the Malachit tile field over
    the upper glacis and the stowage and jerrycans on its rear deck."""
    m, h = v.mats, v.hull
    half = v.length / 2
    top = glacis_top(v)
    run, rise = half - top, DECK - nose_z(v)
    slope = math.atan2(rise, run)
    # The T-14's tile field; the T-15's glacis is one smooth plate.
    if not front_engine(v):
        x = top + run * 0.52
        VP.armour_tiles("glacis_era", (x, 0, glacis_z(v, x) + 0.005), (math.hypot(run, rise) * 0.72, 2.50), (2, 6),
                        0.04, m, h, rot=(0, slope, 0))
    for k in range(4):
        cyl(f"spare_link_{k}", 0.05, 0.62, (half - 0.30, -0.95 + k * 0.63, nose_z(v) + 0.05), "Y", m["track"], h,
            seg=8, lods=NEAR)
    # The skirt boxes fill the skirts' depth at their front end.
    foot = skirt_foot(v, half - 1.75)
    deep = min(0.62, skirt_top(v, half - 0.55) - foot - 0.06)
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"skirt_era_{s}", (half - 1.75, side * (SKIRT_Y - 0.01), foot + 0.03 + deep / 2),
                        (2.40, deep), (4, 2 if deep > 0.45 else 1), 0.07, m, h, rot=(-side * math.pi / 2, 0, 0))
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
    """The engine's grilles between `x0` and `x1`, on the deck or, ahead of
    its top, lying on the glacis (the T-15's), and the exhaust."""
    m, h = v.mats, v.hull
    mid = (x0 + x1) / 2
    on_glacis = mid > glacis_top(v)
    z, rot = (glacis_z(v, mid), (0, glacis_slope(v), 0)) if on_glacis else (DECK, (0, 0, 0))
    length = (x1 - x0) / math.cos(rot[1])
    VP.grille("engine_grille", (mid, 0.55, z), (length, 1.0), m, h, slats=12, rot=rot)
    VP.grille("radiator_grille", (mid, -0.55, z), (length, 1.0), m, h, slats=12, rot=rot)
    VP.bolted_panel("engine_access", (mid, 0, z), (length + 0.1, 0.12, 0.03), m, h, bolts=(4, 1), bevel=0.01,
                    rot=rot, lods=MID)
    # The exhaust out of the right band, behind the grilles on the deck or
    # ahead of them through the T-15's slab.
    face = side_face(v)
    at = x0 + 0.30 if on_glacis else x0 - 0.10
    loc, _ = VP.on_side(at, (face[0][1] + DECK) / 2, -1, *face, proud=(SLAB if front_engine(v) else 0) + 0.03)
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
