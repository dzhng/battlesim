"""J-20, from assets/references/j_20_mighty_dragon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/j20.py -- [--wreck]

What the photos settle: a long chined body, the canopy far forward, the
diverterless intakes either side of the cockpit, canards ahead of the
leading-edge extensions and the delta wing, all-moving fins canted outward,
two ventral fins, two nozzles; weapon bays under the body. Dark grey.

The references' dimensions: about 21.2 m long, 13.01 m span and
4.69 m high (published estimates; no official figures).

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the airframe alone from our own renders of the lofted
J-20 this script drew before (both sides, top and bottom). Its fins came
out short, standing its body 0.7 m high (`lower`), and its wings 0.35 m
short a side (`span`); the canopy is fitted to its hump from the lofted
canopy's plan. The intakes' dark mouths, the nozzles (the skin's flat pipe
ends cut away inside them), the gear, bays and lights are measured on it or
seated on it (`Probe`).
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

SKIN = "assets/source/skins/j_20_mighty_dragon/eastern_j_20_mighty_dragon_stealth_fighter.glb"
WING_Z = 2.0
# The fins, canted outward from the booms (their star's frame).
FIN_Y, FIN_Z, FIN_CANT = 1.1, 2.35, 0.4
# The canopy's plan: the lofted canopy's outline with the windscreen's foot
# where the skin's hump starts (6.9), the glass ending on the spine at 4.3,
# its opaque tail faired in by 3.3; the windscreen's bow at 6.35.
CANOPY_GLASS = framed_plan(x_front=6.9, x_back=4.3, half_width=0.52, peak=0.45, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (3.9, 0.24), (3.6, 0.1), (3.3, 0.02)]
# Each intake's dark mouth on the skin's flat face (x 4.8 to 5.0), inside its lips.
MOUTH = ((1.14, 1.5), (1.4, 1.5), (1.6, 2.1), (1.14, 2.1))
# The two nozzles in place of the skin's flat pipe ends (cut inside their radius).
NOZZLES = [dict(loc=(-8.8, y, 1.9), r_front=0.5, r_exit=0.46, length=1.62, petals=14) for y in (0.52, -0.52)]
NOZZLE_CUT = [((-10.6, -8.9), tuple(sorted((s * 0.08, s * 0.96))), (1.46, 2.34)) for s in (1, -1)]
SPEC = dict(
    # The skin's wings, canards (`wing_canard_*`, kept from the wreck's
    # fold) and canted fins split off by name.
    skin=dict(path=SKIN, length=20.82, top=4.58, lower=(0.7, 3.9), span=(2.0, 6.51), cut=NOZZLE_CUT,
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(6.35,)),
              parts=[("wing_canard_L", ((1.5, 4.8), (1.9, 4.5), (1.8, 2.7))),
                     ("wing_canard_R", ((1.5, 4.8), (-4.5, -1.9), (1.8, 2.7))),
                     ("wing_L", ((-10.0, 1.5), (2.2, 7.5), (1.4, 2.6))),
                     ("wing_R", ((-10.0, 1.5), (-7.5, -2.2), (1.4, 2.6))),
                     ("tail_fin_0", ((-10.5, -4.0), (0.9, 3.5), (2.45, 9.0))),
                     ("tail_fin_1", ((-10.5, -4.0), (-3.5, -0.9), (2.45, 9.0)))]),
    nozzles=NOZZLES,
)
# Gear (x, y, radius, width, extras), each leg's top seated under the skin.
GEAR = [("nose", 6.0, 0.0, 0.3, 0.17, dict(door=(1.0, 0.5), light=True)),
        ("main_L", -2.0, 1.6, 0.48, 0.24, dict(door=(1.3, 0.55), rake=0.1)),
        ("main_R", -2.0, -1.6, 0.48, 0.24, dict(door=(1.3, 0.55), rake=0.1))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH))
    # The wing-tip lights mid-chord on the tips.
    chord = shell.chord(6.3, 0.0, -10.45)
    tip_x = (max(chord) + min(chord)) / 2
    A.nav_lights(m, hull, **{k: (tip_x, side * 6.3, shell.over(tip_x, side * 6.3) + 0.01)
                             for k, side in (("left", 1), ("right", -1))})
    # The photo's markings: the PLA's star on the fins' outer faces, the
    # star and bars under the wings, the white 2011 on the nose.
    fin_n, fin_up = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT)), (0, math.sin(FIN_CANT), math.cos(FIN_CANT))
    A.markings(v, [
        ("insignia", dict(kind="cn_fin", centre=(-6.6, FIN_Y + 1.4 * fin_up[1] + 0.3, FIN_Z + 1.4 * fin_up[2]),
                          normal=fin_n, up=fin_up, size=0.38, onto=("tail_fin_0", "tail_fin_1"))),
        *[("insignia", dict(kind="cn_star", centre=(-5.0, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.7, onto=("wing_",), mirror=False)) for y in (4.0, -4.0)],
        ("text", dict(text="2011", height=0.34, centre=(6.6, 1.5, 1.9), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="white")),
    ])
    # The weapon bays' seams: the main bay under the body, a side bay each
    # side of the intakes.
    box("main_bay", (4.6, 1.4, 0.03), (0.6, 0, shell.under(0.6, 0) - 0.01), m["dark"], hull, lods=MID)
    A.mirrored(lambda side, k: box(f"side_bay_{k}", (1.8, 0.03, 0.35), (3.2, shell.flank(3.2, side, 1.6), 1.6),
                                   m["dark"], hull, lods=MID))


def wreck(variant, v):
    A.crash(v, tail_x=-5.4, wing_y=4.2, wing_side=-1, tail_yaw=-0.28, seed=20)


run("j_20_mighty_dragon", "chinese_air_grey", build, wreck, script="j20.py")
