"""EBRC Jaguar armed reconnaissance vehicle (disabled card), from assets/references/jaguar/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/jaguar.py -- [--wreck]

What the photos settle (French Army, 2023-2026): a 6x6 on big black tyres,
the front axle well forward and the two rear axles close together behind a
long gap; a tall hull with flat, near-upright upper sides from the belt to a
flat roof (the roof about four fifths of the turret's height), a blunt nose
sloping down to headlights at the belt, stowage bags on the sides, a rear
door between the lights. The T40 turret is a long, wide faceted box behind
the cab, a third of the hull long, with the 40 mm CTA gun on the centre line
and the twin Akeron MP launcher box forming its left flank (it raises to
fire), the remote machine gun station on its roof, Galix smoke launchers on
its rear corners, the commander's sight mast. French three-tone paint, as on
the 2023 parade car.

Frame 7.1 x 2.99 x 2.8 m, turret on the roof at 1.95 m (`DIMENSIONS`, `MOUNTS`).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "europe_ebrc_jaguar_armed_reconnaissance_vehicle"
# The frame its references give (a disabled card has no unit type): the
# hull box, and the mounts its turret and guns are rigged on for the art.
DIMENSIONS = (7.1, 2.99, 2.8)
MOUNTS = [
    dict(name="autocannon", role="gun", on=None, pivot_m=[0.0, 0.0, 1.95], muzzle_m=[2.982, 0.0, 0.38]),
    dict(name="launcher", role="hmg", on="autocannon", pivot_m=[0.0, 1.28, 2.40], muzzle_m=[1.25, 0.0, 0.0]),
]
WHEEL_X = [2.15, -0.65, -2.25]
WHEEL_R = 0.66
WHEEL_W = 0.46
WHEEL_Y = 1.16
BELT = 1.45
ROOF = 1.95
TURRET_H = 0.68


def hull_rings():
    plan = VP.hull_plan
    return [(0.55, plan(-3.25, 3.05, 0.95, 0.20)),
            (0.95, plan(-3.45, 3.50, 1.36, 0.35)),
            (BELT, plan(-3.50, 3.55, 1.43, 0.40)),
            (ROOF, plan(-3.50, 2.95, 1.40, 0.35))]


def hull(v):
    m, h = v.mats, v.hull
    loft("jaguar_hull", hull_rings(), mat=m["paint"], parent=h, bevel=0.04)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, h,
                          rim_radius=0.34, ctis=True)
            box(f"suspension_arm_{s}_{k}", (0.18, 0.40, 0.12), (x, side * (WHEEL_Y - 0.36), WHEEL_R + 0.06),
                m["dark"], h, lods=NEAR)
        # Stowage bags on the upper sides, a light at the belt on each corner.
        for k, x in enumerate((1.0, -0.2, -1.4)):
            box(f"side_bag_{s}_{k}", (0.42, 0.14, 0.36), (x, side * 1.45, BELT + 0.22), m["canvas"], h, bevel=0.05,
                lods=MID)
        VP.light_with_guard(f"headlight_{s}", (3.52, side * 0.95, BELT - 0.15), 0.06, m, h)
        VP.light_with_guard(f"tail_light_{s}", (-3.51, side * 0.95, BELT + 0.15), 0.05, dict(m, lamp=m["tail"]), h,
                            rot=(0, 0, math.pi))
        VP.tow_hook(f"front_tow_{s}", (3.45, side * 0.55, 0.95), m, h, size=0.12)
        VP.mudflap(f"mudflap_{s}", (-3.00, side * WHEEL_Y, 1.05), (0.46, 0.45), m, h)
    VP.bolted_panel("rear_door", (-3.53, 0, 1.25), (0.05, 0.85, 1.00), m, h, bolts=(1, 3), bevel=0.015, lods=VP.ALL)
    VP.hatch("driver_hatch", (2.35, 0, ROOF), m, h, radius=0.26)
    for k in range(3):
        VP.periscope(f"driver_periscope_{k}", (3.00, -0.22 + k * 0.22, ROOF - 0.04), m, h, size=(0.11, 0.16, 0.08),
                     rot=(0, 0.69, 0))
    VP.grille("engine_grille", (-2.60, 0.0, ROOF), (1.10, 1.40), m, h, slats=9)
    VP.exhaust("exhaust", (-1.90, -1.44, BELT + 0.20), 0.07, 0.30, m, h, rot=(0, 0, -math.pi / 2))
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=h)
        VP.antenna(f"antenna_{s}", (-3.20, side * 0.80, ROOF), m, whip, height=2.4)
    # The photos' bolted armour: plates down the upright upper band from the
    # belt to the roof, and on the lower band leaning out under it; a tow
    # cable along the belt, a basket and jerrycans on the rear plate.
    upper = ((1.43, BELT), (1.40, ROOF))
    lower = ((1.36, 0.95), (1.43, BELT))
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate((1.75, 0.55, -0.65, -1.85)):
            loc, rot = VP.on_side(x, (BELT + ROOF) / 2, side, *upper)
            VP.bolted_panel(f"upper_plate_{s}_{k}", loc, (1.10, 0.40, 0.04), m, h, bolts=(3, 2), rot=rot, bevel=0.015,
                            lods=VP.ALL)
        for k, x in enumerate((1.35, -1.35)):
            loc, rot = VP.on_side(x, BELT - 0.20, side, *lower)
            VP.bolted_panel(f"lower_plate_{s}_{k}", loc, (1.30, 0.28, 0.03), m, h, bolts=(4, 1), rot=rot, bevel=0.01)
        VP.cable(f"tow_cable_{s}", [(-2.90, side * 1.44, BELT + 0.02), (-0.4, side * 1.46, BELT + 0.02),
                                    (2.10, side * 1.44, BELT + 0.02)], m, h, radius=0.018)
        VP.jerrycan(f"jerrycan_{s}", (-3.50, side * 0.62, 0.95), dict(m, paint=m["dark"]), h, rot=(0, 0, math.pi))
    VP.slat_armour("rear_basket", (-3.50, 0, 1.75), (0.60, 0.35), m, h, spacing=0.10, bar=0.014,
                   rot=(0, 0, math.pi / 2))
    # The rear tow hooks.
    for side, sd in ((1, "L"), (-1, "R")):
        VP.tow_hook(f"rear_tow_{sd}", (-3.42, side * 0.75, 0.85), v.mats, v.hull, size=0.13,
                    rot=(0, 0, math.pi))

def turret(v, t, gun, launcher, launcher_pitch):
    m = v.mats
    base = ROOF - v.frame["mounts"][0]["pivot_m"][2]
    top = base + TURRET_H
    # A tall, wide faceted box, nearly the hull's width with the Akeron pod
    # on its left.
    foot = [(1.25, 0.45), (0.95, 1.02), (-1.25, 1.04), (-1.45, 0.80), (-1.45, -0.80), (-1.25, -1.04), (0.95, -1.02),
            (1.25, -0.45)]
    crown = [(1.10, 0.40), (0.85, 0.96), (-1.20, 0.98), (-1.38, 0.75), (-1.38, -0.75), (-1.20, -0.98), (0.85, -0.96),
             (1.10, -0.40)]
    cyl("turret_ring", 1.05, 0.06, (0, 0, base + 0.03), "Z", m["dark"], t, seg=36, lods=MID)
    loft("t40_shell", [(base + 0.04, foot), (top, crown)], mat=m["paint"], parent=t, bevel=0.035)
    VP.roof_fittings("roof", crown, top, m, t, periscopes=((0.30, 0.50, 0.3),))
    VP.laser_warners("laser_warner", crown, top, m, t)
    VP.sight_housing("gunner_sight", (0.55, -0.60, top - 0.02), m, t, size=(0.40, 0.30, 0.28))
    mast = empty("dressing_commander_sight", parent=t)
    cyl("commander_mast", 0.08, 0.30, (-0.60, -0.55, top + 0.15), "Z", m["dark"], mast, seg=12)
    VP.sight_housing("commander_head", (-0.60, -0.55, top + 0.28), m, mast, size=(0.32, 0.28, 0.26))
    # The roof RWS on its pedestal, stacked above the turret as the photos
    # show (fixed here: it is not a frame mount).
    rws = empty("dressing_rws", parent=t)
    cyl("rws_base", 0.16, 0.18, (-0.20, 0.10, top + 0.09), "Z", m["dark"], rws, seg=18)
    box("rws_body", (0.42, 0.32, 0.22), (-0.20, 0.10, top + 0.29), m["paint"], rws, bevel=0.03)
    cyl("rws_barrel", 0.016, 0.55, (0.25, 0.10, top + 0.30), "X", m["dark"], rws, seg=8, lods=NEAR)
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(3):
            cyl(f"galix_{s}_{k}", 0.045, 0.22, (-1.32, side * (0.50 + k * 0.10), top - 0.10), "X", m["dark"], t,
                seg=10, rot=(0, -0.35, side * 2.6), lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("cta_mantlet", (0.40, 0.56, 0.44), (1.20, 0, 0), m["paint"], gun, bevel=0.04)
    cyl("cta_sleeve", 0.09, 0.60, (1.70, 0, 0), "X", m["paint"], gun, seg=16)
    cyl("cta_barrel", 0.045, reach - 2.00, ((reach + 2.00) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("cta_muzzle", 0.060, 0.20, (reach - 0.10, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("cta_bore", 0.028, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    # The twin Akeron MP launcher box on its arm: the turret's whole left
    # flank, standing proud of the turret roof and outboard of the hull side,
    # two tube mouths in front.
    lreach = v.frame["mounts"][1]["muzzle_m"][0]
    box("akeron_arm", (0.30, 0.10, 0.30), (-0.15, -0.28, -0.15), m["dark"], launcher, bevel=0.02)
    box("akeron_box", (lreach + 0.65, 0.48, TURRET_H + 0.17), ((lreach - 0.65) / 2, 0, 0), m["paint"], launcher_pitch,
        bevel=0.03)
    for k, y in enumerate((-0.12, 0.12)):
        cyl(f"akeron_mouth_{k}", 0.12, 0.01, (lreach + 0.005, y, 0), "X", m["black"], launcher_pitch, seg=14,
            lods=MID)

def build(variant, v):
    hull(v)
    mounts = rig(v.frame, v.root)
    t, gun, _, _ = mounts["autocannon"]
    launcher, launcher_pitch, _, _ = mounts["launcher"]
    turret(v, t, gun, launcher, launcher_pitch)


def wreck(variant, v):
    """The Jaguar after its fire: the front right wheel blown off and the
    nose settled onto that corner, the stowage bags burnt away, the rear door
    hanging open, the Akeron box blown off its arm, plates warped; the
    turret is thrown."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("wheel_R_1_", "side_bag_", "akeron_box", "akeron_mouth_", "mudflap_R")
    solid("fallen_akeron", (1.9, 0.48, 0.85), (1.4, -2.3, 0.36), m["paint"], v.hull, rot=(0.1, 0.05, 0.5),
          bevel=0.03)
    bend(parts("rear_door"), (-3.53, 0.43, 1.25), (0, 0, 1), (1, 0, 0), 1.0)
    shell = parts("jaguar_hull", "t40_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=41.0), dent((2.6, -1.4, 1.4), 0.5, 0.12, (0, 1, -0.2)))
    v.root.rotation_euler = (0.035, 0.04, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("jaguar", {CARD: DIMENSIONS}, "french_three_tone", build, wreck, mounts={CARD: MOUNTS},
                 skip=("dressing_", "gun", "hmg"), chip=0.6)
