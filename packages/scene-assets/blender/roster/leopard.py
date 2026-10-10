"""Leopard 2A6, 2A7V and 2A8, from assets/references/leopard/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/leopard.py -- [--variant=<id>] [--wreck]

What the photos settle: seven dual road wheels a side, evenly spaced, four
return rollers behind the skirts, the drive sprocket raised at the rear and the
idler raised at the front; skirts in panel sections, the front three thick
ballistic panels with a sloped leading edge; a long shallow glacis rising to a
flat deck; the engine deck's grilles and the full-width louvred rear plate. The
turret is the A5-and-later arrowhead: wedge add-on armour either side of the
mantlet, meeting in a ridge at mid height, flat bolted sides, eight smoke tubes
a side behind the wedge, side stowage boxes and a rear basket on the bustle,
the gunner's sight box on the right front roof and the commander's PERI behind
it. 2A7V: the slab armour module across the hull front with its lamps let in,
bolted roof armour, the bustle's auxiliary unit and the FLW 100 weapon station.
2A8: Trophy's angular housings on the turret sides over the bustle, the FLW 200
with its heavy machine gun, and the turret front plates (from the Eurosatory
2022 demonstrator; no licensable photo of a series 2A8's turret roof).

Built to the catalog frame (hull 7.7 x 3.75 x 3.0 m, turret pivot 0.3 m ahead
of the hull's middle and 1.8 m up, cannon muzzle 7.12 m ahead and 0.54 m up):
nothing here moves it; the turret and the commander in his hatch are built at
the frame's pivot. The frame's
3.0 m box is taller than the hull and turret roof the photos show (about
2.6 m); the art stands at its real height inside it (specs/done/unit-models/choices.md). The wreck is cut into hull and turret pieces (wreckage.export_wreck).
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
TRACK_Y = 1.50
TRACK_W = 0.62
ROAD_R = 0.33
ROAD_Z = 0.41
ROAD_X = [2.40 - 0.805 * k for k in range(7)]
SPROCKET = (-3.12, 0.60, 0.32)
IDLER = (3.30, 0.62, 0.30)
RETURNS = [(1.98, 0.88, 0.10), (0.38, 0.89, 0.10), (-1.02, 0.89, 0.10), (-2.24, 0.88, 0.10)]
SKIRT_Y = 1.84  # the skirt's outer face
SKIRT_TOP, SKIRT_FOOT = 1.52, 0.80
DECK = 1.62
# Turret (its own frame, origin at the pivot).
FOOT = -0.14
ROOF = 0.81
TRUNNION = 1.10
CORE = [(1.30, 0.0), (1.30, 1.40), (-1.95, 1.45), (-2.30, 1.30), (-2.30, 0.0)]


def glacis_z(x):
    """The hull roof's height at x: the long shallow glacis, then the deck."""
    if x >= 2.10:
        return DECK - (x - 2.10) / 1.75 * 0.32
    return DECK


def build(variant, v):
    a7v = variant["variant"] in ("2A7V", "2A8")
    a8 = variant["variant"] == "2A8"
    hull_body(v, a7v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.50, 0.40), pitch=0.18)
    skirts(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, pivot = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret, a7v, a8)
    main_gun(v, gun)
    weapon_station(v, hmg, hmg_gun, a7v, a8)
    # The 2A6's commander rides head and shoulders out (photos: side).
    if not a7v:
        v.head_out("commander", turret, pivot.x - 0.95, -0.78, pivot.z + ROOF)


# ---------------------------------------------------------------- hull
def hull_body(v, a7v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 1.00), (3.78, 1.00), (half, 1.30), (2.10, DECK), (-3.78, DECK), (-half, 1.56)],
          3.56, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.55, 0.50), (3.15, 0.50), (3.80, 1.02), (-half, 1.02), (-half, 0.86)], 2.30,
          mat=m["paint"], parent=hull, bevel=0.04)
    # The driver's hatch on the right of the glacis head, periscopes ahead of it.
    VP.hatch("driver_hatch", (2.30, -0.62, glacis_z(2.30)), m, hull, size=(0.62, 0.52),
             rot=(0, math.atan(0.32 / 1.75), 0))
    slope = math.atan(0.32 / 1.75)
    for k, y in enumerate((-0.40, -0.62, -0.84)):
        VP.periscope(f"driver_periscope_{k}", (2.72, y, glacis_z(2.72) - 0.01), m, hull, size=(0.14, 0.16, 0.08),
                     rot=(0, slope, 0))
    VP.weld_line("glacis_weld", [(3.80, -1.70, 1.30), (3.80, 1.70, 1.30)], m, hull)
    VP.weld_line("deck_weld", [(2.10, -1.70, DECK + 0.004), (2.10, 1.70, DECK + 0.004)], m, hull)
    for side, s in ((1, "L"), (-1, "R")):
        if not a7v:
            VP.light_with_guard(f"headlight_{s}", (3.40, side * 1.30, glacis_z(3.40) + 0.10), 0.07, m, hull)
        VP.tow_hook(f"front_tow_{s}", (3.70, side * 0.80, 0.94), m, hull, size=0.15)
        VP.shackle(f"front_shackle_{s}", (3.80, side * 0.52, 0.94), m, hull, size=0.13, rot=(0, 0, math.pi / 2))
        VP.tow_hook(f"rear_tow_{s}", (-3.70, side * 0.85, 0.92), m, hull, size=0.14, rot=(0, 0, math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-3.80, side * 1.55, 1.38), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        VP.mudflap(f"rear_mudflap_{s}", (-3.90, side * TRACK_Y, 0.95), (0.56, 0.42), m, hull)
        VP.cable(f"tow_cable_{s}", [(-3.55, side * 1.66, DECK + 0.025), (-2.2, side * 1.68, DECK + 0.025),
                                    (-0.8, side * 1.68, DECK + 0.02), (0.9, side * 1.66, DECK + 0.02)], m, hull,
                 radius=0.024)
        cyl(f"fuel_cap_{s}", 0.10, 0.04, (-1.60, side * 1.42, DECK + 0.02), "Z", m["steel"], hull, seg=12, lods=NEAR)
        # Front track guards over the idlers.
        box(f"track_guard_{s}", (0.50, 0.66, 0.05), (3.62, side * TRACK_Y, 1.28), m["paint"], hull, bevel=0.015,
            rot=(0, 0.30, 0), lods=MID)
    # Engine deck: grilles over the radiators, access doors ahead of them.
    for k, y in enumerate((0.80, -0.80)):
        VP.grille(f"engine_grille_{k}", (-2.95, y, DECK), (1.35, 1.25), m, hull, slats=11)
        VP.bolted_panel(f"engine_door_{k}", (-1.65, y, DECK), (0.95, 1.20, 0.03), m, hull, bolts=(3, 2),
                        bevel=0.01, lods=MID)
    # The rear plate is one full-width louvre, two lamps under it.
    face = (0, -math.pi / 2, 0)
    VP.grille("rear_louvre", (-half, 0, 1.28), (0.48, 2.90), m, hull, slats=10, rot=face)
    stencil("rear_number", "Y-123 456", 0.11, (-half - 0.003, 0.0, 1.06), (math.pi / 2, 0, -math.pi / 2),
            m["marking"], hull)
    if a7v:
        # The armour module across the hull front, its lamps let into it.
        prism("front_module", [(3.30, 1.02), (3.88, 1.06), (3.88, 1.36), (2.95, 1.52)], 3.20, mat=m["paint"],
              parent=hull, bevel=0.05)
        for side, s in ((1, "L"), (-1, "R")):
            for k, y in enumerate((1.30, 0.55)):
                box(f"module_lamp_box_{s}_{k}", (0.10, 0.24, 0.16), (3.88, side * y, 1.22), m["dark"], hull,
                    bevel=0.015, lods=MID)
                box(f"module_lamp_{s}_{k}", (0.012, 0.16, 0.10), (3.935, side * y, 1.22), m["lamp"], hull, lods=NEAR)
            for k in range(3):
                cyl(f"module_bolt_{s}_{k}", 0.025, 0.02, (3.60 - k * 0.22, side * 1.40, 1.30 + k * 0.07), "Z",
                    m["steel"], hull, seg=6, lods=FINE)


def skirts(v):
    """Panels a side with their real breaks: the front three thick ballistic
    panels, the first with its sloped leading edge, then thinner panels to the
    sprocket, cut up over it; bolts along their tops and a handle on each."""
    m, hull = v.mats, v.hull
    panels = [[(2.78, SKIRT_FOOT + 0.06), (3.30, SKIRT_FOOT + 0.06), (3.78, 1.24), (3.78, SKIRT_TOP),
               (2.78, SKIRT_TOP)]]
    for front, rear in ((2.76, 1.80), (1.78, 0.82)):
        panels.append([(rear, SKIRT_FOOT), (front, SKIRT_FOOT), (front, SKIRT_TOP), (rear, SKIRT_TOP)])
    for front, rear in ((0.80, -0.16), (-0.18, -1.14), (-1.16, -2.12)):
        panels.append([(rear, SKIRT_FOOT), (front, SKIRT_FOOT), (front, SKIRT_TOP), (rear, SKIRT_TOP)])
    panels.append([(-3.60, 1.12), (-3.30, 1.00), (-2.90, SKIRT_FOOT), (-2.14, SKIRT_FOOT), (-2.14, SKIRT_TOP),
                   (-3.60, SKIRT_TOP)])
    for side, s in ((1, "L"), (-1, "R")):
        for k, outline in enumerate(panels):
            thick = 0.13 if k < 3 else 0.07
            y = side * (SKIRT_Y - thick / 2)
            prism(f"skirt_{s}_{k}", outline, thick, loc=(0, y, 0), mat=m["paint"], parent=hull,
                  bevel=0.035 if k < 3 else 0.02)
            xs = [p[0] for p in outline]
            mid = (min(xs) + max(xs)) / 2
            face = side * SKIRT_Y
            box(f"skirt_handle_{s}_{k}", (0.14, 0.04, 0.05), (mid, face + side * 0.02, SKIRT_TOP - 0.16), m["dark"],
                hull, lods=NEAR)
            for j in range(3):
                bx = min(xs) + (max(xs) - min(xs)) * (j + 0.5) / 3
                cyl(f"skirt_bolt_{s}_{k}_{j}", 0.026, 0.024, (bx, face + side * 0.008, SKIRT_TOP - 0.06), "Y",
                    m["steel"], hull, seg=6, lods=FINE)
        # The sponson's top edge over the skirts.
        box(f"sponson_lip_{s}", (6.6, 0.12, 0.06), (-0.30, side * (SKIRT_Y - 0.06), SKIRT_TOP + 0.03), m["paint"],
            hull, bevel=0.02, lods=MID)
        stencil(f"skirt_cross_{s}", "+", 0.34, (0.32, side * (SKIRT_Y + 0.002), 1.16),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- turret
def core_rings():
    left = CORE
    right = [(x, -y) for x, y in reversed(CORE) if y > 0]
    outline = left + right

    def ring(lean):
        return [(x, math.copysign(max(0.0, abs(y) - lean), y) if y else 0.0) for x, y in outline]

    return [(FOOT, ring(0.04)), (FOOT + 0.10, ring(0.0)), (ROOF, ring(0.07))]


def wedge(v, turret, side, s, a8):
    """The arrowhead add-on armour on one cheek: its faces slope back above
    and below a ridge at mid height, its point beside the mantlet."""
    m = v.mats
    back = [(1.30, 0.42), (1.78, 0.46), (1.02, 1.47), (0.28, 1.47)]
    ridge = [(1.30, 0.42), (2.36, 0.50), (1.22, 1.53), (0.28, 1.53)]
    top = [(1.30, 0.42), (1.70, 0.46), (0.98, 1.44), (0.28, 1.44)]
    rings = []
    for z, pts in ((FOOT + 0.04, back), (0.34, ridge), (ROOF - 0.03, top)):
        p = [(x, side * y) for x, y in pts]
        rings.append((z, p if side > 0 else p[::-1]))
    loft(f"wedge_{s}", rings, mat=m["paint"], parent=turret, bevel=0.045)
    for k, (x, y) in enumerate(((1.95, 0.80), (1.55, 1.10), (1.40, 0.70))):
        cyl(f"wedge_bolt_{s}_{k}", 0.028, 0.03, (x, side * y, 0.34 + 0.20), "Z", m["steel"], turret, seg=6,
            lods=FINE)
    if a8:
        # The demonstrator's flat plate on the wedge's upper face.
        box(f"wedge_plate_{s}", (0.60, 0.42, 0.03), (1.30, side * 1.00, 0.62), m["paint"], turret, bevel=0.012,
            rot=(side * 0.0, 0.48, 0), lods=MID)


def turret_body(v, turret, a7v, a8):
    m = v.mats
    loft("turret_shell", core_rings(), mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.10, 0.10, (0, 0, FOOT - 0.02), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        wedge(v, turret, side, s, a8)
        # Bolted side plates along the turret flank.
        for k, x in enumerate((-0.05, -0.95)):
            VP.bolted_panel(f"side_plate_{s}_{k}", (x, side * 1.42, 0.30), (0.82, 0.60, 0.04), m, turret,
                            bolts=(3, 2), bevel=0.012, rot=(-side * math.pi / 2, 0, 0), lods=MID)
        # Eight smoke tubes a side behind the wedge, two rows of four.
        for row in range(2):
            VP.smoke_discharger_bank(f"smoke_{s}_{row}", (0.10 - row * 0.02, side * 1.50, 0.46 + row * 0.15), m,
                                     turret, count=4, tube_radius=0.05, tube_length=0.30, elevation=0.30,
                                     spread=0.25, rot=(0, 0, side * 0.45))
        # Stowage boxes along the bustle sides.
        VP.stowage_box(f"side_bin_{s}", (-1.55, side * 1.58, 0.0), (1.25, 0.36, 0.68), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.05, side * 1.10, ROOF), m, whip, height=2.4)
        if a8:
            trophy_station(v, turret, side, s)
        elif a7v:
            VP.bolted_panel(f"side_module_{s}", (0.70, side * 1.48, 0.30), (0.62, 0.62, 0.10), m, turret,
                            bolts=(2, 2), bevel=0.03, rot=(-side * math.pi / 2, 0, 0), lods=VP.ALL)
    # Roof: the gunner's sight box at the front right, the PERI behind it,
    # the commander's and loader's hatches.
    VP.sight_housing("emes_sight", (1.00, -0.82, ROOF - 0.04), m, turret, size=(0.52, 0.48, 0.30))
    for k, y in enumerate((-0.64, -1.00)):
        box(f"emes_door_{k}", (0.04, 0.17, 0.22), (1.28, y, ROOF + 0.13), m["paint"], turret, bevel=0.01,
            lods=NEAR)
    cyl("peri_pedestal", 0.18, 0.22, (0.18, -1.00, ROOF + 0.11), "Z", m["paint"], turret, seg=20, bevel=0.02,
        lods=MID)
    VP.sight_housing("peri_head", (0.18, -1.00, ROOF + 0.20), m, turret, size=(0.36, 0.32, 0.26))
    VP.hatch("loader_hatch", (-0.55, 0.72, ROOF), m, turret, radius=0.34)
    commander_hatch(v, turret)
    for k in range(4):
        VP.periscope(f"loader_periscope_{k}", (-0.05, 0.40 + k * 0.20, ROOF), m, turret, size=(0.10, 0.14, 0.07))
    if a7v:
        for k, (x, y) in enumerate(((0.55, 0.62), (-1.45, 0.50), (-1.45, -0.55))):
            VP.bolted_panel(f"roof_armour_{k}", (x, y, ROOF), (0.80, 0.62, 0.05), m, turret, bolts=(2, 2),
                            bevel=0.015, lods=VP.ALL)
        # The bustle's auxiliary power and cooling unit.
        VP.bolted_panel("bustle_unit", (-2.55, 0, 0.12), (0.46, 1.50, 0.50), m, turret, bolts=(1, 3),
            bevel=0.04, lods=VP.ALL)
        VP.grille("bustle_unit_grille", (-2.785, 0, 0.40), (0.30, 1.10), m, turret, slats=5, rot=(0, -math.pi / 2, 0))
    else:
        bustle_basket(v, turret)


def commander_hatch(v, turret):
    """The commander's hatch ring with its periscopes, the lid swung open."""
    m = v.mats
    x, y = -0.95, -0.78
    cyl("commander_ring", 0.40, 0.09, (x, y, ROOF + 0.045), "Z", m["paint"], turret, seg=28, bevel=0.015, lods=MID)
    for k in range(6):
        a = -1.6 + k * 0.62
        VP.periscope(f"commander_periscope_{k}", (x + 0.36 * math.cos(a), y + 0.36 * math.sin(a), ROOF + 0.08), m,
                     turret, size=(0.10, 0.12, 0.06), rot=(0, 0, a))
    cyl("commander_lid", 0.34, 0.06, (x - 0.70, y, ROOF + 0.09), "Z", m["paint"], turret, seg=28, bevel=0.015,
        rot=(0, -0.10, 0), lods=MID)


def bustle_basket(v, turret):
    """The open basket behind the bustle with its load: a tarp roll and a bag."""
    m = v.mats
    x0, x1, wy, foot, top = -2.30, -2.68, 1.25, 0.10, 0.55
    box("basket_floor", (x0 - x1, 2 * wy, 0.03), ((x0 + x1) / 2, 0, foot), m["steel"], turret, lods=MID)
    box("basket_rail_rear", (0.04, 2 * wy, 0.04), (x1, 0, top), m["steel"], turret, lods=MID)
    for k in range(7):
        y = -wy + k * 2 * wy / 6
        box(f"basket_post_{k}", (0.035, 0.035, top - foot), (x1, y, (top + foot) / 2), m["steel"], turret, lods=NEAR)
    VP.tarp_roll("basket_tarp", (-2.48, 0.45, foot + 0.17), 1.20, 0.17, m, turret, straps=3)
    box("basket_bag", (0.32, 0.70, 0.30), (-2.48, -0.70, foot + 0.17), m["canvas"], turret, bevel=0.08, lods=MID)


def trophy_station(v, turret, side, s):
    """2A8 Trophy: the angular radar and launcher housing on the turret side
    over the bustle, its launcher head on top. Nodes `trophy_*` are the part's."""
    m = v.mats
    holder = empty(f"dressing_trophy_{s}", parent=turret)
    radar = empty(f"trophy_radar_{s}", (-1.55, side * 1.70, 0.62), holder)
    box(f"trophy_housing_{s}", (1.05, 0.34, 0.52), (0, 0, 0), m["paint"], radar, bevel=0.05,
        taper=(0.85, 0.80))
    box(f"trophy_panel_{s}", (0.72, 0.02, 0.34), (0.0, side * 0.175, 0.0), m["dark"], radar, bevel=0.01, lods=MID)
    launcher = empty(f"trophy_launcher_{s}", (-1.25, side * 1.66, 0.98), holder)
    box(f"trophy_pedestal_{s}", (0.22, 0.22, 0.14), (0, 0, -0.04), m["dark"], launcher, bevel=0.012)
    cyl(f"trophy_head_{s}", 0.14, 0.40, (0.02, 0, 0.12), "X", m["paint"], launcher, seg=16, bevel=0.02)


# ---------------------------------------------------------------- weapons
def main_gun(v, gun):
    """The 120 mm L55: the mantlet between the wedges, the thermal sleeve in
    sections, the bore evacuator a third of the way out, and the muzzle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.62, 0.76, 0.50), (0.32, 0, -0.02), m["paint"], gun, bevel=0.06)
    box("gun_shield_face", (0.05, 0.60, 0.40), (0.64, 0, -0.02), m["paint"], gun, bevel=0.015, lods=NEAR)
    cyl("coax_port", 0.035, 0.05, (0.66, 0.26, 0.05), "X", m["black"], gun, seg=10, lods=FINE)
    cyl("barrel_root", 0.15, 0.36, (0.80, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.015)
    evac = 2.30
    for k, (a, b) in enumerate([(0.98, evac - 0.40), (evac + 0.40, reach - 0.24)]):
        cyl(f"thermal_sleeve_{k}", 0.095, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k, x in enumerate((1.40, evac + 1.10, evac + 2.10, reach - 0.70)):
        cyl(f"sleeve_band_{k}", 0.104, 0.06, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("bore_evacuator", 0.16, 0.66, (evac, 0, 0), "X", m["paint"], gun, seg=28, bevel=0.02)
    cyl("evacuator_rear", 0.16, 0.10, (evac - 0.38, 0, 0), "X", m["paint"], gun, seg=28, r2=0.10, lods=MID)
    cyl("evacuator_front", 0.10, 0.10, (evac + 0.38, 0, 0), "X", m["paint"], gun, seg=28, r2=0.16, lods=MID)
    cyl("muzzle_end", 0.10, 0.24, (reach - 0.12, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.064, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)
    box("muzzle_reference", (0.12, 0.09, 0.08), (reach - 0.34, 0, 0.15), m["dark"], gun, lods=NEAR)


def mg3(parent, reach, mats):
    """A 7.62 mm MG3 on its cradle: receiver, perforated jacket, muzzle."""
    made = box("mg3_receiver", (0.44, 0.08, 0.12), (0.0, 0, 0), mats["dark"], parent, bevel=0.01)
    made += cyl("mg3_jacket", 0.032, reach - 0.50, ((reach + 0.22) / 2 + 0.03, 0, 0), "X", mats["dark"], parent,
                seg=10)
    made += cyl("mg3_muzzle", 0.026, 0.10, (reach - 0.05, 0, 0), "X", mats["steel"], parent, seg=8, lods=NEAR)
    made += box("mg3_belt_box", (0.16, 0.10, 0.14), (-0.02, 0.10, -0.06), mats["paint"], parent, bevel=0.012,
                lods=MID)
    return made


def weapon_station(v, hmg, hmg_gun, a7v, a8):
    """The machine gun on the commander's side: the 2A6's MG3 on a ring
    pintle, the 2A7V's FLW 100 with an MG3 and the 2A8's FLW 200 with an M2,
    each a sight block beside its gun."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    if not a7v:
        cyl("pintle_ring", 0.22, 0.05, (0, 0, -0.17), "Z", m["dark"], hmg, seg=20, lods=MID)
        cyl("pintle_post", 0.035, lift + 0.15, (0, 0, (lift + 0.15) / 2 - 0.17), "Z", m["steel"], hmg, seg=10)
        box("pintle_cradle", (0.20, 0.10, 0.08), (0.02, 0, lift - 0.07), m["dark"], hmg, lods=MID)
        mg3(hmg_gun, muzzle[0], m)
        return
    cyl("rws_bearing", 0.26, 0.10, (0, 0, -0.14), "Z", m["dark"], hmg, seg=24, bevel=0.012)
    box("rws_base", (0.50, 0.42, 0.12), (-0.02, 0, -0.03), m["paint"], hmg, bevel=0.025)
    for side in (-1, 1):
        box(f"rws_cradle_{side}", (0.30, 0.06, lift + 0.05), (0.02, side * 0.17, (lift + 0.05) / 2 + 0.03),
            m["paint"], hmg, bevel=0.015)
    VP.sight_housing("rws_sight", (0.10, -0.30, -0.12), m, hmg_gun, size=(0.30, 0.18, 0.24))
    box("rws_ammo", (0.30, 0.16, 0.26), (-0.05, 0.28, -0.08), m["paint"], hmg_gun, bevel=0.02, lods=MID)
    if a8:
        VP.browning_m2(hmg_gun, muzzle[0], m)
    else:
        mg3(hmg_gun, muzzle[0], m)


# ---------------------------------------------------------------- wreck
def wreck(variant, v):
    """The tank after its cook-off: the left track run off and lying behind it,
    two left road wheels and the rear left skirts gone, a front skirt bent out,
    plates warped by the fire, the glacis dented where it was hit, the
    loader's hatch blown onto the engine deck, the basket's load burnt away
    and torn plate on the ground. `wreckage.burn` then heaves the turret, sags
    the gun and burns every surface."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    import bpy
    m = v.mats
    for o in bpy.data.objects:
        if o.name.startswith("dressing_trophy_"):
            o.name = o.name.replace("dressing_", "", 1)
    # The turret died traversed a little left of the bow: with the ring ahead
    # of the hull's middle, the long L55 laid dead ahead would reach past the
    # ground the wreck may cover (its footprint allowance).
    bpy.data.objects["turret"].rotation_euler.z += math.radians(7.0)
    remove("track_L_band", "wheel_L_5_", "wheel_L_6_", "skirt_L_5", "skirt_L_6", "skirt_handle_L_5",
           "skirt_handle_L_6", "skirt_bolt_L_5", "skirt_bolt_L_6", "loader_hatch", "basket_tarp", "basket_bag",
           "skirt_cross_L")
    thrown = solid("thrown_track", (4.4, TRACK_W, 0.05), (-1.6, 2.15, 0.03), m["track"], v.hull, rot=(0, 0, -0.08),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=7.0))
    bend(parts("skirt_L_1"), (0, SKIRT_Y, SKIRT_TOP - 0.05), (1, 0, 0), (0, 0, -1), 0.6)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell", "wedge_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.024, 0.9, seed=9.0), dent((3.40, -0.70, 1.40), 0.55, 0.11, (-0.6, 0, -1)))
    plate("loader_lid", [(-0.32, -0.28), (0.34, -0.3), (0.34, 0.28), (-0.3, 0.32)], 0.05, (-2.0, -0.6, DECK + 0.05),
          (0.06, -0.12, 0.9), m["paint"], v.hull, seed=13)
    for k, (loc, rot, size) in enumerate((((2.6, 2.25, 0.03), (0.04, 0.02, 0.6), 0.48),
                                          ((-2.9, -2.10, 0.03), (-0.03, 0.05, 2.1), 0.36),
                                          ((4.3, -0.8, 0.03), (0.0, 0.06, 2.4), 0.30))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=31 + k)
    rest_on_ground(0.004)


run("leopard", "german_three_tone", build, wreck, chip=1.0)
