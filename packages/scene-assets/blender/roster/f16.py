"""F-16C Block 50, from assets/references/f_16_fighting_falcon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f16.py -- [--wreck]

What the photos settle: the chin intake under the cockpit; the frameless
bubble canopy high on the nose; the cropped delta wing blended into the body
by long strakes; the single fin over a brake-chute fairing; two canted
ventral fins; all-moving tailplanes either side of the engine; wing-tip
AIM-120 rails, tanks on the inner wing pylons, missiles
outboard, the targeting and HARM pods on the intake's cheeks (Block 50),
the tall nose gear behind the intake and the main legs slanting out from
the duct to wheels under the wing roots.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Meshy's multi-image model, given the cleaned USAF three-quarter photo and
side, top and front renders of the lofted F-16 that was measured station by
station from the CC0 three-view (`f_16c_block_50-side-3.png`, `-top-2.png`,
`-front-2.png`), at one scale from its overall length (pitot tip to the fin
cap's trailing edge, 15.06 m). The skin matches those renders' outline to a
few centimetres, so the numbers below, measured on the drawing, still place
everything on it: the canopy, the gear, the stores, the pods and the
markings. The drawing stands 5.19 m from wheel to fin top (the gear at full
extension; Lockheed gives 5.09 m), the catalog 4.88 m, so heights are
pressed by 4% about the fin top (`Z`) and the gear is 0.1 m shorter.
"""
import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_loft import loft  # noqa: E402
from aircraft_skin import Probe  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR, tube_part  # noqa: E402

# Heights from the drawing, pressed about the fin top into the catalog's 4.88 m.
SQUASH = 0.96


def Z(z):
    return round(4.88 - SQUASH * (4.88 - z), 4)


WING_Z = Z(1.58)

# The canopy's plan, from the drawing's outline (x, half width): the
# windscreen's foot at 4.74 m, widest (0.44 m half) over 3.6, the bow where
# the glass ends at 2.42, the frame's opaque tail narrowing onto the spine
# by 1.35. Its heights are the skin's hump's (`aircraft_skin.fit_canopy`).
CANOPY_GLASS = [(4.74, 0.06), (4.5, 0.29), (4.3, 0.35), (4.1, 0.40), (3.9, 0.42), (3.6, 0.44), (3.3, 0.43),
                (3.0, 0.40), (2.7, 0.35), (2.42, 0.31)]
CANOPY_TAIL = [(2.42, 0.31), (2.1, 0.25), (1.8, 0.18), (1.5, 0.08), (1.35, 0.02)]
CANOPY_BOW = 2.42
# A main leg's links in to the duct's side from its top and its middle
# (gear-node frame, inboard negative): the leg slants out to the wheel. Its
# bay door hangs from the duct's floor inboard of it (x, y, length, drop).
MAIN_LEG = (((0, 0, 0.02), (0, -0.46, 0.64), 0.085), ((0, 0, -0.06), (-0.35, -0.42, 0.56), 0.04))
MAIN_DOOR = (-1.55, 0.6, 1.1, 0.42)
# The IFF interrogator's four blades on the nose ahead of the windscreen (span).
IFF = (0.1, 0.27)
SKIN = "assets/source/skins/f_16_fighting_falcon/f_16c_block_50.glb"
# The F110's nozzle from the engine bay's end (1.31 m across) to its exit
# (0.85 m) at -7.03, in place of the skin's own: its petals round the exit as
# a shell of bare-metal flaps converging aft (x, radius), creased between each.
NOZZLE_Z = Z(1.6)
NOZZLE = dict(loc=(-5.55, 0.0, NOZZLE_Z), r_front=0.64, r_exit=0.425, length=1.48, petals=15)
NOZZLE_CUT = ((-7.3, -5.7), (-0.6, 0.6), (NOZZLE_Z - 0.62, NOZZLE_Z + 0.6))
PETALS, PETAL_SHELL = 15, ((-6.05, 0.59), (-6.55, 0.525), (-7.04, 0.442))
SPEC = dict(
    # The generated skin, from the cleaned USAF photo and the lofted model
    # measured from the three-view (`skins/`): its wings, out from the
    # strakes' end, and its fin above the spine split off by name.
    # The canopy fitted to the skin's hump (the plan's widths, the hump's
    # heights), the skin a little low and wide on the measured outline: up
    # 0.15 m, its fin's tip held, and the tip rails in to the drawing's span.
    skin=dict(path=SKIN, length=15.06, top=4.88, lower=(-0.15, 2.95), span=(1.5, 4.9),
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(CANOPY_BOW,)),
              cut=[NOZZLE_CUT],
              parts=[("wing_L", ((-4.0, 1.5), (1.3, 6.0), (WING_Z - 0.45, WING_Z + 0.45))),
                     ("wing_R", ((-4.0, 1.5), (-6.0, -1.3), (WING_Z - 0.45, WING_Z + 0.45))),
                     ("tail_fin_0", ((-8.0, -3.0), (-0.3, 0.3), (Z(2.31) + 0.12, 9.0)))]),
    nozzles=[NOZZLE],
    # Wheelbase 4.0 m and track 2.36 m; the nose leg on the duct behind its
    # lip, its legs a tenth shorter (`Z`). The mains' legs stand out from the
    # duct's sides (`MAIN_LEG`), as the head-on photo shows them.
    gear=[dict(name="nose", x=2.28, y=0.0, top=Z(0.63), radius=0.24, width=0.15, light=True),
          dict(name="main_L", x=-1.7, y=1.18, top=Z(0.44), radius=0.35, width=0.22, rake=0.1),
          dict(name="main_R", x=-1.7, y=-1.18, top=Z(0.44), radius=0.35, width=0.22, rake=0.1)],
    # Stations 4/6 (tanks) and 3/7 (AIM-120s), where the drawings' pylon marks sit.
    pylons=[((-1.0, 1.95, Z(1.49)), 1.5, 0.3), ((-1.55, 3.15, Z(1.51)), 1.0, 0.22)],
    stores=[("missile", (-1.45, 3.15, Z(1.18)), 3.65, 0.09),
            ("missile", (-1.9, 4.84, WING_Z), 3.65, 0.09, False)],
    nav=dict(left=(2.45, 0.755, Z(1.1)), right=(2.45, -0.755, Z(1.1)), tail=(-6.48, 0.0, Z(2.5))),
)
# Shaw's 20th Fighter Wing, as the side photo: low-visibility star-and-bar on
# the fuselage and the wings (upper left, lower right), "SW" and the serial on the fin.
MARKINGS = [
    ("insignia", dict(kind="us_lowvis", centre=(-4.3, 1.0, Z(2.02)), normal=(0, 1, 0), up=(0, 0, 1), size=0.3,
                      onto=("fuselage",))),
    ("insignia", dict(kind="us_lowvis", centre=(-1.6, 3.1, WING_Z + 0.1), normal=(0, 0, 1), up=(1, 0, 0), size=0.5,
                      onto=("wing_L",), mirror=False)),
    ("insignia", dict(kind="us_lowvis", centre=(-1.6, -3.1, WING_Z - 0.1), normal=(0, 0, -1), up=(1, 0, 0),
                      size=0.5, onto=("wing_R",), mirror=False)),
    ("text", dict(text="SW", height=0.62, centre=(-6.05, 0.1, Z(3.78)), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("tail_fin_0",))),
    ("text", dict(text="AF 91 359", height=0.18, centre=(-5.6, 0.1, Z(3.2)), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("tail_fin_0",))),
    ("text", dict(text="USAF", height=0.3, centre=(-1.5, -3.3, WING_Z + 0.1), normal=(0, 0, 1), up=(-1, 0, 0),
                  onto=("wing_R",), mirror=False, colour="lowvis_dark")),
]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for side, s in ((1, "L"), (-1, "R")):
        leg = bpy.data.objects[f"gear_main_{s}"]
        for k, (a, b, r) in enumerate(MAIN_LEG):
            tube_part(f"gear_main_{s}_link_{k}_brace", lambda lod, a=a, b=b: [a, (b[0], side * b[1], b[2])], r,
                      m["gear"], leg, MID)
        dx, dy, length, drop = MAIN_DOOR
        top = shell.under(dx, side * dy) + 0.01
        box(f"gear_bay_door_{s}", (length, 0.025, drop), (dx, side * dy, top - drop / 2), m["paint"], hull,
            bevel=0.008, lods=MID)
    # The nose leg's taxi light on its bracket to the strut (as `gear` sizes them).
    nose = next(g for g in SPEC["gear"] if g["name"] == "nose")
    r_strut, drop = 0.06 + nose["radius"] * 0.12, nose["top"] - nose["radius"]
    box("gear_nose_light_arm", (0.07, 0.03, 0.03), (r_strut + 0.025, 0, -drop * 0.35), m["gear"],
        bpy.data.objects["gear_nose"], lods=NEAR)
    # The drop tanks on stations 4/6, smooth: the dozen sides a missile has would show as facets.
    A.mirrored(lambda side, s: A.store(f"store_tank_{s}", "tank", (0.2, side * 1.95, Z(0.835)), 4.8, 0.34, m, hull,
                                       seg=28))
    loft("nozzle_petals", [(x, [(r * math.sin(math.pi * k / PETALS) * (1 if k % 2 == 0 else 0.94),
                                 NOZZLE_Z + r * math.cos(math.pi * k / PETALS) * (1 if k % 2 == 0 else 0.94), k % 2 == 1)
                                for k in range(PETALS + 1)]) for x, r in PETAL_SHELL],
         m["steel"], hull, caps=False, outline_steps=(2, 1, 1, 1))
    for k, y in enumerate(IFF):
        A.mirrored(lambda side, s, y=y, k=k: A.blade_antenna(f"antenna_iff_{s}{k}", (5.15, side * y,
                                                                                      shell.over(5.15, side * y)),
                                                              0.06, m, hull, chord=0.07))
    # Block 50's HARM targeting pod and the Sniper pod on the intake's cheeks.
    for side, s, length in ((1, "L", 2.5), (-1, "R", 2.2)):
        box(f"cheek_pylon_{s}", (0.8, 0.1, 0.16), (2.25, side * 0.72, Z(0.87)), m["paint"], hull, bevel=0.02, lods=MID)
        A.store(f"cheek_pod_{s}", "pod", (2.25, side * 0.86, Z(0.69)), length, 0.17, m, hull)
    A.blade_antenna("antenna_spine", (-1.2, 0, shell.over(-1.2, 0)), 0.22, m, hull)
    A.blade_antenna("antenna_belly", (0.4, 0, shell.under(0.4, 0)), 0.22, m, hull, down=True)
    A.markings(v, MARKINGS)


def wreck(variant, v):
    # Its litter kept within the wreck's footprint (the frame `test_jet` borrows).
    A.crash(v, tail_x=-4.2, wing_y=2.8, wing_side=-1, seed=16, litter_reach=0.95)


run("f_16_fighting_falcon", "us_compass_grey", build, wreck, script="f16.py")
