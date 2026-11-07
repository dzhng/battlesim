"""MiG-31BM, from assets/references/mig_31_foxhound/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mig31.py -- [--wreck]

What the photos settle: a long heavy interceptor. A long radome, the tandem
two-seat canopy, two huge rectangular intakes along the forward fuselage, a
shoulder wing, twin fins canted a little outward over ventral fins,
stabilators, two big nozzles; offset twin-wheel main gear; four long-range
missiles under the body. Light grey.

The references' dimensions (MiG figures): length 22.69 m, span
13.46 m, height 6.15 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the airframe alone from our own renders of the
MiG-31BM's guide (`guides/mig31.py`, measured from a three-view), both
sides, top and bottom. The guide is the airframe without its 1.1 m pitot, so
it sits `GUIDE_X` aft of the frame's centre and the pitot is built. Its fins
came out short, standing its body 0.24 m high (`lower`); its wings came out
0.5 m long a side (`span`) and 0.4 m forward of the drawing's, which no
correction here moves. The canopy is fitted to its hump from the guide
canopy's plan. The intakes' dark mouths (on the skin's raked faces), the
nozzles (the skin's flat pipe ends cut away inside them), the gear, stores,
fences and antennas are measured on it or seated on it (`Probe`).
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

SKIN = "assets/source/skins/mig_31_foxhound/eastern_mig_31_foxhound_mig_31bm.glb"
WING_Z = 3.2
# The guide (10.3 to -11.28 m) sits this far aft of the frame's centre: its
# tail on the frame's, its nose 1.12 m short of it for the pitot.
GUIDE_X = -0.07
# The tandem canopy's plan: the guide canopy's outline with the
# windscreen's foot where the skin's hump starts, the glass ending on the
# spine, 0.08 m wider so its sill lies at the hump's foot, its opaque tail
# faired into the spine; bows at the windscreen and between the seats.
CANOPY_GLASS = framed_plan(x_front=6.1, x_back=2.5 + GUIDE_X, half_width=0.58, peak=0.43, tail=0.85)
CANOPY_TAIL = [CANOPY_GLASS[-1], (2.1, 0.35), (1.8, 0.2), (1.5, 0.05)]
# Each intake's dark mouth on the skin's raked face (`aircraft_skin.
# intake_mouth` corners, y and z, inside its lips).
MOUTH = ((0.92, 2.1), (1.6, 2.1), (1.6, 3.2), (0.92, 3.2))
# The rear view: two big D-30F6 pipes side by side, nearly touching, in
# place of the skin's one flat end, cut to their lips' radius in slabs 0.1 m
# high, so nothing of the skin stands inside a pipe and its ragged edge
# stays behind the pipe's wall.
NOZZLES = [dict(loc=(-10.2, y, 2.66), r_front=0.7, r_exit=0.68, length=1.15, petals=14) for y in (0.72, -0.72)]
NOZZLE_CUT = [((-11.6, -10.3), (y - w, y + w), (2.66 + dz - 0.05, 2.66 + dz + 0.05))
              for y in (0.72, -0.72) for dz in (0.1 * k for k in range(-5, 6))
              for w in [math.sqrt(0.69 ** 2 - (abs(dz) + 0.05) ** 2)]]
SPEC = dict(
    # The skin's wings and its two fins split off by name.
    skin=dict(path=SKIN, length=21.58, top=6.16, x=(10.3 - 11.28) / 2 + GUIDE_X, lower=(0.24, 4.35),
              span=(2.5, 6.73), cut=NOZZLE_CUT, canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL,
                                                             bows=(5.25 + GUIDE_X, 3.65 + GUIDE_X)),
              parts=[("wing_L", ((-7.8, 2.0), (1.95, 7.5), (2.85, 3.6))),
                     ("wing_R", ((-7.8, 2.0), (-7.5, -1.95), (2.85, 3.6))),
                     ("tail_fin_0", ((-11.5, -5.0), (0.8, 2.3), (3.45, 9.0))),
                     ("tail_fin_1", ((-11.5, -5.0), (-2.3, -0.8), (3.45, 9.0)))]),
    nozzles=NOZZLES,
)
# The nose gear under the rear seat, the mains' tandem bogies aft of the
# wing's root (the side photo): (x, y, radius, width, extras).
GEAR = [("nose", 3.68 + GUIDE_X, 0.0, 0.35, 0.18, dict(wheels=2, spread=0.3, door=(1.0, 0.6), light=True)),
        ("main_L", -3.3 + GUIDE_X, 1.85, 0.47, 0.26, dict(tandem=1.15, door=(1.6, 0.8))),
        ("main_R", -3.3 + GUIDE_X, -1.85, 0.47, 0.26, dict(tandem=1.15, door=(1.6, 0.8)))]
# The four R-37Ms under the body (x, y), and the wing tanks' pylons (span,
# length, depth) 40% back along the wing's chord there.
MISSILES = [(2.2, 0.8), (-3.2, 0.8)]
PYLON = (3.4, 1.2, 0.34)
# The wing fence on each wing's upper face, a third back along its chord.
FENCE_Y = 4.6


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    y, length, depth = PYLON
    chord = shell.chord(y, 2.0, -8.2)
    x = max(chord) - (max(chord) - min(chord)) * 0.4
    for side, s in ((1, "L"), (-1, "R")):
        top = shell.under(x, side * y)
        A.pylon(f"pylon_{s}", (x, side * y, top), length, depth, m, hull)
        A.store(f"store_tank_{s}", "tank", (x - 0.1, side * y, top - depth - 0.44), 4.6, 0.42, m, hull)
        for k, (mx, my) in enumerate(MISSILES):
            A.store(f"store_missile_{s}{k}", "missile", (mx, side * my, shell.under(mx, side * my) - 0.25), 4.2, 0.19,
                    m, hull)
        intake_mouth(f"intake_mouth_{s}", shell, m["black"], hull, side, MOUTH)
    # The wing-tip lights and the fences, on the skin's wing.
    chord = shell.chord(6.5, 2.0, -8.2)
    tip_x = (max(chord) + min(chord)) / 2
    A.nav_lights(m, hull, **{k: (tip_x, side * 6.5, shell.under(tip_x, side * 6.5) - 0.02)
                             for k, side in (("left", 1), ("right", -1))})
    fence = shell.chord(FENCE_Y, 2.0, -8.2)
    fence_x = max(fence) - (max(fence) - min(fence)) * 0.35
    A.mirrored(lambda side, k: box(f"wing_fence_{k}", (0.8, 0.04, 0.12),
                                   (fence_x, side * FENCE_Y, shell.over(fence_x, side * FENCE_Y) + 0.05), m["paint"],
                                   hull, lods=MID))
    # The pitot from the skin's nose tip to the frame's nose.
    tip_z = 2.5
    for z in (2.3 + 0.05 * k for k in range(9)):
        try:
            if shell.along(0.0, z) > shell.along(0.0, tip_z):
                tip_z = z
        except ValueError:
            pass
    tip_x = shell.along(0.0, tip_z) - 0.03
    A.pitot("nose_probe", (tip_x, 0.0, tip_z), 11.35 - tip_x, m, hull)
    A.blade_antenna("antenna_spine", (0.3 + GUIDE_X, 0, shell.over(0.3 + GUIDE_X, 0)), 0.25, m, hull)
    # The photo's markings: the red star and VVS ROSSII on the fins' outer
    # faces, the red star under the wings, the blue 86 on the nose.
    fin_x, fin_z = -9.0, 4.85
    under_wing = shell.chord(4.2, 2.0, -8.2)
    star_x = (max(under_wing) + min(under_wing)) / 2
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(fin_x, 1.95, fin_z), normal=(0, 1, 0), up=(0, 0, 1), size=0.5,
                          onto=("tail_fin_",))),
        ("text", dict(text="ВВС РОССИИ", height=0.18, centre=(fin_x + 0.45, 1.95, fin_z - 0.75), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("tail_fin_",), colour="lowvis_dark")),
        ("text", dict(text="RF 92369", height=0.13, centre=(fin_x + 0.45, 1.95, fin_z - 0.5), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("tail_fin_",), colour="lowvis_dark")),
        *[("insignia", dict(kind="ru_star", centre=(star_x, y, WING_Z - 0.4), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.6, onto=("wing_",), mirror=False)) for y in (4.2, -4.2)],
        ("text", dict(text="86", height=0.5, centre=(7.2, 1.2, 2.55), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-5.8, wing_y=4.2, wing_side=1, tail_yaw=0.25, seed=31)


run("mig_31_foxhound", "russian_air_grey", build, wreck, script="mig31.py")
