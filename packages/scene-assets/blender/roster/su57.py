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
Hunyuan3D's model of the airframe alone from our own renders of the lofted
Su-57 this script drew before, both sides, top and bottom (every photo of it
in the references is share-alike, so none went in). Its plan matches the
lofted one to 0.2 m with its wings a little short, which `span` closes to
the frame's; its fins stand short of the lofted ones, so with their tips at
the frame's 4.62 m its body stood 0.6 m high, and it is lowered (`lower`)
until the canopy's top is at the side photo's 3.25 m. The canopy is fitted
to its hump from the lofted canopy's plan; the old windscreen's arch the
renders baked into it is pressed flat. The intakes' dark faces, the
nozzles, the gear and the sensors are measured on the lowered skin (the
stations below) or seated on it (`Probe`).
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
# The engines' axes on the lowered skin, where the nacelles end (0.4 m
# round, at -8.46) in the nozzles.
ENGINE_Y, ENGINE_Z = 1.5, 1.75
# The fins stand on the booms, canted outward: the star on the left one's
# outer face, aft where the fin is tallest (centre x, y, z; the face lies
# about 2.55 m out at 3.4 m up).
FIN_CANT, STAR = 0.45, (-7.5, 2.85, 3.4)
# The skin lowered 0.6 m below its canopy's top (3.86 m as generated), the
# fins above stretched to keep their tips at the frame's height.
LOWER = (0.6, 3.95)
# Its wingtips reach 6.5 m out, short of the frame's 7.06: outboard of the
# booms the wings stretch to it.
SPAN = (3.0, 7.06)
# The canopy's plan, the lofted canopy's (the glass ending on the spine at
# 4.3, highest past its middle) with the windscreen's foot at 7.3 m, just
# behind the old arch, its opaque tail onto the spine by 3.2; one bow at 6.75.
CANOPY_GLASS = framed_plan(x_front=7.3, x_back=4.3, half_width=0.55, peak=0.55, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (3.9, 0.28), (3.5, 0.12), (3.2, 0.02)]
# The lofted canopy's windscreen arch and its bow, baked into the skin
# ahead of the glass and through it: pressed flat.
ARCH = [((7.28, 7.62), (-0.42, 0.42), (2.43, 9.0)), ((6.45, 6.9), (-0.32, 0.32), (2.6, 9.0))]
# The skin's nacelles end flat inside the nozzles: those ends go.
NOZZLE_CUT = [((-9.6, -8.0), tuple(sorted((s * 1.12, s * 1.88))), (1.37, 2.13)) for s in (1, -1)]
# Each intake's dark mouth on its raked face on the skin, inset from the
# lip (`aircraft_skin.intake_mouth` corners, y and z), the top swept further
# out than the foot.
INTAKE_FACE = ((0.95, 1.14), (1.85, 1.14), (2.08, 2.02), (1.12, 2.02))
SPEC = dict(
    # The skin's wings (with their LEVCONs, outboard of the booms and below
    # the fins' roots) and its two fins split off by name.
    skin=dict(path=SKIN, length=20.0, top=4.62, lower=LOWER, span=SPAN, press=ARCH, cut=NOZZLE_CUT,
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(6.75,)),
              parts=[("wing_L", ((-4.2, 8.0), (2.75, 8.0), (1.6, 2.65))),
                     ("wing_R", ((-4.2, 8.0), (-8.0, -2.75), (1.6, 2.65))),
                     ("tail_fin_0", ((-9.5, -3.5), (1.6, 4.5), (2.65, 9.0))),
                     ("tail_fin_1", ((-9.5, -3.5), (-4.5, -1.6), (2.65, 9.0)))]),
    # The rear photo: the nozzles wide apart at the wing's height, the
    # stinger reaching well past them.
    nozzles=[dict(loc=(-7.95, y, ENGINE_Z), r_front=0.44, r_exit=0.4, length=0.95, petals=14)
             for y in (ENGINE_Y, -ENGINE_Y)],
)
# Gear (x, y, radius, width, extras), each leg's top seated under the skin.
GEAR = [("nose", 3.6, 0.0, 0.34, 0.18, dict(wheels=2, spread=0.3, door=(1.0, 0.5), light=True)),
        ("main_L", -2.6, 2.12, 0.5, 0.26, dict(door=(1.3, 0.6), rake=0.1)),
        ("main_R", -2.6, -2.12, 0.5, 0.26, dict(door=(1.3, 0.6), rake=0.1))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    A.nav_lights(m, hull, left=(-2.8, 6.95, shell.under(-2.8, 6.95) - 0.02),
                 right=(-2.8, -6.95, shell.under(-2.8, -6.95) - 0.02))
    # Each intake's dark face, just proud of the skin's closed one.
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, INTAKE_FACE))
    # The IRST ball ahead of the windscreen, offset to starboard.
    A.sensor_ball("irst", (7.75, -0.3, shell.over(7.75, -0.3) + 0.06), 0.14, m, hull)
    # The photo's markings: the red star on the fins' outer faces and under
    # the wings, the blue 054 on the nose's flat side under the canopy.
    fin_n = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT))
    fin_up = (0, math.sin(FIN_CANT), math.cos(FIN_CANT))
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=STAR, normal=fin_n, up=fin_up, size=0.3, onto=("tail_fin_",))),
        *[("insignia", dict(kind="ru_star", centre=(-0.5, y, 1.9), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)],
        ("text", dict(text="054", height=0.3, centre=(6.3, 1.0, 2.05), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])
    box("main_bay", (4.4, 1.2, 0.03), (0.5, 0, shell.under(0.5, 0) - 0.01), m["dark"], hull, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-5.4, wing_y=4.4, wing_side=1, tail_yaw=0.28, seed=57)


run("su_57_felon", "russian_air_grey", build, wreck, script="su57.py")
