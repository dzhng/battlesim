"""JAS 39 Gripen E, from assets/references/jas_39_gripen/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/gripen.py -- [--wreck]

What the photos settle: a small single-engine delta, the close-coupled
canards mounted on the side intakes just behind the cockpit, one swept fin,
the single nozzle; wing-tip missiles. The E (from the NG demonstrator and
published figures) keeps the C's layout on a longer body with its main gear
moved out into the wing roots. Light grey.

Dimensions stated from the references (Saab figures): length 15.2 m, span
8.6 m over the tip missiles, height 4.5 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"europe_jas_39_gripen_e": (15.2, 8.6, 4.5)}
WING_Z = 1.6
SPEC = dict(
    fuselage=[(7.6, 0.0, 1.75, 1.75), (6.9, 0.34, 1.5, 1.98, 1.75), (5.8, 0.5, 1.32, 2.2, 1.72, 2.2),
              (4.3, 0.56, 1.25, 2.32, 1.7, 2.4), (2.6, 0.62, 1.22, 2.3, 1.68, 2.6), (0.0, 0.7, 1.22, 2.25, 1.68, 2.8),
              (-3.0, 0.66, 1.3, 2.15, 1.7, 2.6), (-5.6, 0.52, 1.42, 2.0, 1.72, 2.2), (-6.6, 0.46, 1.48, 1.95, 1.72)],
    bodies=[(f"fuselage_intake_{s}", [(3.65, 0.3, 1.24, 2.0, 1.62, 3.0), (1.5, 0.32, 1.24, 2.02, 1.62, 3.0),
                                       (-0.4, 0.0, 1.62, 1.62)]) for s in "LR"],
    canopy=dict(x_front=5.6, x_back=3.5, sill=2.18, top=2.72, half_width=0.42, peak=0.45),
    canard=[(3.6, 0.95, 2.02, 1.5, 0.08), (2.45, 2.6, 2.02, 0.7, 0.03)],
    wing=[(1.2, 0.7, WING_Z, 5.0, 0.24), (-3.0, 4.06, WING_Z, 1.0, 0.04)],
    fins=[dict(root_x=-2.6, root_z=2.15, height=2.35, root_chord=3.0, tip_chord=1.0, sweep_m=2.1)],
    intakes=[("rect", (3.68, 0.8, 1.62), 0.42, 0.66, 0.2), ("rect", (3.68, -0.8, 1.62), 0.42, 0.66, 0.2)],
    nozzles=[dict(loc=(-6.6, 0.0, 1.72), r_front=0.46, r_exit=0.42, length=0.95, petals=12)],
    gear=[dict(name="nose", x=5.0, y=0.0, top=1.25, radius=0.24, width=0.14, wheels=2, spread=0.2),
          dict(name="main_L", x=-0.6, y=1.25, top=1.25, radius=0.34, width=0.18, door=(0.9, 0.5)),
          dict(name="main_R", x=-0.6, y=-1.25, top=1.25, radius=0.34, width=0.18, door=(0.9, 0.5))],
    pylons=[((-0.6, 1.8, WING_Z - 0.06), 1.0, 0.28), ((-1.2, 2.8, WING_Z - 0.04), 0.9, 0.22)],
    stores=[("tank", (-0.7, 1.8, 0.98), 3.6, 0.32), ("missile", (-1.2, 2.8, 1.24), 3.65, 0.09),
            ("missile", (-2.1, 4.16, WING_Z), 2.94, 0.065)],
)


def build(variant, v):
    s = dict(SPEC)
    bodies = s.pop("bodies")
    m = A.jet(v, s)
    hull = v.hull
    for name, stations in bodies:
        A.body(name, stations, m["paint"], hull, seg=14, loc=(0, 0.8 if name.endswith("L") else -0.8, 0))
    A.mirrored(lambda side, k: box(f"wing_tip_rail_{k}", (2.0, 0.07, 0.08), (-2.2, side * 4.1, WING_Z - 0.02),
                                   m["dark"], hull))
    box("antiglare", (0.9, 0.5, 0.03), (6.3, 0, 2.1), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    box("probe_fairing", (1.0, 0.1, 0.1), (4.6, -0.5, 2.1), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (0.0, 0, 2.25), 0.2, m, hull)


def wreck(variant, v):
    A.crash(v, tail_x=-2.8, wing_y=2.4, wing_side=-1, tail_yaw=-0.3, seed=39)


run_disabled("gripen", CARDS, "nato_air_grey", build, wreck)
