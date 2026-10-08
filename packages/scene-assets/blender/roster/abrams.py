"""M1A2 Abrams SEPv2, SEPv2 Trophy and SEPv3 Trophy, from assets/references/abrams/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/abrams.py -- [--variant=<id>] [--wreck]

What the photos settle: seven dual road wheels a side on torsion arms, a rear
sprocket and front idler both raised, two return rollers hidden by the
skirts; skirts in eight panels, the first a long wedge over the idler and the
last cut up over the sprocket; a long shallow glacis with the driver's hatch
and three periscopes at its head; the rear plate one wide exhaust grille
between two smaller ones, tail lights in guards at its corners; the turret's
flat, swept-back cheeks either side of a recessed gun shield, a squared
bustle with its rack and side stowage, six-tube smoke banks ahead of the side
boxes, the GPS doghouse on the right front roof, the CITV on the left, the
commander under a CROWS on the right and the loader's hatch on the left.
SEPv3: the auxiliary power unit's armoured box on the left rear deck, CROWS-LP
and the ammunition data link over the muzzle. Trophy: a radar and launcher
housing on each turret side over the side stowage, its launcher head on top
(from the SEPv2 Trophy photos; no licensable photo shows a SEPv3 with it).

Built to the catalog frame (hull 7.93 x 3.66 x 2.44 m, turret pivot 1.464 m,
cannon muzzle 5.805 m ahead): nothing here moves it. The wreck is cut into
hull and turret pieces (wreckage.export_wreck).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

# Running gear (hull frame, metres; +X forward, +Y left).
TRACK_Y = 1.40
TRACK_W = 0.64
ROAD_R = 0.32
ROAD_Z = 0.40  # a road wheel's centre: its radius plus the belt
ROAD_X = [2.42 - 0.80 * k for k in range(7)]
SPROCKET = (-3.43, 0.64, 0.31)
IDLER = (3.33, 0.56, 0.30)
RETURNS = [(1.25, 0.86, 0.10), (-1.05, 0.88, 0.10)]
SKIRT_Y = 1.765
SKIRT_TOP, SKIRT_FOOT = 1.38, 0.66
DECK = 1.48
# Turret (its own frame, origin at the pivot 1.464 m up).
ROOF = 0.86
TRUNNION = 1.55
# The turret's foot, front to rear, left half: the gun shield's recess, the
# swept cheek face, the side and the bustle.
TURRET_PLAN = [(1.45, 0.0), (1.45, 0.40), (2.18, 0.44), (1.40, 1.55), (0.55, 1.60), (-1.80, 1.60), (-2.05, 1.48),
               (-2.62, 1.36), (-2.62, 0.0)]


def glacis_z(x):
    """The hull roof's height at x: the glacis from the nose, then the deck."""
    if x >= 2.25:
        return 1.44 - (x - 2.25) / 1.715 * 0.36
    return 1.44 + (2.25 - x) / 6.05 * 0.04


def build(variant, v):
    m, hull = v.mats, v.hull
    ident = variant["id"]
    sep_v3 = "sep_v3" in ident
    trophy = "trophy" in ident
    hull_body(v, sep_v3)
    running_gear(v)
    skirts(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret, trophy)
    main_gun(v, gun, sep_v3)
    crows(v, hmg, hmg_gun, sep_v3)
    # The commander head and shoulders out of his open hatch (photos: SEPv3
    # three-quarter front).
    if not v.wreck:
        seat = empty("dressing_commander", parent=turret)
        z = 1.464 + ROOF
        v.crew.append(("commander", seat, (-0.84, -0.62, z - 0.43),
                       ((-0.62, -0.40, z + 0.06), (-0.62, -0.84, z + 0.06)),
                       ((-0.70, -0.52, z - 1.25), (-0.70, -0.74, z - 1.25))))


# ---------------------------------------------------------------- hull
def hull_body(v, sep_v3):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.98), (3.60, 0.98), (half, 1.08), (2.25, 1.44), (-3.80, DECK), (-half, 1.40)],
          3.40, mat=m["paint"], parent=hull, bevel=0.035)
    prism("hull_lower", [(-3.80, 0.46), (2.85, 0.46), (3.92, 1.0), (-3.95, 1.0)], 2.10, mat=m["paint"], parent=hull,
          bevel=0.03)
    # Glacis: the driver's hatch at its head, three periscopes in a row ahead.
    VP.hatch("driver_hatch", (2.02, 0, glacis_z(2.02)), m, hull, radius=0.30)
    slope = math.atan(0.36 / 1.715)
    for k, y in enumerate((-0.24, 0.0, 0.24)):
        VP.periscope(f"driver_periscope_{k}", (2.40, y, glacis_z(2.40) - 0.01), m, hull, size=(0.14, 0.18, 0.08),
                     rot=(0, slope, 0))
    VP.weld_line("glacis_weld", [(3.6, -1.6, glacis_z(3.6) + 0.004), (3.6, 1.6, glacis_z(3.6) + 0.004)], m, hull)
    for side, s in ((1, "L"), (-1, "R")):
        # Front fenders over the idlers, headlights in their guards behind them.
        box(f"front_fender_{s}", (0.52, 0.68, 0.05), (3.78, side * TRACK_Y, 1.06), m["paint"], hull, bevel=0.012,
            rot=(0, 0.25, 0), lods=MID)
        VP.light_with_guard(f"headlight_{s}", (3.56, side * 1.50, 1.26), 0.065, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.90, side * 1.05, 0.88), m, hull, size=0.14)
        VP.shackle(f"front_shackle_{s}", (4.00, side * 1.05, 0.84), m, hull, size=0.12, rot=(0, 0, math.pi / 2))
        VP.tow_hook(f"rear_tow_{s}", (-3.86, side * 0.95, 0.82), m, hull, size=0.13, rot=(0, 0, math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-3.92, side * 1.52, 1.30), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        # Tow cables along the sponson deck, under the turret's overhang.
        VP.cable(f"tow_cable_{s}", [(-3.55, side * 1.58, DECK + 0.025), (-2.0, side * 1.60, DECK + 0.025),
                                    (-0.4, side * 1.60, DECK + 0.02), (1.0, side * 1.58, DECK + 0.01)], m, hull,
                 radius=0.022)
        cyl(f"fuel_cap_{s}", 0.09, 0.035, (-1.95, side * 1.36, DECK + 0.015), "Z", m["steel"], hull, seg=12, lods=NEAR)
        for k, x in enumerate((2.0, -3.7)):
            box(f"lift_eye_{s}_{k}", (0.12, 0.035, 0.09), (x, side * 1.62, glacis_z(x) + 0.04), m["steel"], hull,
                lods=FINE)
    # Engine deck: two grille doors over the turbine, an access plate ahead.
    for k, y in enumerate((0.60, -0.60)):
        VP.grille(f"engine_grille_{k}", (-2.95, y, DECK - 0.005), (1.40, 1.0), m, hull, slats=10)
    box("deck_access", (1.0, 2.2, 0.025), (-1.55, 0, DECK + 0.008), m["paint"], hull, bevel=0.008, lods=MID)
    for k in range(6):
        cyl(f"deck_bolt_{k}", 0.02, 0.02, (-1.55 + (k % 3 - 1) * 0.45, (k // 3 - 0.5) * 2.0, DECK + 0.025), "Z",
            m["steel"], hull, seg=6, lods=FINE)
    # The rear plate: one wide exhaust grille between two smaller ones.
    face = (0, -math.pi / 2, 0)
    VP.grille("exhaust_grille", (-v.length / 2, 0, 1.02), (0.58, 1.30), m, v.hull, slats=9, rot=face)
    for k, y in enumerate((1.12, -1.12)):
        VP.grille(f"rear_grille_{k}", (-v.length / 2, y, 1.10), (0.42, 0.62), m, v.hull, slats=6, rot=face)
    if sep_v3:
        # The auxiliary power unit's armoured box on the left rear deck.
        VP.stowage_box("sepv3_uapu", (-3.30, 1.16, DECK), (0.72, 0.56, 0.32), m, hull)


def running_gear(v):
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        wheels = []
        for k, x in enumerate(ROAD_X):
            # A dual road wheel.
            VP.road_wheel(f"wheel_{s}_{k + 1}", (x, side * (TRACK_Y + 0.15), ROAD_Z), ROAD_R, 0.26, side, m, hull,
                          bolts=8, inner=(0.24, 0.30))
            # The torsion arm trailing back to the hull.
            box(f"road_arm_{s}_{k}", (0.50, 0.10, 0.12), (x - 0.22, side * (TRACK_Y - 0.36), ROAD_Z + 0.12),
                m["dark"], hull, rot=(0, 0.42, 0), lods=NEAR)
            wheels.append((x, ROAD_Z, ROAD_R))
        x, z, r = SPROCKET
        VP.sprocket(f"wheel_{s}_sprocket", (x, side * TRACK_Y, z), r, 0.50, m, hull, teeth=11)
        x, z, r = IDLER
        VP.idler(f"wheel_{s}_idler", (x, side * TRACK_Y, z), r, 0.50, m, hull)
        wheels += [SPROCKET, IDLER]
        for k, (x, z, r) in enumerate(RETURNS):
            VP.return_roller(f"wheel_{s}_return_{k + 1}", (x, side * TRACK_Y, z), r, 0.40, m, hull)
            wheels.append((x, z, r))
        VP.track_loop(f"track_{s}", wheels, side * TRACK_Y, TRACK_W, 0.19, m, hull)


def skirts(v):
    """Eight panels a side with their real breaks: a long wedge over the idler,
    six plain panels and the last cut up over the sprocket; bolts along their
    tops, a lifting handle on each. The front two are the thicker ballistic
    panels."""
    m, hull = v.mats, v.hull
    panels = [
        [(2.36, SKIRT_FOOT), (3.22, SKIRT_FOOT), (3.86, 1.04), (3.86, SKIRT_TOP), (2.36, SKIRT_TOP)],
        [(1.50, SKIRT_FOOT), (2.34, SKIRT_FOOT), (2.34, SKIRT_TOP), (1.50, SKIRT_TOP)],
    ]
    for front, rear in ((1.48, 0.66), (0.64, -0.18), (-0.20, -1.02), (-1.04, -1.86), (-1.88, -2.66)):
        panels.append([(rear, SKIRT_FOOT), (front, SKIRT_FOOT), (front, SKIRT_TOP), (rear, SKIRT_TOP)])
    panels.append([(-3.62, 1.06), (-3.30, 0.94), (-3.02, 0.72), (-2.68, SKIRT_FOOT), (-2.68, SKIRT_TOP),
                   (-3.62, SKIRT_TOP)])
    for side, s in ((1, "L"), (-1, "R")):
        for k, outline in enumerate(panels):
            thick = 0.10 if k < 2 else 0.07
            y = side * (SKIRT_Y + 0.035 - thick / 2)
            prism(f"skirt_{s}_{k}", outline, thick, loc=(0, y, 0), mat=m["paint"], parent=hull, bevel=0.018)
            xs = [p[0] for p in outline]
            mid = (min(xs) + max(xs)) / 2
            face = side * (SKIRT_Y + 0.035)
            box(f"skirt_handle_{s}_{k}", (0.12, 0.03, 0.035), (mid, face + side * 0.012, SKIRT_TOP - 0.10), m["dark"],
                hull, lods=FINE)
            for j in range(3):
                bx = min(xs) + (max(xs) - min(xs)) * (j + 0.5) / 3
                cyl(f"skirt_bolt_{s}_{k}_{j}", 0.02, 0.02, (bx, face + side * 0.006, SKIRT_TOP - 0.04), "Y", m["steel"],
                    hull, seg=6, lods=FINE)
        # The tactical chevron and number on the third panel.
        stencil(f"skirt_number_{s}", "< 32" if side > 0 else "32 >", 0.30, (1.07, side * (SKIRT_Y + 0.037), 1.02),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- turret
def turret_shell():
    plan = TURRET_PLAN
    left = plan
    right = [(x, -y) for x, y in reversed(plan) if y > 0]
    outline = left + right

    def ring(z):
        out = []
        for x, y in outline:
            a = abs(y)
            if z == 0.0:  # the undercut foot
                yy = y * 0.93 if a > 0.6 else y
                xx = x - 0.04 if x > 1.0 and a > 0.42 else x
            elif z == ROOF:  # the roof: cheeks swept back, sides leaning in
                xx = x - (0.30 if x > 1.0 and a > 0.42 else 0.05 if x > -2.0 else -0.02)
                yy = y if a <= 0.44 else math.copysign(a - 0.10, y)
                if x < -2.5:
                    xx = x + 0.06
            else:
                xx, yy = x, y
            out.append((xx, yy))
        return out

    return [(0.0, ring(0.0)), (0.16, ring(0.16)), (ROOF, ring(ROOF))]


def turret_body(v, turret, trophy):
    m = v.mats
    loft("turret_shell", turret_shell(), mat=m["paint"], parent=turret, bevel=0.03)
    cyl("turret_ring_guard", 1.05, 0.10, (0, 0, -0.02), "Z", m["dark"], turret, seg=40, lods=MID)
    # Roof: GPS doghouse (right front), loader's and commander's hatches, the
    # CITV (left front) and the wind sensor; blow-out panels over the bustle.
    VP.sight_housing("gps", (1.10, -0.72, ROOF - 0.06), m, turret, size=(0.56, 0.46, 0.24))
    VP.hatch("loader_hatch", (-0.55, 0.66, ROOF), m, turret, radius=0.32)
    commander_hatch(v, turret)
    citv = empty("dressing_citv", parent=turret)
    cyl("citv_pedestal", 0.14, 0.16, (0.95, 0.78, ROOF + 0.08), "Z", m["dark"], citv, seg=16, lods=MID)
    VP.sight_housing("citv_head", (0.95, 0.78, ROOF + 0.12), m, citv, size=(0.36, 0.34, 0.24))
    mast = empty("dressing_wind_sensor", parent=turret)
    cyl("wind_sensor_mast", 0.018, 0.42, (-1.62, 0.0, ROOF + 0.21), "Z", m["dark"], mast, seg=8, lods=NEAR)
    box("wind_sensor_vane", (0.12, 0.03, 0.05), (-1.62, 0.0, ROOF + 0.44), m["dark"], mast, lods=FINE)
    for k, y in enumerate((0.55, -0.55)):
        box(f"blowout_panel_{k}", (0.85, 0.75, 0.02), (-2.02, y, ROOF + 0.005), m["paint"], turret, bevel=0.006,
            lods=MID)
        for j in range(4):
            cyl(f"blowout_bolt_{k}_{j}", 0.018, 0.015, (-2.02 + (j % 2 - 0.5) * 0.72, y + (j // 2 - 0.5) * 0.62,
                                                       ROOF + 0.02), "Z", m["steel"], turret, seg=6, lods=FINE)
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.05, side * 1.18, ROOF), m, whip, height=2.2)
        VP.weld_line(f"cheek_weld_{s}", [(1.88, side * 0.46, ROOF + 0.002), (1.10, side * 1.43, ROOF + 0.002)], m,
                     turret)
        # The cheek's side face in two bolted armour plates, chunky edges.
        for k, (x, length) in enumerate(((0.99, 0.72),)):
            box(f"cheek_plate_{s}_{k}", (length, 0.04, 0.48), (x, side * 1.585, 0.42), m["paint"], turret,
                bevel=0.015, lods=MID)
            for j in range(4):
                cyl(f"cheek_bolt_{s}_{k}_{j}", 0.022, 0.02, (x + (j % 2 - 0.5) * (length - 0.12), side * 1.61,
                                                             0.42 + (j // 2 - 0.5) * 0.36), "Y", m["steel"],
                    turret, seg=6, lods=FINE)
        # Side stowage along the bustle sides, a rail over it.
        box_y = side * 1.71
        VP.stowage_box(f"side_bin_{s}", (-1.15, box_y, 0.26), (1.55, 0.22, 0.42), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        box(f"side_rail_{s}", (1.6, 0.03, 0.03), (-1.15, side * 1.80, 0.78), m["steel"], turret, lods=NEAR)
        for j in range(4):
            box(f"side_rail_post_{s}_{j}", (0.03, 0.03, 0.12), (-1.85 + j * 0.47, side * 1.80, 0.72), m["steel"],
                turret, lods=FINE)
        # Six-tube smoke banks ahead of the side bins, fanned forward and out.
        for row in range(2):
            VP.smoke_discharger_bank(f"smoke_{s}_{row}", (0.22, side * 1.55, 0.42 + row * 0.13), m, turret, count=3,
                                     tube_radius=0.055, tube_length=0.32, elevation=0.35, spread=0.30,
                                     rot=(0, 0, side * 0.55))
        if trophy:
            trophy_station(v, turret, side, s)
    bustle_rack(v, turret)


def commander_hatch(v, turret):
    """The commander's hatch ring with its periscopes, the lid thrown open."""
    m = v.mats
    x, y = -0.84, -0.62
    cyl("commander_ring", 0.40, 0.08, (x, y, ROOF + 0.04), "Z", m["paint"], turret, seg=28, bevel=0.012, lods=MID)
    for k in range(5):
        a = -1.0 + k * 0.5
        VP.periscope(f"commander_periscope_{k}", (x + 0.36 * math.cos(a), y + 0.36 * math.sin(a), ROOF + 0.07), m,
                     turret, size=(0.10, 0.12, 0.06), rot=(0, 0, a))
    # Thrown open rearward, resting on its stop behind the ring.
    cyl("commander_lid", 0.33, 0.05, (x - 0.70, y, ROOF + 0.08), "Z", m["paint"], turret, seg=28, bevel=0.012,
        rot=(0, -0.10, 0), lods=MID)
    box("commander_lid_stop", (0.06, 0.30, 0.10), (x - 0.42, y, ROOF + 0.05), m["dark"], turret, lods=NEAR)


def trophy_station(v, turret, side, s):
    """Trophy on one turret side: the radar housing over the side bin, its
    flat panels facing out and forward, and the launcher head on top at its
    front. Its nodes (`trophy_radar_*`, `trophy_launcher_*`) are the part's."""
    m = v.mats
    holder = empty(f"dressing_trophy_{s}", parent=turret)
    radar = empty(f"trophy_radar_{s}", (-1.45, side * 1.70, 0.74), holder)
    box(f"trophy_housing_{s}", (1.05, 0.30, 0.46), (0, 0, 0), m["paint"], radar, bevel=0.04)
    box(f"trophy_panel_side_{s}", (0.70, 0.02, 0.32), (-0.05, side * 0.16, 0.0), m["dark"], radar, bevel=0.01,
        lods=MID)
    box(f"trophy_panel_front_{s}", (0.02, 0.24, 0.32), (0.535, 0, 0.0), m["dark"], radar, lods=MID)
    box(f"trophy_bracket_{s}", (0.70, 0.12, 0.14), (0, -side * 0.18, -0.20), m["dark"], radar, lods=NEAR)
    launcher = empty(f"trophy_launcher_{s}", (-1.12, side * 1.70, 1.02), holder)
    box(f"trophy_pedestal_{s}", (0.20, 0.20, 0.12), (0, 0, -0.03), m["dark"], launcher, bevel=0.01)
    cyl(f"trophy_head_{s}", 0.13, 0.38, (0.02, 0, 0.12), "X", m["paint"], launcher, seg=16, bevel=0.015)
    cyl(f"trophy_head_face_{s}", 0.10, 0.01, (0.215, 0, 0.12), "X", m["dark"], launcher, seg=14, lods=NEAR)


def bustle_rack(v, turret):
    """The open rack behind the bustle, with its load: a tarp roll, a bin,
    jerrycans and a kit bag."""
    m = v.mats
    x0, x1, wy, foot, top = -2.62, -3.28, 1.42, 0.28, 0.76
    box("bustle_floor", (x0 - x1, 2 * wy, 0.03), ((x0 + x1) / 2, 0, foot), m["steel"], turret, lods=MID)
    box("bustle_rail_rear", (0.035, 2 * wy, 0.035), (x1, 0, top), m["steel"], turret, lods=MID)
    for side in (-1, 1):
        box(f"bustle_rail_side_{side}", (x0 - x1, 0.035, 0.035), ((x0 + x1) / 2, side * wy, top), m["steel"], turret,
            lods=MID)
    for k in range(7):
        y = -wy + k * 2 * wy / 6
        box(f"bustle_post_rear_{k}", (0.03, 0.03, top - foot), (x1, y, (top + foot) / 2), m["steel"], turret,
            lods=NEAR)
    for side in (-1, 1):
        box(f"bustle_post_side_{side}", (0.03, 0.03, top - foot), ((x0 + x1) / 2, side * wy, (top + foot) / 2),
            m["steel"], turret, lods=NEAR)
    VP.tarp_roll("bustle_tarp", (-2.95, 0.55, foot + 0.17), 1.30, 0.16, m, turret, straps=3)
    VP.stowage_box("bustle_bin", (-2.92, -0.80, foot + 0.015), (0.52, 0.66, 0.38), m, turret)
    for k in range(2):
        VP.jerrycan(f"bustle_jerrycan_{k}", (-3.12, -0.25 + k * 0.20, foot + 0.015), dict(m, paint=m["dark"]),
                    turret, rot=(0, 0, math.pi / 2))
    box("bustle_bag", (0.46, 0.52, 0.30), (-2.92, 1.05, foot + 0.17), m["canvas"], turret, bevel=0.07, lods=MID)


# ---------------------------------------------------------------- weapons
def main_gun(v, gun, sep_v3):
    """The 120 mm L44 in its gun shield: thermal sleeve in sections, the bore
    evacuator, the muzzle and the muzzle reference sensor over it."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.50, 0.76, 0.56), (0.15, 0, 0.0), m["paint"], gun, bevel=0.045)
    box("gun_shield_face", (0.04, 0.62, 0.44), (0.415, 0, 0.0), m["paint"], gun, bevel=0.01, lods=NEAR)
    cyl("coax_port", 0.03, 0.04, (0.43, -0.22, 0.06), "X", m["black"], gun, seg=10, lods=FINE)
    cyl("barrel_root", 0.12, 0.30, (0.55, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.01)
    evac = 1.95
    sections = [(0.70, evac - 0.32), (evac + 0.32, reach - 0.22)]
    for k, (a, b) in enumerate(sections):
        cyl(f"thermal_sleeve_{k}", 0.092, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k, x in enumerate((1.10, 1.45, evac + 0.75, evac + 1.25, reach - 0.60)):
        cyl(f"sleeve_band_{k}", 0.098, 0.045, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("bore_evacuator", 0.150, 0.50, (evac, 0, 0), "X", m["paint"], gun, seg=28, bevel=0.01)
    cyl("evacuator_rear", 0.150, 0.08, (evac - 0.29, 0, 0), "X", m["paint"], gun, seg=28, r2=0.095, lods=MID)
    cyl("evacuator_front", 0.095, 0.08, (evac + 0.29, 0, 0), "X", m["paint"], gun, seg=28, r2=0.150, lods=MID)
    cyl("muzzle_end", 0.098, 0.22, (reach - 0.11, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.062, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)
    box("muzzle_reference", (0.10, 0.08, 0.07), (reach - 0.32, 0, 0.14), m["dark"], gun, lods=NEAR)
    if sep_v3:
        box("ammunition_data_link", (0.14, 0.10, 0.06), (reach - 0.55, 0, 0.13), m["dark"], gun, lods=NEAR)


def crows(v, hmg, hmg_gun, sep_v3):
    """The commander's CROWS remote station with its M2: a bearing on the
    roof, a cradle, the sight block on its right and the ammunition box on
    its left. The SEPv3's CROWS-LP sits lower and squarer than the SEPv2's
    CROWS II, which carries an armour shield."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    cyl("crows_bearing", 0.26, 0.10, (0, 0, 0.02), "Z", m["dark"], hmg, seg=24, bevel=0.01)
    box("crows_base", (0.42, 0.36, 0.10), (-0.02, 0, 0.11), m["paint"], hmg, bevel=0.015)
    for side in (-1, 1):
        box(f"crows_cradle_{side}", (0.30, 0.05, lift + 0.06), (0.02, side * 0.15, (lift + 0.06) / 2 + 0.10),
            m["paint"], hmg, bevel=0.012)
    VP.sight_housing("crows_sight", (0.10, -0.29, -0.12), m, hmg_gun, size=(0.30, 0.16, 0.22))
    box("crows_ammo", (0.30, 0.14, 0.26), (-0.05, 0.27, -0.08), m["paint"], hmg_gun, bevel=0.015, lods=MID)
    if not sep_v3:
        box("crows_shield", (0.06, 0.62, 0.34), (0.32, 0, -0.03), m["paint"], hmg_gun, bevel=0.012, lods=MID)
    VP.browning_m2(hmg_gun, muzzle[0], m)


# ---------------------------------------------------------------- wreck
def wreck(variant, v):
    """The tank after its cook-off: the right track thrown flat beside it with
    three road wheels torn off, two right skirt panels gone and a left one
    bent out, plates warped by the fire, a dent in the glacis where it was
    hit, the loader's hatch blown off onto the deck, the bustle's load burnt
    away and torn plate on the ground. `wreckage.burn` then heaves the
    turret, sags the guns and burns every surface."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    import bpy
    m = v.mats
    # Trophy's housings are dressing on the live tank, but they are steel
    # boxes that survive the fire: they stay, so the Trophy tank's wreck is
    # its own.
    for o in bpy.data.objects:
        if o.name.startswith("dressing_trophy_"):
            o.name = o.name.replace("dressing_", "", 1)
    remove("track_R_band", "wheel_R_2_", "wheel_R_3_", "wheel_R_6_", "skirt_R_3", "skirt_R_4",
           "skirt_handle_R_3", "skirt_handle_R_4", "skirt_bolt_R_3", "skirt_bolt_R_4", "loader_hatch",
           "bustle_tarp", "bustle_bag", "bustle_jerrycan_", "skirt_number_R")
    thrown = solid("thrown_track", (3.9, TRACK_W, 0.05), (-0.4, -2.05, 0.03), m["track"], v.hull, rot=(0, 0, 0.06),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=3.0))
    bend(parts("skirt_L_5"), (0, SKIRT_Y, SKIRT_TOP - 0.05), (1, 0, 0), (0, 0, -1), 0.55)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=5.0), dent((3.35, 0.55, 1.20), 0.55, 0.10, (-0.6, 0, -1)))
    plate("loader_lid", [(-0.3, -0.25), (0.3, -0.28), (0.32, 0.26), (-0.28, 0.3)], 0.05, (-2.5, 0.75, DECK + 0.05),
          (0.05, -0.1, 0.7), m["paint"], v.hull, seed=11)
    for k, (loc, rot, size) in enumerate((((2.2, -2.2, 0.03), (0.04, 0.02, 0.4), 0.45),
                                          ((-2.8, 2.15, 0.03), (-0.03, 0.05, 1.9), 0.38),
                                          ((4.2, 0.9, 0.03), (0.0, 0.06, 2.7), 0.30))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=21 + k)
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("abrams", "us_desert_tan", build, wreck)
