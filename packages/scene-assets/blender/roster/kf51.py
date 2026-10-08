"""KF51 Panther (2022 prototype), from assets/references/kf51/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/kf51.py -- [--variant=<id>] [--wreck]

What the photos settle: two licensable photos of the prototype (front, and
front right in motion) and the KF-51U at Eurosatory 2024 for the hull, which
the U shares. The Leopard 2 running gear: seven dual road wheels a side,
four return rollers behind the armour, the sprocket raised at the rear and
the idler at the front. Big faceted armour panels cover the hull sides from
nose to tail; the nose plate carries a lamp band at its top corners and two
shackles. The turret is the long faceted one: front plates above and below a
horizontal edge meet in a nose that overhangs the mantlet, the gun leaving
through a slot; flat chined sides with rows of round smoke dischargers along
the lower face; the commander's tall sight box on the left of the roof, the
gunner's on the right front, the Natter remote weapon station on the right
and the HERO launcher box on the rear of the bustle. Not settled by any
photo: the prototype's turret roof and rear (gaps).

Built to the catalog frame (hull 7.8 x 3.7 x 2.5 m, turret pivot 1.5 m,
cannon muzzle 6.1 m ahead): nothing here moves it.
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
ROAD_R = 0.33
ROAD_Z = 0.41
ROAD_X = [2.42 - 0.81 * k for k in range(7)]
SPROCKET = (-3.15, 0.60, 0.32)
IDLER = (3.32, 0.62, 0.30)
RETURNS = [(1.98, 0.88, 0.10), (0.38, 0.89, 0.10), (-1.02, 0.89, 0.10), (-2.24, 0.88, 0.10)]
DECK = 1.46
PANEL_Y = 1.84
FOOT = -0.04
EDGE = 0.36  # the turret's chine, over its foot
ROOF = 0.90
TRUNNION = 1.20


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.50, 0.40), pitch=0.18)
    side_panels(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    main_gun(v, gun)
    weapon_station(v, hmg, hmg_gun)


# ---------------------------------------------------------------- hull
def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 1.00), (3.70, 1.00), (half, 1.06), (half, 1.30), (2.30, DECK), (-3.80, DECK),
                         (-half, 1.40)], 3.40, mat=m["paint"], parent=hull, bevel=0.06)
    prism("hull_lower", [(-3.55, 0.50), (3.20, 0.50), (3.85, 1.04), (-half, 1.04), (-half, 0.86)], 2.30,
          mat=m["paint"], parent=hull, bevel=0.04)
    VP.hatch("driver_hatch", (2.40, -0.58, DECK + 0.02), m, hull, size=(0.60, 0.50),
             rot=(0, math.atan(0.16 / 1.60), 0))
    for k, y in enumerate((-0.36, -0.58, -0.80)):
        VP.periscope(f"driver_periscope_{k}", (2.80, y, DECK - 0.02), m, hull, size=(0.14, 0.16, 0.08),
                     rot=(0, 0.10, 0))
    for side, s in ((1, "L"), (-1, "R")):
        # The lamp band at the nose plate's top corners.
        box(f"lamp_band_{s}", (0.05, 0.62, 0.12), (half - 0.01, side * 1.18, 1.22), m["black"], hull, lods=MID)
        for k in range(3):
            box(f"lamp_{s}_{k}", (0.012, 0.12, 0.06), (half + 0.016, side * (0.98 + k * 0.18), 1.22), m["lamp"], hull,
                lods=NEAR)
        VP.shackle(f"front_shackle_{s}", (half - 0.02, side * 0.56, 0.98), m, hull, size=0.16,
                   rot=(0, 0, math.pi / 2))
        VP.tow_hook(f"rear_tow_{s}", (-3.72, side * 0.85, 0.92), m, hull, size=0.14, rot=(0, 0, math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-3.82, side * 1.50, 1.30), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-3.55, side * 1.58, DECK + 0.025), (-2.0, side * 1.60, DECK + 0.025),
                                    (-0.4, side * 1.60, DECK + 0.02), (1.2, side * 1.58, DECK + 0.02)], m, hull,
                 radius=0.024)
    stencil("nose_name", "RHEINMETALL", 0.10, (half + 0.002, 0.0, 1.20), (math.pi / 2, 0, math.pi / 2), m["marking"],
            hull)
    for k, y in enumerate((0.78, -0.78)):
        VP.grille(f"engine_grille_{k}", (-2.95, y, DECK), (1.35, 1.20), m, hull, slats=11)
    VP.grille("rear_louvre", (-half, 0, 1.22), (0.42, 2.80), m, hull, slats=9, rot=(0, -math.pi / 2, 0))


def side_panels(v):
    """The big faceted armour panels down each side: the front one sloped,
    each with its lower face bevelled in toward the tracks."""
    m, hull = v.mats, v.hull
    breaks = [(3.85, 2.55), (2.52, 1.12), (1.09, -0.31), (-0.34, -1.74), (-1.77, -3.85)]
    for side, s in ((1, "L"), (-1, "R")):
        for k, (front, rear) in enumerate(breaks):
            nose = front - (0.40 if k == 0 else 0.0)
            outline = [(rear, 0.86), (nose, 0.86), (front, 1.18 if k == 0 else 0.86), (front, 1.50), (rear, 1.50)]
            prism(f"side_panel_{s}_{k}", outline, 0.12, loc=(0, side * (PANEL_Y - 0.06), 0), mat=m["paint"],
                  parent=hull, bevel=0.05, taper_y=lambda z: 1.0 if z > 1.0 else 0.6)
            for j, x in enumerate((rear + 0.20, front - 0.24)):
                cyl(f"panel_bolt_{s}_{k}_{j}", 0.03, 0.02, (x, side * (PANEL_Y + 0.004), 1.40), "Y", m["steel"], hull,
                    seg=6, lods=FINE)
        box(f"panel_lip_{s}", (7.6, 0.16, 0.05), (0.0, side * (PANEL_Y - 0.08), 1.52), m["paint"], hull, bevel=0.02,
            lods=MID)
        stencil(f"panel_name_{s}", "KF51", 0.22, (-1.00, side * (PANEL_Y + 0.003), 1.18),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


# ---------------------------------------------------------------- turret
def turret_rings():
    """The long faceted turret: its foot and roof narrow, its widest and
    longest at the chine, so the front plates meet in a nose over the slot."""
    foot = [(1.80, 0.40), (1.55, 1.40), (-2.60, 1.42), (-2.85, 1.10)]
    edge = [(2.62, 0.48), (1.85, 1.66), (-2.75, 1.66), (-3.00, 1.25)]
    roof = [(1.70, 0.42), (1.20, 1.32), (-2.60, 1.34), (-2.85, 1.00)]

    def ring(half):
        return half + [(x, -y) for x, y in reversed(half)]

    return [(FOOT, ring(foot)), (EDGE, ring(edge)), (EDGE + 0.12, ring([(x - 0.06, y) for x, y in edge])),
            (ROOF, ring(roof))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_rings(), mat=m["paint"], parent=turret, bevel=0.06)
    cyl("turret_ring_guard", 1.10, 0.10, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=40, lods=MID)
    # The slot the gun leaves through.
    box("gun_slot", (0.80, 0.44, 0.40), (2.10, 0, 0.45), m["black"], turret, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # Round smoke dischargers along the lower side face.
        for k in range(4):
            cyl(f"smoke_cup_{s}_{k}", 0.065, 0.12, (0.60 - k * 0.38, side * 1.58, 0.16), "Y", m["dark"], turret,
                seg=14, rot=(side * 0.6, 0, 0), lods=MID)
            cyl(f"smoke_cap_{s}_{k}", 0.05, 0.02, (0.60 - k * 0.38, side * 1.64, 0.12), "Y", m["black"], turret,
                seg=12, rot=(side * 0.6, 0, 0), lods=FINE)
        # The hatch on the flank's upper face, over the chine band (the
        # shell's side at x -0.70, from the band's top to the roof ring).
        loc, rot = VP.on_side(-0.70, EDGE + 0.34, side, (1.66, EDGE + 0.12), (1.33, ROOF))
        VP.bolted_panel(f"side_hatch_{s}", loc, (0.70, 0.46, 0.02), m, turret, bolts=(2, 2), bevel=0.008, rot=rot,
                        lods=MID)
        for k in range(2):
            cyl(f"warning_sensor_{s}_{k}", 0.04, 0.03, (1.40, side * (1.70 + 0.0), EDGE + 0.08 - k * 0.10), "Y",
                m["glass"], turret, seg=12, lods=NEAR)
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.30, side * 0.95, ROOF), m, whip, height=2.4)
    # Roof: the commander's tall sight box on the left, the gunner's at the
    # right front, a cupola, the HERO launcher box on the bustle.
    sight = empty("dressing_commander_sight", parent=turret)
    VP.sight_housing("commander_sight", (0.55, 0.72, ROOF - 0.02), m, sight, size=(0.42, 0.36, 0.38))
    VP.sight_housing("gunner_sight", (1.05, -0.80, ROOF - 0.10), m, turret, size=(0.50, 0.40, 0.22))
    VP.cupola("commander_cupola", (-0.30, 0.62, ROOF), m, turret, radius=0.34, lid_open=False)
    VP.hatch("loader_hatch", (-0.90, -0.70, ROOF), m, turret, radius=0.30)
    VP.bolted_panel("hero_launcher", (-2.10, 0.30, ROOF - 0.02), (0.90, 0.80, 0.12), m, turret, bolts=(3, 2),
                    bevel=0.03, lods=VP.ALL)
    for k in range(4):
        cyl(f"hero_cell_{k}", 0.08, 0.02, (-2.30 + (k % 2) * 0.40, 0.12 + (k // 2) * 0.36, ROOF + 0.11), "Z",
            m["black"], turret, seg=12, lods=NEAR)


# ---------------------------------------------------------------- weapons
def main_gun(v, gun):
    """The 130 mm L52 leaving its slot: a plain heavy barrel, a single sleeve
    band and the muzzle, no bore evacuator in the photos."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_cradle", (0.80, 0.40, 0.36), (0.40, 0, 0), m["dark"], gun, bevel=0.03, lods=MID)
    cyl("barrel_root", 0.17, 1.00, (1.20, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.015)
    cyl("barrel", 0.125, reach - 1.90, ((reach + 1.50) / 2, 0, 0), "X", m["paint"], gun, seg=24, r2=0.11)
    for k, x in enumerate((2.40, reach - 0.80)):
        cyl(f"barrel_band_{k}", 0.13, 0.06, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("muzzle_end", 0.12, 0.30, (reach - 0.15, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.068, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)


def weapon_station(v, hmg, hmg_gun):
    """The Natter remote weapon station: a low faceted body on its bearing,
    the sight block on its side and a heavy machine gun."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    cyl("rws_bearing", 0.24, 0.10, (0, 0, -0.03), "Z", m["dark"], hmg, seg=24, bevel=0.012)
    box("rws_body", (0.52, 0.40, lift - 0.04), (-0.04, 0, (lift - 0.04) / 2 + 0.02), m["paint"], hmg, bevel=0.04,
        taper=(0.8, 0.85))
    VP.sight_housing("rws_sight", (0.06, -0.28, -0.12), m, hmg_gun, size=(0.30, 0.16, 0.24))
    VP.browning_m2(hmg_gun, muzzle[0], m)


# ---------------------------------------------------------------- wreck
def wreck(variant, v):
    """The tank after its cook-off: the right track thrown flat beside it,
    two right road wheels gone, a side panel blown off and lying by the hull,
    another bent out, plates warped, the turret nose dented, the HERO box
    burst. `wreckage.burn` then heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_3_", "wheel_R_4_", "side_panel_R_2", "panel_bolt_R_2", "hero_cell_",
           "panel_name_R")
    thrown = solid("thrown_track", (4.2, TRACK_W, 0.05), (-0.2, -2.25, 0.03), m["track"], v.hull, rot=(0, 0, 0.06),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=5.0))
    solid("fallen_panel", (1.40, 0.12, 0.64), (0.4, -2.75, 0.07), m["paint"], v.hull, rot=(1.5, 0.0, 0.3), bevel=0.04)
    bend(parts("side_panel_L_3"), (-1.0, PANEL_Y, 1.50), (1, 0, 0), (0, 0, -1), 0.55)
    shell = parts("hull_upper", "hull_lower", "side_panel_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.024, 0.9, seed=3.0), dent((3.60, 0.50, 1.25), 0.5, 0.10, (-0.6, 0, -1)))
    for k, (loc, rot, size) in enumerate((((2.6, 2.25, 0.03), (0.04, 0.02, 0.5), 0.42),
                                          ((-3.0, 2.15, 0.03), (-0.03, 0.05, 1.9), 0.34),
                                          ((4.3, -0.6, 0.03), (0.0, 0.06, 2.4), 0.28))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=61 + k)
    rest_on_ground(0.004)


run("kf51", "german_three_tone", build, wreck, chip=1.0)
