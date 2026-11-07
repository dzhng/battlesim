"""Eurofighter Typhoon (Tranche 3), from assets/references/eurofighter_typhoon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/typhoon.py -- [--wreck]

What the photos settle: the long pointed nose, canards just behind and below
the cockpit, the big rectangular chin intake under the forward body with its
variable lip, the cranked delta wing with no tailplane, one swept fin, two
EJ200 nozzles side by side; wing-tip defensive-aids pods, tanks and missiles
under the wing, missiles semi-recessed under the body. Light grey.

The references' dimensions (Eurofighter figures agree): length
15.96 m, span 10.95 m, height 5.28 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

# Measured on the public-domain three-view (Inductiveload), its side view
# scaled by the length and stood on the fin's tip at 5.28 m: the nose's tip
# 1.77 m up and drooping, the canopy's peak 3.3 m over 3.5, the spine
# falling from 2.8 to 2.5 m at the fin, the belly low (0.85 m under the
# wing), the chin intake's floor 0.8 m with its lip raked back from 3.3 m,
# the nozzles' centres 1.65 m up.
WING_Z = 1.3
SPEC = dict(
    fuselage=[(7.98, 0.0, 1.77, 1.77), (7.2, 0.38, 1.6, 2.03, 1.8), (6.0, 0.58, 1.49, 2.4, 1.9, 2.2),
              (4.4, 0.66, 1.5, 2.55, 2.0, 2.4), (2.6, 0.76, 1.45, 2.75, 2.1, 2.6), (0.0, 0.88, 1.3, 2.75, 2.0, 2.8),
              (-3.0, 0.88, 1.15, 2.55, 1.85, 2.8), (-5.6, 0.82, 1.05, 2.45, 1.75, 2.6), (-6.4, 0.76, 1.1, 2.4, 1.75)],
    # The chin intake's trunk, wide and deep under the forward body and
    # faired into the belly behind the wing's root.
    bodies=[("fuselage_intake", [(3.3, 0.7, 0.8, 1.6, 1.2, 4.0), (2.0, 0.74, 0.79, 1.65, 1.22, 4.0),
                                 (-0.1, 0.7, 0.85, 1.7, 1.27, 3.0), (-2.3, 0.55, 0.95, 1.7, 1.32, 2.4),
                                 (-4.0, 0.0, 1.15, 1.15)], 0.0, 16),
            ("fuselage_spine", [(2.1, 0.4, 2.5, 2.95, 2.6, 2.4), (-0.5, 0.42, 2.4, 2.75, 2.5, 2.6),
                                (-4.0, 0.0, 2.5, 2.5)])],
    canopy=dict(x_front=5.4, x_back=2.0, sill=2.62, top=3.3, half_width=0.5, bows=(4.85,), peak=0.42, tail=0.75),
    canard=[(5.1, 0.62, 1.95, 1.45, 0.08), (3.95, 1.85, 1.93, 0.45, 0.03)],
    wing=[(1.6, 0.86, WING_Z, 6.6, 0.3), (-3.7, 5.32, WING_Z, 1.3, 0.05)],
    wing_hinge=((0.82, 0, 1),),
    fins=[dict(root_x=-3.3, root_z=2.55, height=2.73, root_chord=3.4, tip_chord=0.93, sweep_m=3.6)],
    intakes=[("rect", (3.33, 0.0, 1.2), 1.3, 0.72, 0.22, (0, -0.1, 0))],
    nozzles=[dict(loc=(-6.4, 0.44, 1.65), r_front=0.45, r_exit=0.42, length=0.45, petals=12),
             dict(loc=(-6.4, -0.44, 1.65), r_front=0.45, r_exit=0.42, length=0.45, petals=12)],
    gear=[dict(name="nose", x=2.7, y=0.0, top=0.82, radius=0.26, width=0.16, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-1.5, y=1.2, top=1.0, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-1.5, y=-1.2, top=1.0, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-1.7, 2.0, WING_Z - 0.06), 1.1, 0.3), ((-2.5, 3.3, WING_Z - 0.04), 1.0, 0.24)],
    stores=[("tank", (-1.8, 2.0, WING_Z - 0.68), 4.0, 0.36), ("missile", (-2.5, 3.3, WING_Z - 0.4), 3.65, 0.09),
            ("pod", (-3.7, 5.42, WING_Z), 1.6, 0.12), ("missile", (0.4, 0.55, 0.72), 3.65, 0.09, False),
            ("missile", (-2.6, 0.55, 0.88), 3.65, 0.09, False)],
    nav=dict(left=(-3.9, 5.2, WING_Z + 0.04), right=(-3.9, -5.2, WING_Z + 0.04), tail=(-7.8, 0.0, 5.0)),
)
# The RAF's low-visibility roundels and fin flash, as the side photo.
MARKINGS = [
    ("insignia", dict(kind="uk_lowvis", centre=(1.4, 1.2, 2.05), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("fuselage",))),
    ("insignia", dict(kind="uk_flash", centre=(-5.9, 0.3, 3.45), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("tail_fin_0",))),
    ("insignia", dict(kind="uk_lowvis", centre=(-3.6, 3.4, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.45,
                      onto=("wing_",))),
    ("text", dict(text="ZK353", height=0.2, centre=(-4.2, 1.2, 1.8), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("fuselage",))),
]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("intake_splitter", (0.5, 0.04, 0.72), (3.15, 0, 1.2), m["paint"], hull, lods=MID)
    box("antiglare", (1.1, 0.55, 0.03), (6.0, 0, 2.42), m["dark"], hull, rot=(0, 0.12, 0), lods=MID)
    box("probe_fairing", (1.2, 0.12, 0.12), (5.0, -0.56, 2.4), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (-1.2, 0, 2.62), 0.22, m, hull)
    A.markings(v, MARKINGS)


def wreck(variant, v):
    import bpy
    A.crash(v, tail_x=-3.6, wing_y=3.2, wing_side=1, tail_yaw=0.28, seed=3)
    # The crash lays its litter by the airframe's span; the unit type's
    # frame is a little wider than the wreck's box allows, so the far
    # plate lies a little nearer (it sat 7 cm past the box).
    for o in bpy.data.objects:
        if o.name.startswith("litter_1_") and o.parent == v.root:
            o.location.y *= 0.95


run("eurofighter_typhoon", "nato_air_grey", build, wreck, script="typhoon.py")
