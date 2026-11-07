"""JAS 39 Gripen E, from assets/references/jas_39_gripen/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/gripen.py -- [--wreck]

What the photos settle: a small single-engine delta, the close-coupled
canards mounted on the side intakes just behind the cockpit, one swept fin,
the single nozzle; wing-tip missiles. The E (from the NG demonstrator and
published figures) keeps the C's layout on a longer body with its main gear
moved out into the wing roots. Light grey.

The references' dimensions (Saab figures): length 15.2 m, span
8.6 m over the tip missiles, height 4.5 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`), made
from shaded views of the measured lofted model in guides/gripen.py, which
it matches standing 0.09 m high (`lower`), its wings 0.8 m long a side
(`span`). Its canopy is fitted to its hump; the intakes' dark
mouths, the nozzle, gear, stores, lights and markings are built on it,
seated where it is (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/jas_39_gripen/europe_jas_39_gripen_e.glb"
WING_Z = 1.25
# The canopy's plan, the lofted model's framed canopy's outline, its opaque
# tail onto the spine; its heights are the skin's hump's.
CANOPY_GLASS = framed_plan(x_front=4.85, x_back=1.9, half_width=0.44, peak=0.48, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (1.5, 0.24), (1.1, 0.03)]
# Each side intake's dark mouth inside its lips on the skin's face
# (`aircraft_skin.intake_mouth` corners, y and z).
MOUTH = ((0.73, 1.1), (0.9, 1.1), (0.9, 1.62), (0.73, 1.62))
# The single nozzle in place of the skin's, the skin cut inside it.
NOZZLE = dict(loc=(-7.0, 0.0, 1.66), r_front=0.44, r_exit=0.4, length=0.6, petals=12)
NOZZLE_CUT = ((-7.9, -7.1), (-0.4, 0.4), (1.26, 2.06))
SPEC = dict(
    # The skin: its wings out from the intakes, its close-coupled canards
    # on the intakes above them, its fin above the spine.
    skin=dict(path=SKIN, length=15.2, top=4.5, lower=(0.09, 2.7), span=(2.0, 4.17), cut=[NOZZLE_CUT],
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(4.4,)),
              parts=[("wing_L", ((-6.0, 0.4), (1.15, 5.0), (0.8, 1.55))),
                     ("wing_R", ((-6.0, 0.4), (-5.0, -1.15), (0.8, 1.55))),
                     ("wing_canard_L", ((-0.3, 3.0), (1.05, 3.2), (1.58, 2.1))),
                     ("wing_canard_R", ((-0.3, 3.0), (-3.2, -1.05), (1.58, 2.1))),
                     ("tail_fin_0", ((-7.5, -3.0), (-0.3, 0.3), (2.4, 9.0)))]),
    nozzles=[NOZZLE],
)
GEAR = [dict(name="nose", x=3.7, y=0.0, radius=0.24, width=0.14, wheels=2, spread=0.2, light=True),
        dict(name="main_L", x=-2.0, y=1.25, radius=0.34, width=0.18, door=(0.9, 0.5)),
        dict(name="main_R", x=-2.0, y=-1.25, radius=0.34, width=0.18, door=(0.9, 0.5))]
# The wing pylons (x, y, length, depth) from the skin's underside and what
# hangs under each (kind, x offset, length, radius); the tips' missiles on
# their rails, measured at the tip's chord.
PYLONS = [((-1.6, 1.8, 1.0, 0.28), ("tank", -0.1, 3.6, 0.32)), ((-2.4, 2.8, 0.9, 0.22), ("missile", 0.0, 3.65, 0.09))]
TIP_X, TIP = -3.9, 4.1


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(PYLONS):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull, rail=kind == "missile")
    for side, s in ((1, "L"), (-1, "R")):
        z = shell.under(TIP_X, side * (TIP - 0.1))
        A.store(f"store_tip_{s}", "missile", (TIP_X, side * TIP, z - 0.09), 2.94, 0.065, m, hull)
    fin = Probe(hull, ("tail_fin",))
    A.nav_lights(m, hull, tail=(-6.4, 0.0, fin.over(-6.4, 0.0) - 0.1),
                 **{k: (TIP_X, side * (TIP - 0.3), shell.over(TIP_X, side * (TIP - 0.3)) + 0.01)
                    for k, side in (("left", 1), ("right", -1))})
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH))
    A.blade_antenna("antenna_spine", (-1.0, 0, shell.over(-1.0, 0)), 0.2, m, hull)
    # The Flygvapnet's roundel on the intakes and wings, the 270 of the photo on the fin.
    A.markings(v, [
        ("insignia", dict(kind="se", centre=(3.4, 1.2, 1.3), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="se", centre=(-3.0, 2.2, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.4,
                          onto=("wing_",))),
        ("text", dict(text="270", height=0.42, centre=(-5.7, 0.3, 3.2), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="lowvis_dark")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-2.8, wing_y=2.4, wing_side=-1, tail_yaw=-0.3, seed=39)


run("jas_39_gripen", "nato_air_grey", build, wreck, script="gripen.py")
