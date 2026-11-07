"""MiG-29SMT, from assets/references/mig_29_fulcrum/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mig29.py -- [--wreck]

What the photos settle: the short drooping radome in its darker grey and
the long probe, the bubble canopy on the SMT's humped spine (its extra
fuel), the wide flat centre body with the engine nacelles slung apart under
it, a tunnel between them, their raked box intakes under the long curved
leading-edge extensions, the nacelles toeing in to nozzles that all but
touch, a swept wing, twin fins on tail booms outboard of the nozzles canted
a little outward, stabilators on the booms; twin nose wheels. Two greys.

The references' dimensions (MiG figures): length 17.32 m, span
11.36 m, height 4.73 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the lofted MiG-29 this script drew before, from its
shaded side, top and bottom views. It stands 0.27 m too high on its short
fins (lowered here, the fins' tips held) and spans 11.03 m (stretched out to
the hit box's 11.34 m; the MiG figure is 11.36). Its canopy is fitted to its
hump; the numbers below are measured on it: the dark mouths on its closed
intake faces and the nozzles in its nacelles' ends (its radome stays in the
airframe's grey: a split along the joint would cut a ragged line across
the skin). The gear, pylons and antennas sit where it is.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

SKIN = "assets/source/skins/mig_29_fulcrum/eastern_mig_29_fulcrum_mig_29smt.glb"
WING_Z = 1.72
# The canopy's plan (x, half width): the windscreen's foot where the hump
# rises off the nose, widest from the windscreen's arch back, as wide as the
# skin's hump to the glass's end, and its opaque tail on the SMT's humped
# spine narrowing to nothing; its heights are the hump's
# (`aircraft_skin.fit_canopy`), stations close enough to follow its crest.
CANOPY_GLASS = [(4.25, 0.1), (4.1, 0.3), (3.95, 0.42), (3.8, 0.48), (3.6, 0.5), (3.4, 0.5), (3.2, 0.5),
                (3.0, 0.5), (2.8, 0.5), (2.55, 0.5), (2.3, 0.5)]
CANOPY_TAIL = [(2.3, 0.5), (2.1, 0.48), (1.9, 0.45), (1.7, 0.38), (1.5, 0.3), (1.3, 0.16), (1.05, 0.02)]
# The skin's IRST ball, mirrored by its symmetric generation onto both
# sides of the nose: pressed flat onto the nose, and the one ball, right of
# the centreline, is built.
IRST_PRESS = ((4.15, 4.65), (-0.45, 0.45), (2.6, 3.2))
# Each intake's dark mouth within the lips of its closed, raked face on the
# skin (`aircraft_skin.intake_mouth` corners, y and z).
MOUTH = ((0.76, 0.86), (1.28, 0.86), (1.28, 1.66), (0.76, 1.66))
# The nozzles in place of the skin's nacelle ends (x of their front, centre
# y, z, radius), the skin cut away inside their radius.
NOZZLE_X, NOZZLE_Y, NOZZLE_Z, NOZZLE_R = -6.6, 1.0, 1.33, 0.42
NOZZLE_CUTS = [((-7.7, -6.65), tuple(sorted((side * (NOZZLE_Y - 0.36), side * (NOZZLE_Y + 0.36)))),
                (NOZZLE_Z - 0.36, NOZZLE_Z + 0.36)) for side in (1, -1)]
# The skin's wings span 11.03 m; outboard of `SPAN_FROM` it stretches to the
# hit box's half span (the MiG figure is 11.36 m), continuous at the root.
SPAN_FROM, TIP = 2.2, 5.67
# The fins on the skin: the red stars' centre on the left fin.
STAR = (-6.1, 1.68, 3.3)
SPEC = dict(
    # The skin's wings outboard of the nacelles' shelves, its fins above the
    # booms.
    skin=dict(path=SKIN, length=17.16, top=4.72, lower=(0.27, 3.7), span=(SPAN_FROM, TIP), cut=NOZZLE_CUTS, press=[IRST_PRESS],
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(3.95,)),
              parts=[("wing_L", ((-4.5, 3.0), (1.95, 6.5), (1.4, 2.4))),
                     ("wing_R", ((-4.5, 3.0), (-6.5, -1.95), (1.4, 2.4))),
                     ("tail_fin_0", ((-9.0, -4.0), (1.15, 3.0), (2.1, 6.0))),
                     ("tail_fin_1", ((-9.0, -4.0), (-3.0, -1.15), (2.1, 6.0)))]),
    nozzles=[dict(loc=(NOZZLE_X, y, NOZZLE_Z), r_front=NOZZLE_R, r_exit=NOZZLE_R - 0.04, length=0.85, petals=12)
             for y in (NOZZLE_Y, -NOZZLE_Y)],
)
# The nose leg in the tunnel between the intakes, the mains under the
# shelves outboard of the nacelles (x, y, radius, width).
GEAR = [dict(name="nose", x=2.1, y=0.0, radius=0.28, width=0.15, wheels=2, spread=0.24, light=True),
        dict(name="main_L", x=-1.4, y=1.55, radius=0.42, width=0.22, door=(1.1, 0.5), rake=0.1),
        dict(name="main_R", x=-1.4, y=-1.55, radius=0.42, width=0.22, door=(1.1, 0.5), rake=0.1)]
# Missiles on the inner and middle wing pylons (x, y, pylon length and
# depth, missile length and radius), the nacelles clear in the side photo.
STORES = [(-0.9, 3.0, 1.4, 0.28, 3.6, 0.11), (-1.6, 4.1, 1.2, 0.24, 2.9, 0.09)]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    for x, y, pl, depth, sl, r in STORES:
        def hang(side, k, x=x, y=y, pl=pl, depth=depth, sl=sl, r=r):
            z = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{abs(y):.0f}", (x, side * y, z + 0.02), pl, depth + 0.02, m, hull)
            A.store(f"store_{k}_{abs(y):.0f}", "missile", (x, side * y, z - depth - r - 0.02), sl, r, m, hull,
                    rail=True)
        A.mirrored(hang)
    # The wing-tip navigation lights, mid-chord on the tips' edges.
    tip_y = TIP - 0.12
    chord = shell.chord(tip_y, 0.0, -5.95)
    nav_x = (max(chord) + min(chord)) / 2
    A.nav_lights(m, hull, **{k: (nav_x, s * tip_y, (shell.under(nav_x, s * tip_y) + shell.over(nav_x, s * tip_y)) / 2)
                             for k, s in (("left", 1), ("right", -1))})
    # Each intake's dark mouth just ahead of its closed face on the skin.
    A.mirrored(lambda side, k: intake_mouth(f"intake_{k}_mouth", shell, m["black"], hull, side, MOUTH))
    body = Probe(hull, ("fuselage",))
    A.sensor_ball("irst", (4.4, -0.22, body.over(4.4, -0.22) + 0.06), 0.14, m, hull)
    # The anti-glare panel ahead of the windscreen, laid along the nose's slope.
    box("antiglare", (0.7, 0.3, 0.02), (5.3, 0, body.over(5.3, 0) - 0.004), m["dark"], hull, rot=(0, 0.24, 0),
        lods=MID)
    A.blade_antenna("antenna_spine", (-1.0, 0, body.over(-1.0, 0)), 0.22, m, hull)
    # The VKS's red star on the fins (both faces) and under the wings, the
    # photo's blue 777 on the nacelles behind the intakes.
    x, y, z = STAR
    rows = [("insignia", dict(kind="ru_star", centre=(x, yy, z), normal=(0, n, 0), up=(0, 0, 1), size=0.3,
                              onto=("tail_fin_",), mirror=False))
            for yy, n in ((y + 0.2, 1), (y - 0.4, -1), (-y - 0.2, -1), (-y + 0.4, 1))]
    rows += [("insignia", dict(kind="ru_star", centre=(-2.2, yy, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                               size=0.6, onto=("wing_",), mirror=False)) for yy in (3.6, -3.6)]
    rows.append(("text", dict(text="777", height=0.42, centre=(0.2, 1.6, 1.35), normal=(0, 1, 0), up=(0, 0, 1),
                              onto=("fuselage",), colour="blue")))
    A.markings(v, rows)


def wreck(variant, v):
    A.crash(v, tail_x=-4.0, wing_y=3.4, wing_side=-1, tail_yaw=-0.3, seed=29)


run("mig_29_fulcrum", "russian_air_grey", build, wreck, script="mig29.py")
