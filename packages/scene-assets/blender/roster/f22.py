"""F-22A Raptor, from assets/references/f_22_raptor/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f22.py -- [--wreck]

What the photos settle: the chined, flat-sided nose under a frameless bubble
canopy, caret intakes raked back either side of the cockpit, the body widening
into a broad flat blended deck, the trapezoid wing (leading edge swept, the
trailing edge forward-swept inboard), twin fins canted outward, stabilators
level with the wing, two-dimensional vectoring nozzles side by side between
the tail booms, the tall gear. Compass greys.

The references' dimensions (USAF fact sheet figures agree):
length 18.92 m, span 13.56 m, height 5.08 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D from our own shaded views of the guide (`guides/f22.py`, the
lofted F-22 measured against the USAF's public-domain three-view: the wing
0.6 m further aft, thin canted fins, the intake trunks without box cowls),
one smooth surface on that outline, 0.25 m high there and its wings 0.55 m
short of the span, both corrected. The
frameless canopy is fitted to its hump; the skin's own vectoring nozzles
get dark bores; the intakes' mouths, the bays' seams, the gear and the
lights sit on the skin (`Probe`).
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
from vehicle_parts import MID, NEAR  # noqa: E402


# The wing's mean height on the skin, for the insignia laid on it.
WING_Z = 2.24
# The fins' root and cant, for the lettering laid on them.
FIN_Y, FIN_Z, FIN_CANT = 1.25, 2.55, 0.49
# The frameless canopy's plan (the lofted model's outline) and its tail onto the spine.
GLASS = framed_plan(6.75, 3.75, 0.54, peak=0.45, tail=0.7)
# The caret intakes' mouths within the lips of the skin's raked faces
# (`aircraft_skin.intake_mouth` corners, y and z), measured from ahead.
MOUTH = ((1.22, 1.38), (1.82, 1.38), (1.85, 2.42), (1.22, 2.42))
# The vectoring nozzles are the skin's own square pipes between their
# flaps: each exit's dark bore sits in its recess (x, centre y and z).
BORE_X, BORE_Y, BORE_Z = -8.285, 0.6, 1.8
SPEC = dict(
    skin=dict(path="assets/source/skins/f_22_raptor/us_f_22_raptor_f_22a.glb", length=18.92, top=5.12,
              lower=(0.25, 3.75), span=(2.2, 6.78),
              canopy=dict(glass=GLASS, tail=[GLASS[-1], (3.3, 0.3), (2.9, 0.03)]),
              parts=[("wing_L", ((-5.5, 3.5), (2.3, 8.0), (1.95, 2.6))),
                     ("wing_R", ((-5.5, 3.5), (-8.0, -2.3), (1.95, 2.6))),
                     ("tail_fin_0", ((-8.5, -3.5), (0.9, 3.4), (2.75, 9.0))),
                     ("tail_fin_1", ((-8.5, -3.5), (-3.4, -0.9), (2.75, 9.0)))]),
)
# The nose gear under the canopy's back, the mains 2.6 m behind the middle;
# each leg's top on the skin.
GEAR = [dict(name="nose", x=3.6, y=0.0, radius=0.32, width=0.18, door=(1.0, 0.5), light=True),
        dict(name="main_L", x=-2.6, y=1.62, radius=0.48, width=0.24, door=(1.4, 0.6), rake=0.1),
        dict(name="main_R", x=-2.6, y=-1.62, radius=0.48, width=0.24, door=(1.4, 0.6), rake=0.1)]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH))
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    A.nav_lights(m, hull, **{k: (-3.3, side * 6.4, shell.over(-3.3, side * 6.4) + 0.01)
                             for k, side in (("left", 1), ("right", -1))})
    for side, s in ((1, "L"), (-1, "R")):
        box(f"nozzle_{s}_bore", (0.03, 0.6, 0.38), (BORE_X, side * BORE_Y, BORE_Z), m["black"], hull, lods=MID)
        # The side weapon bays' door seams under the intakes.
        box(f"side_bay_{s}", (1.8, 0.03, 0.4), (1.9, shell.flank(1.9, side, 1.6) - side * 0.01, 1.6), m["dark"],
            hull, lods=NEAR)
    box("main_bay", (4.2, 1.6, 0.03), (0.4, 0, shell.under(0.4, 0) - 0.01), m["dark"], hull, lods=MID)
    box("gun_door", (0.6, 0.06, 0.12), (3.6, shell.flank(3.6, -1, 2.4) + 0.02, 2.4), m["dark"], hull, lods=NEAR)
    # Langley's FF on the fins, the low-visibility insignia on the intakes and wings.
    fin_n, fin_up = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT)), (0, math.sin(FIN_CANT), math.cos(FIN_CANT))
    A.markings(v, [
        ("insignia", dict(kind="us_lowvis", centre=(2.6, 2.3, 1.75), normal=(0, 1, 0), up=(0, 0, 1), size=0.36,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.6, 4.6, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.6, -4.6, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.55, onto=("wing_R",), mirror=False)),
        ("text", dict(text="FF", height=0.6, centre=(-6.0, FIN_Y + 1.4 * fin_up[1] + 0.3, FIN_Z + 1.4 * fin_up[2]),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",))),
        ("text", dict(text="AF 05 4085", height=0.14, centre=(-6.0, FIN_Y + 0.75 * fin_up[1] + 0.3,
                                                              FIN_Z + 0.75 * fin_up[2]),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",))),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-5.2, wing_y=4.2, wing_side=1, tail_yaw=0.3, seed=22)


run("f_22_raptor", "us_compass_grey", build, wreck, script="f22.py")
