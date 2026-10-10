"""Stryker M1126 ICV, M1127 RV, M1134 ATGM and M1296 Dragoon, from assets/references/stryker/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/stryker.py -- [--variant=<id>] [--wreck]

What the photos settle: eight black treaded tyres on true-size rims with CTIS
air lines, axles in two pairs with the wide gap amidships; the hull's
pronounced chine, a lower hull flaring out over the wheels to a sharp knuckle
just above the tyres, then upper sides leaning well in to the roof; a nose of
bolted applique plates coming to a point at the knuckle; a long sloped
glacis, the engine's grille on its right and the driver's hatch with three periscopes on its left; the
commander's hatch with its vision blocks on the right beside the remote
station; two squad hatches on the rear roof and the ramp in the rear plate;
bolted armour tiles in rows up the upper sides, stowage bins on their rear half,
jerrycans on the rear; the M151 Protector remote station with its M2 and two
smoke banks; the M1134's TOW launcher on its raised arm; the Dragoon's MCT-30
turret. The M1127 carries more radios. Crew stand in the hatches in the
M1126 and M1134 photos.

Built to the catalog frame (hull 6.95 x 2.72 x 2.64 m): nothing here moves
it. The roof is drawn at its real 2.30 m; the remote station's fixed base
rises to the frame's top. The Dragoon's frame puts its gun axis at 2.19 m,
below the real hull roof, so its hull is drawn lower (specs/done/unit-models/choices.md).
Each weapon stands on its frame's pivot, where the photos put it
(specs/off-centre-turrets.md): the remote station ahead of amidships, the
TOW launcher on the rear left, the Dragoon's turret behind amidships. A
squad hatch the weapon's base stands on is left off.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [2.00, 0.76, -0.89, -2.13]
WHEEL_R = 0.56
WHEEL_W = 0.38
WHEEL_Y = 1.15
# The chine: the lower hull flares out from the belly over the wheels to a
# sharp knuckle just above the tyres, flush with their outer faces, and the
# upper side leans in from there to the roof (photos: M1126 three-quarter
# front and rear, the Dragoon's side). The leaning side is most of the hull's
# visible height, so it reads at a distance.
BELLY = (0.52, 0.84)  # (z, half width)
KNUCKLE = (1.16, 1.34)
NOSE = 3.475  # the nose's point, at the knuckle
ROOF = 2.30
DRAGOON_ROOF = 1.95
SIDE_LEAN = math.radians(14)
GLACIS = math.radians(25)  # the upper glacis, from the nose's point up to the roof
COMMANDER_Y = -0.55
# How far each weapon's base reaches over the roof from its pivot: the
# remote station's fixed adapter, and the MCT-30 out to its bustle rack.
STATION_RADIUS = 0.34
MCT30_REACH = 1.27


def side_face(roof):
    """The upper side face, as (y, z) at the knuckle and at the roof
    (`vehicle_parts.on_side`)."""
    z, y = KNUCKLE
    return (y, z), (y - (roof - z) * math.tan(SIDE_LEAN), roof)


def glacis_top(roof):
    """Where the glacis meets the roof (x): further forward on a lower roof,
    so the glacis keeps its slope."""
    return NOSE - (roof - KNUCKLE[0]) / math.tan(GLACIS)


def glacis_z(x, roof):
    """The upper glacis' height at `x`, from the nose's point to the roof."""
    return KNUCKLE[0] + (NOSE - x) * math.tan(GLACIS) if x > glacis_top(roof) else roof


def nose_x(z):
    """Where the lower nose, from the belly's front up to the nose's point,
    is at height `z`."""
    (z0, _), (z1, _) = BELLY, KNUCKLE
    return 2.75 + (NOSE - 2.75) * (z - z0) / (z1 - z0)


def hull_rings(roof):
    """The hull as horizontal rings: the tucked-in belly, the knuckle over
    the wheels and the roof, its front edge where the glacis meets it."""

    def plan(rear, front, half, chamfer):
        return [(rear, -half), (front - chamfer, -half), (front, -half + chamfer), (front, half - chamfer),
                (front - chamfer, half), (rear, half)]

    return [(BELLY[0], plan(-3.22, 2.75, BELLY[1], 0.15)),
            (KNUCKLE[0], plan(-3.475, NOSE, KNUCKLE[1], 0.40)),
            (roof, plan(-3.44, glacis_top(roof), side_face(roof)[1][0], 0.25))]


def build(variant, v):
    ident = variant["id"]
    dragoon = "m1296" in ident
    roof = DRAGOON_ROOF if dragoon else ROOF
    v.roof = roof
    m, hull = v.mats, v.hull
    loft("stryker_hull", hull_rings(roof), mat=m["paint"], parent=hull, bevel=0.05)
    wheels(v)
    px, py, _ = v.frame["mounts"][0]["pivot_m"]
    fittings(v, roof, dragoon, base=(px, py, MCT30_REACH if dragoon else STATION_RADIUS))
    if dragoon:
        mounts = rig(v.frame, v.root)
        turret, gun, _, _ = mounts["autocannon"]
        mct30(v, turret, gun)
    elif "m1134" in ident:
        mounts = rig(v.frame, v.root)
        launcher, pitch, _, pivot = mounts["launcher"]
        tow_launcher(v, launcher, pitch, pivot)
    else:
        mounts = rig(v.frame, v.root)
        station, pitch, _, pivot = mounts["HMG"]
        protector(v, station, pitch, pivot)
    if "m1127" in ident:
        for k, (x, y) in enumerate(((-3.0, 0.95), (-3.0, -0.95), (-2.6, 1.05), (0.9, -1.0))):
            whip = empty(f"dressing_radio_antenna_{k}", parent=hull)
            VP.antenna(f"radio_antenna_{k}", (x, y, roof), m, whip, height=2.6)
    # Crew stand in the hatches (M1126 front, M1134 three-quarter front).
    if not v.wreck and not dragoon:
        seat = empty("dressing_commander", parent=hull)
        x, y, z = 0.25, COMMANDER_Y, roof
        v.crew.append(("commander", seat, (x, y, z - 0.30),
                       ((x + 0.25, y + 0.22, z + 0.08), (x + 0.25, y - 0.22, z + 0.08)),
                       ((x + 0.05, y + 0.12, z - 1.10), (x + 0.05, y - 0.12, z - 1.10))))


def wheels(v):
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.29, ctis=True)
            # Suspension arms reaching in from the hub to the belly.
            box(f"suspension_arm_{s}_{k}", (0.16, 0.42, 0.10), (x, side * (WHEEL_Y - 0.36), WHEEL_R + 0.05),
                m["dark"], hull, lods=NEAR)
            cyl(f"spring_{s}_{k}", 0.07, 0.38, (x - 0.18, side * (WHEEL_Y - 0.42), WHEEL_R + 0.30), "Z", m["dark"],
                hull, seg=10, lods=FINE)
        # Mudflaps hang behind the rear wheels from under the lower hull.
        VP.mudflap(f"mudflap_{s}", (-2.75, side * WHEEL_Y, 0.95), (0.40, 0.50), m, hull)


def side_top(x, roof):
    """The top of the upper side at `x`: the roof, or forward of it the edge
    the side shares with the glacis' chamfered corner (the rings' corners)."""
    front, back = NOSE - 0.40, glacis_top(roof) - 0.25
    if x <= back:
        return roof
    return KNUCKLE[0] + (roof - KNUCKLE[0]) * max(0.0, front - x) / (front - back)


def side_tiles(v, roof, side, s):
    """Bolted armour tiles in rows up the leaning side, ahead of the stowage:
    as many rows as the side is tall for, each tile lying on the face
    (`on_side`). Under the glacis' edge a tile is cut down to what fits, as
    the photos' tiles step down toward the nose, and left off below a
    useful height."""
    m = v.mats
    face = side_face(roof)
    cos = math.cos(SIDE_LEAN)
    foot, band = KNUCKLE[0] + 0.08, roof - KNUCKLE[0] - 0.14
    rows = max(1, int(band / 0.46))
    pitch = band / rows
    for r in range(rows):
        low = foot + r * pitch
        for k in range(3):
            x = 1.83 - k * 0.72
            high = min(low + (pitch - 0.04), side_top(x + 0.34, roof) - 0.05)
            if high - low < 0.24:
                continue
            tall = (high - low) / cos
            loc, rot = VP.on_side(x, (low + high) / 2, side, *face)
            VP.bolted_panel(f"armour_tile_{s}_{r}_{k}", loc, (0.68, tall, 0.04), m, v.hull,
                            bolts=(3, 3 if tall > 0.5 else 2), rot=rot, bevel=0.015)


def fittings(v, roof, dragoon, base=None):
    """The hull's fittings. `base` is the roof weapon's footprint, (x, y,
    radius): a squad hatch it would stand on is left off."""
    m, hull = v.mats, v.hull
    top = glacis_top(roof)
    face = side_face(roof)
    knuckle = KNUCKLE[0]

    def glacis(x):
        return glacis_z(x, roof)

    slope = GLACIS
    # The engine's grille on the right of the glacis, the driver's hatch and
    # periscopes on its left, the hatch outboard, clear of the remote
    # station behind it.
    VP.grille("engine_grille", (2.30, -0.55, glacis(2.30)), (0.80, 0.85), m, hull, slats=8, rot=(0, slope, 0))
    VP.hatch("driver_hatch", (top - 0.35, 0.66, roof), m, hull, radius=0.30)
    for k, y in enumerate((0.38, 0.60, 0.82)):
        x = top + 0.12
        VP.periscope(f"driver_periscope_{k}", (x, y, glacis(x) - 0.01), m, hull, size=(0.12, 0.17, 0.08),
                     rot=(0, slope, 0))
    # The bolted applique: the lower nose covered in two rows of plates, each
    # row as wide as the nose is where it starts (photos: front,
    # three-quarter front), and a plate beside the grille on the upper glacis.
    (z0, half0), (z1, half1) = BELLY, KNUCKLE
    run = math.hypot(NOSE - 2.75, z1 - z0)
    lower = math.pi - math.atan2(z1 - z0, NOSE - 2.75)
    for r, t in enumerate((0.25, 0.75)):
        z = z0 + t * (z1 - z0)
        half = (half0 - 0.15) + (half1 - 0.40 - half0 + 0.15) * (t - 0.25)
        for k, side in enumerate((1, -1)):
            VP.bolted_panel(f"nose_applique_{r}_{k}", (nose_x(z), side * (half / 2 + 0.01), z),
                            (run / 2 - 0.04, half - 0.05, 0.045), m, hull, bolts=(3, 4), rot=(0, lower, 0))
    VP.bolted_panel("glacis_applique", (2.45, 0.55, glacis(2.45)), (0.95, 0.80, 0.04), m, hull, bolts=(4, 3),
                    rot=(0, slope, 0))
    # Headlights in guards at the glacis corners, tow eyes on the nose.
    rear_half = face[0][0] - 0.45 * math.tan(SIDE_LEAN)
    for side, s in ((1, "L"), (-1, "R")):
        x = 3.05
        VP.light_with_guard(f"headlight_{s}", (x, side * 1.00, glacis(x) + 0.10), 0.07, m, hull,
                            rot=(0, 0, side * 0.15))
        z = knuckle - 0.08
        VP.tow_hook(f"front_tow_{s}", (nose_x(z) - 0.02, side * 0.80, z), m, hull, size=0.13)
        VP.shackle(f"front_shackle_{s}", (nose_x(z) + 0.08, side * 0.80, z - 0.04), m, hull, size=0.11,
                   rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-3.41, side * (rear_half - 0.18), knuckle + 0.45), 0.05,
                            dict(m, lamp=m["tail"]), hull, rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-3.40, side * 0.80, knuckle - 0.10), m, hull, size=0.12, rot=(0, 0, math.pi))
        # Armour tiles on the side's front half and stowage bins on its rear
        # half: both lean with the side.
        side_tiles(v, roof, side, s)
        for k in range(4):
            loc, rot = VP.on_side(-0.55 - k * 0.72, knuckle + 0.16, side, *face, proud=0.05, standing=True)
            VP.stowage_box(f"side_bin_{s}_{k}", loc, (0.68, 0.10, 0.48), m, hull, rot=rot)
    # The exhaust on the right, between the tiles and the bins, out of the side.
    loc, _ = VP.on_side(-0.08, knuckle + 0.30, -1, *face, proud=0.08)
    VP.exhaust("exhaust", loc, 0.07, 0.40, m, hull, rot=(0, 0, -math.pi / 2))
    # The commander's hatch and vision blocks, beside the remote station.
    if not dragoon:
        cyl("commander_ring", 0.40, 0.10, (0.25, COMMANDER_Y, roof + 0.05), "Z", m["paint"], hull, seg=24, bevel=0.01,
            lods=MID)
        for k in range(6):
            a = -0.9 + k * 0.6
            VP.periscope(f"commander_vision_{k}", (0.25 + 0.36 * math.cos(a), COMMANDER_Y + 0.36 * math.sin(a),
                                                   roof + 0.09),
                         m, hull, size=(0.10, 0.13, 0.07), rot=(0, 0, a))
        cyl("commander_lid", 0.32, 0.05, (-0.42, COMMANDER_Y, roof + 0.06), "Z", m["paint"], hull, seg=24, bevel=0.01,
            rot=(0, -0.08, 0), lods=MID)
    # Squad hatches on the rear roof, where the weapon's base leaves room
    # (with their coaming rings).
    for k, y in enumerate((0.50, -0.50)):
        x, size = -2.10, (1.00, 0.72)
        if base is not None:
            bx, by, reach = base
            near = (min(max(bx, x - size[0] / 2 - 0.04), x + size[0] / 2 + 0.04),
                    min(max(by, y - size[1] / 2 - 0.04), y + size[1] / 2 + 0.04))
            if math.dist(near, (bx, by)) < reach:
                continue
        VP.hatch(f"squad_hatch_{k}", (x, y, roof), m, hull, size=size)
    # The rear plate: the ramp with its door, hinged along its foot, and
    # jerrycans in racks either side of it.
    foot, head = knuckle - 0.12, roof - 0.15
    VP.weld_line("ramp_seam", [(-3.475, -0.80, foot), (-3.475, -0.80, head - 0.15),
                               (-3.475, 0.80, head - 0.15), (-3.475, 0.80, foot)], m, hull, radius=0.012)
    box("rear_ramp", (0.06, 1.56, head - foot), (-3.475, 0, (head + foot) / 2), m["paint"], hull, bevel=0.012)
    door = min(0.95, head - foot - 0.20)
    box("ramp_door", (0.03, 0.60, door), (-3.51, 0.35, foot + 0.10 + door / 2), m["paint"], hull, bevel=0.01,
        lods=NEAR)
    cyl("ramp_hinge", 0.04, 1.50, (-3.50, 0, foot - 0.03), "Y", m["steel"], hull, seg=10, lods=NEAR)
    for side in (-1, 1):
        VP.jerrycan(f"rear_jerrycan_{side}", (-3.39, side * 1.07, knuckle + 0.15), dict(m, paint=m["dark"]), hull,
                    rot=(0, 0, math.pi / 2))
    # The squad's rolled tarp across the rear roof.
    VP.tarp_roll("roof_tarp", (-3.10, 0, roof + 0.13), 1.60, 0.13, m, hull, straps=3)
    edge = face[1][0]
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-3.25, side * (edge - 0.08), roof), m, whip, height=2.8)


# ---------------------------------------------------------------- weapons
def station_base(v, pivot, height):
    """The remote station's fixed adapter on the roof, up to `height` m
    (hull frame): part of the hull, under the turning station."""
    m = v.mats
    base = v.roof
    cyl("station_adapter", STATION_RADIUS, height - base, (pivot.x, pivot.y, (height + base) / 2), "Z", m["paint"],
        v.hull, seg=24, bevel=0.012)
    cyl("station_bearing", 0.30, 0.06, (pivot.x, pivot.y, height + 0.03), "Z", m["dark"], v.hull, seg=24, lods=MID)


def protector(v, station, pitch, pivot):
    """The M151 Protector with its M2: the turning base, the cradle, the
    sight block on its left, the ammunition box on its right and a smoke bank
    on each side."""
    m = v.mats
    station_base(v, pivot, 2.56)
    below = 2.62 - pivot.z
    cyl("protector_base", 0.28, 0.10, (0, 0, below + 0.05), "Z", m["paint"], station, seg=20, bevel=0.01)
    box("protector_column", (0.30, 0.26, -below - 0.05), (-0.08, 0, below / 2 + 0.02), m["paint"], station,
        bevel=0.012)
    for side in (-1, 1):
        box(f"protector_cradle_{side}", (0.36, 0.05, 0.34), (0.0, side * 0.17, -0.08), m["paint"], station,
            bevel=0.012)
        VP.smoke_discharger_bank(f"protector_smoke_{side}", (-0.18, side * 0.34, -0.20), m, station, count=4,
                                 tube_radius=0.04, tube_length=0.22, elevation=0.45, spread=0.4,
                                 rot=(0, 0, side * 0.6))
    VP.sight_housing("protector_sight", (0.08, 0.30, -0.10), m, pitch, size=(0.30, 0.18, 0.24))
    box("protector_ammo", (0.30, 0.14, 0.26), (-0.02, -0.28, -0.08), m["paint"], pitch, bevel=0.012, lods=MID)
    VP.browning_m2(pitch, v.frame["mounts"][0]["muzzle_m"][0], m)


def tow_launcher(v, station, pitch, pivot):
    """The M1134's TOW turret: its fixed base on the roof, the arm raising
    the launcher, and the launcher box with its two tubes and the ITAS sight
    head beside them."""
    m = v.mats
    station_base(v, pivot, 2.56)
    below = 2.62 - pivot.z
    cyl("tow_turntable", 0.32, 0.10, (0, 0, below + 0.05), "Z", m["paint"], station, seg=20, bevel=0.01)
    box("tow_arm", (0.34, 0.30, -below), (-0.30, 0, below / 2), m["paint"], station, bevel=0.02)
    box("tow_arm_knuckle", (0.30, 0.42, 0.26), (-0.20, 0, -0.05), m["dark"], station, bevel=0.02, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("tow_box", (reach, 0.80, 0.50), (reach / 2 - 0.05, 0, 0.05), m["paint"], pitch, bevel=0.03)
    for side in (-1, 1):
        cyl(f"tow_tube_mouth_{side}", 0.15, 0.02, (reach - 0.04, side * 0.20, 0.05), "X", m["black"], pitch, seg=16,
            lods=MID)
        cyl(f"tow_tube_rim_{side}", 0.17, 0.05, (reach - 0.06, side * 0.20, 0.05), "X", m["dark"], pitch, seg=16,
            lods=NEAR)
    VP.sight_housing("itas_sight", (reach - 0.35, 0.53, -0.20), m, pitch, size=(0.42, 0.26, 0.50))
    box("tow_box_lid", (reach * 0.9, 0.82, 0.03), (reach / 2 - 0.05, 0, 0.31), m["dark"], pitch, lods=NEAR)


def mct30(v, turret, gun):
    """The Dragoon's Kongsberg MCT-30: a low, faceted turret on the roof with
    its 30 mm XM813 and coax, the commander's independent sight on its rear
    roof and stowage hung on its bustle."""
    m = v.mats
    base = v.roof - v.frame["mounts"][0]["pivot_m"][2]
    top = 2.58 - v.frame["mounts"][0]["pivot_m"][2]
    foot = [(0.95, 0.30), (0.62, 0.82), (-0.70, 0.86), (-1.05, 0.62), (-1.05, -0.62), (-0.70, -0.86), (0.62, -0.82),
            (0.95, -0.30)]
    crown = [(x * 0.86 - 0.08, y * 0.84) for x, y in foot]
    cyl("turret_ring", 0.92, 0.06, (0, 0, base + 0.03), "Z", m["dark"], turret, seg=32, lods=MID)
    loft("mct30_shell", [(base + 0.04, foot), (top, crown)], mat=m["paint"], parent=turret, bevel=0.03)
    VP.sight_housing("gunner_sight", (0.50, -0.42, top - 0.12), m, turret, size=(0.34, 0.26, 0.20))
    sight = empty("dressing_commander_sight", parent=turret)
    cyl("cits_pedestal", 0.10, 0.12, (-0.55, 0.40, top + 0.04), "Z", m["dark"], sight, seg=12, lods=MID)
    VP.sight_housing("cits_head", (-0.55, 0.40, top + 0.08), m, sight, size=(0.30, 0.28, 0.20))
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"turret_smoke_{s}", (0.30, side * 0.82, base + 0.30), m, turret, count=4,
                                 tube_radius=0.04, tube_length=0.22, elevation=0.4, spread=0.35,
                                 rot=(0, 0, side * 0.9))
        VP.tarp_roll(f"bustle_roll_{s}", (-1.00, side * 0.45, base + 0.22), 0.50, 0.14, m, turret, straps=2,
                     rot=(0, 0, 0))
    box("bustle_rack", (0.30, 1.50, 0.04), (-1.12, 0, base + 0.08), m["steel"], turret, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.40, 0.46, 0.34), (0.75, 0, 0.0), m["paint"], gun, bevel=0.03)
    cyl("cannon_sleeve", 0.075, 0.70, (1.20, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("cannon_barrel", 0.045, reach - 1.55, ((reach + 1.55) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("cannon_muzzle", 0.065, 0.20, (reach - 0.10, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("cannon_bore", 0.03, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.02, 0.45, (1.05, 0.22, 0.02), "X", m["steel"], gun, seg=8, lods=NEAR)


# ---------------------------------------------------------------- wreck
def wreck(variant, v):
    """The Stryker after its fire: the front left wheel blown off and the one
    behind it gone, the hull settled onto that corner, the ramp's door blown out, two side bins
    torn away and another hanging, the squad hatches blown off, plates warped
    and a dent where it was hit, torn plate on the ground. Whole (the Dragoon
    throws its turret: `wreckage.export_wreck` cuts the piece)."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "wheel_L_2_", "side_bin_R_1", "side_bin_R_2", "squad_hatch_", "roof_tarp",
           "rear_jerrycan_", "ramp_door")
    face = side_face(v.roof)
    hinge, _ = VP.on_side(0, KNUCKLE[0] + 0.64, 1, *face, proud=0.08)
    bend(parts("side_bin_L_2"), hinge, (1, 0, 0), (0, 0, -1), 0.5)
    shell = parts("stryker_hull", "armour_tile_", "nose_applique_", "glacis_applique")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=6.0), dent(VP.on_side(1.1, KNUCKLE[0] + 0.50, 1, *face)[0], 0.5, 0.12, (0, -1, -0.2)))
    for k, (loc, rot, size) in enumerate((((1.4, 1.62, 0.03), (0.03, 0.04, 0.9), 0.30),
                                          ((1.0, -1.62, 0.03), (-0.04, 0.02, 2.2), 0.28),
                                          ((-2.5, -0.45, v.roof + 0.04), (0.02, 0.04, 0.3), 0.36))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12, seed=31 + k)
    # With its front left wheels gone it settled onto that corner.
    v.root.rotation_euler = (-0.04, 0.035, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("stryker", "us_desert_tan", build, wreck, chip=0.6)
