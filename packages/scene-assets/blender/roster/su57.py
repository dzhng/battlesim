"""Su-57, from assets/references/su_57_felon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/su57.py -- [--wreck]

What the photos settle: a chined nose under a long canopy, the movable
leading-edge extensions (LEVCONs) ahead of the wing root, two engine
nacelles set wide apart under a flat body with a tunnel between them, their
intakes raked under the LEVCONs, a swept wing, stabilators, all-moving fins
canted outward, the long stinger between widely spaced nozzles. Splinter
greys.

The references' dimensions (published figures): length 20.1 m,
span 14.1 m, height 4.6 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the airframe alone from our own renders of the Su-57's
guide (`guides/su57.py`, measured from a three-view), both sides, top and
bottom. Its fins came out short, so with their tips at the frame's 4.62 m
its body stood 0.3 m high, and it is lowered (`lower`); its wings came out
0.9 m long each side, and `span` closes them to the frame's (pulling the
stabilators' tips in 0.4 m with them). The canopy is fitted to its hump
from the guide canopy's plan. The guide's IRST ball came out on both sides
of the nose, and the ghost to port is pressed flush. The intakes' dark
faces, the nozzles, the gear and the sensors are measured on the lowered
skin (the stations below) or seated on it (`Probe`).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

SKIN = "assets/source/skins/su_57_felon/eastern_su_57_felon_stealth_multirole_fighter.glb"
# The engines' axes on the lowered skin, where the nacelles end flat (0.7 m
# across, 0.5 m high, at -8.8) in the nozzles.
ENGINE_Y, ENGINE_Z = 1.27, 1.96
# The fins stand on the booms, canted outward 15 degrees on the skin (the
# guide's 27 came out halved): the star on the left one's outer face, aft
# where the fin is tallest (centre x, y, z; the face lies about 2.55 m out
# at 3.7 m up).
FIN_CANT, STAR = 0.26, (-7.1, 2.6, 3.7)
# The skin lowered 0.3 m below its canopy's top (3.95 m as generated), the
# fins above stretched to keep their tips at the frame's height.
LOWER = (0.3, 4.0)
# Its wingtips reach 8.0 m out, past the frame's 7.06: outboard of the
# LEVCONs' tips the wings close to it.
SPAN = (3.2, 7.06)
# The canopy's plan, the guide's (the glass ending on the spine at 4.2,
# highest 1.9 m back) with the windscreen's foot at 7.4 m, just behind the
# guide's arch, 0.06 m wider so its sill lies at the hump's foot rather
# than up its flank, its opaque tail as wide as the spine, onto it by 3.0; one bow at 6.0.
CANOPY_GLASS = framed_plan(x_front=7.4, x_back=4.2, half_width=0.62, peak=0.58, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (3.9, 0.45), (3.6, 0.3), (3.3, 0.12), (3.0, 0.02)]
# The guide's IRST ball, baked into the skin and mirrored to port: the
# port one pressed flush (the starboard one carries the ball).
GHOST = [(((7.55, 7.85), (0.15, 0.6), (2.86, 9.0)), 2.86)]
# The skin's nacelles end flat inside the nozzles: those ends go.
NOZZLE_CUT = [((-9.6, -8.0), tuple(sorted((s * 0.95, s * 1.59))), (1.72, 2.2)) for s in (1, -1)]
# Each intake's dark mouth on its raked face on the skin, inset from the
# lip (`aircraft_skin.intake_mouth` corners, y and z), upright where the
# skin closed it.
INTAKE_FACE = ((0.9, 1.6), (1.5, 1.6), (1.5, 1.92), (0.95, 1.85))
SPEC = dict(
    # The skin's wings (with their LEVCONs, outboard of the booms and below
    # the fins' roots) and its two fins split off by name.
    skin=dict(path=SKIN, length=20.0, top=4.62, lower=LOWER, span=SPAN, press=GHOST, cut=NOZZLE_CUT,
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(6.0,)),
              parts=[("wing_L", ((-5.3, 8.0), (2.75, 8.0), (1.75, 2.45))),
                     ("wing_R", ((-5.3, 8.0), (-8.0, -2.75), (1.75, 2.45))),
                     ("tail_fin_0", ((-9.5, -4.0), (1.6, 4.5), (2.95, 9.0))),
                     ("tail_fin_1", ((-9.5, -4.0), (-4.5, -1.6), (2.95, 9.0)))]),
    # The rear photo: the nozzles wide apart at the wing's height, the
    # stinger reaching well past them.
    nozzles=[dict(loc=(-7.95, y, ENGINE_Z), r_front=0.42, r_exit=0.38, length=0.95, petals=14)
             for y in (ENGINE_Y, -ENGINE_Y)],
)
# Gear (x, y, radius, width, extras), each leg's top seated under the skin.
GEAR = [("nose", 3.6, 0.0, 0.34, 0.18, dict(wheels=2, spread=0.3, door=(1.0, 0.5), light=True)),
        ("main_L", -2.4, 2.2, 0.5, 0.26, dict(door=(1.3, 0.6), rake=0.1)),
        ("main_R", -2.4, -2.2, 0.5, 0.26, dict(door=(1.3, 0.6), rake=0.1))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    A.nav_lights(m, hull, left=(-5.0, 6.95, shell.under(-5.0, 6.95) - 0.02),
                 right=(-5.0, -6.95, shell.under(-5.0, -6.95) - 0.02))
    # Each intake's dark face, just proud of the skin's closed one.
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, INTAKE_FACE))
    # The IRST ball ahead of the windscreen, offset to starboard.
    A.sensor_ball("irst", (7.7, -0.3, shell.over(7.7, -0.3) + 0.02), 0.14, m, hull)
    # The photo's markings: the red star on the fins' outer faces and under
    # the wings, the blue 054 on the nose's flat side under the canopy.
    fin_n = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT))
    fin_up = (0, math.sin(FIN_CANT), math.cos(FIN_CANT))
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=STAR, normal=fin_n, up=fin_up, size=0.3, onto=("tail_fin_",))),
        *[("insignia", dict(kind="ru_star", centre=(-3.4, y, 1.6), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)],
        ("text", dict(text="054", height=0.3, centre=(6.3, 1.0, 2.05), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])
    box("main_bay", (4.4, 1.2, 0.03), (-1.5, 0, shell.under(-1.5, 0) - 0.01), m["dark"], hull, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-4.6, wing_y=4.4, wing_side=1, tail_yaw=0.28, seed=57)


run("su_57_felon", "russian_air_grey", build, wreck, script="su57.py")
