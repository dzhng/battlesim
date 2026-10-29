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

# The side photo (an RAF jet taxiing): the nose high (its tip 2.5 m up),
# the canopy's top 3.7 m, the spine falling to 2.8 m at the fin; the chin
# intake's lip 4.6 m behind the nose, 1.35-1.95 m up; the two nozzles low
# under the fin's root (centres 1.8 m up), ending 1.1 m short of the fin's
# tip; the delta's trailing edge across the nozzles; both gear further aft.
WING_Z = 1.9
SPEC = dict(
    fuselage=[(7.98, 0.0, 2.5, 2.5), (7.2, 0.38, 2.22, 2.78, 2.5), (6.0, 0.58, 2.05, 3.02, 2.52, 2.2),
              (4.4, 0.66, 1.98, 3.1, 2.52, 2.4), (2.6, 0.76, 1.85, 3.12, 2.45, 2.6), (0.0, 0.88, 1.7, 3.0, 2.3, 2.8),
              (-3.0, 0.88, 1.62, 2.9, 2.2, 2.8), (-5.6, 0.82, 1.4, 2.75, 2.0, 2.6), (-6.4, 0.76, 1.32, 2.62, 1.9)],
    # The chin intake's trunk, wide and deep under the forward body and
    # faired into the belly behind the wing's root.
    bodies=[("fuselage_intake", [(3.35, 0.7, 1.3, 2.2, 1.7, 4.0), (2.0, 0.74, 1.32, 2.24, 1.75, 4.0),
                                 (-0.1, 0.7, 1.46, 2.26, 1.85, 3.0), (-2.3, 0.55, 1.62, 2.2, 1.9, 2.4),
                                 (-4.0, 0.0, 1.8, 1.8)], 0.0, 16),
            ("fuselage_spine", [(2.1, 0.4, 2.95, 3.38, 3.05, 2.4), (-0.5, 0.42, 2.75, 3.04, 2.85, 2.6),
                                (-4.0, 0.0, 2.8, 2.8)])],
    canopy=dict(x_front=5.5, x_back=2.0, sill=3.02, top=3.72, half_width=0.5, bows=(4.9,), peak=0.42, tail=0.75),
    canard=[(5.3, 0.62, 2.42, 1.3, 0.08), (4.45, 2.1, 2.4, 0.55, 0.03)],
    wing=[(0.9, 0.86, WING_Z, 6.4, 0.3), (-4.5, 5.32, WING_Z, 1.3, 0.05)],
    wing_hinge=((0.82, 0, 1),),
    fins=[dict(root_x=-4.0, root_z=2.95, height=2.33, root_chord=2.7, tip_chord=0.93, sweep_m=3.05)],
    intakes=[("rect", (3.38, 0.0, 1.68), 1.3, 0.62, 0.22, (0, -0.1, 0))],
    nozzles=[dict(loc=(-6.4, 0.44, 1.8), r_front=0.45, r_exit=0.42, length=0.45, petals=12),
             dict(loc=(-6.4, -0.44, 1.8), r_front=0.45, r_exit=0.42, length=0.45, petals=12)],
    gear=[dict(name="nose", x=2.7, y=0.0, top=1.35, radius=0.26, width=0.16, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-1.5, y=1.2, top=1.62, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-1.5, y=-1.2, top=1.62, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-1.7, 2.0, WING_Z - 0.06), 1.1, 0.3), ((-2.5, 3.3, WING_Z - 0.04), 1.0, 0.24)],
    stores=[("tank", (-1.8, 2.0, 1.18), 4.0, 0.36), ("missile", (-2.5, 3.3, 1.5), 3.65, 0.09),
            ("pod", (-3.7, 5.42, WING_Z), 1.6, 0.12), ("missile", (0.4, 0.55, 1.42), 3.65, 0.09, False),
            ("missile", (-2.6, 0.55, 1.5), 3.65, 0.09, False)],
    nav=dict(left=(-3.9, 5.2, WING_Z + 0.04), right=(-3.9, -5.2, WING_Z + 0.04), tail=(-7.8, 0.0, 5.0)),
)
# The RAF's low-visibility roundels and fin flash, as the side photo.
MARKINGS = [
    ("insignia", dict(kind="uk_lowvis", centre=(1.4, 1.2, 2.6), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("fuselage",))),
    ("insignia", dict(kind="uk_flash", centre=(-5.9, 0.3, 3.45), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("tail_fin_0",))),
    ("insignia", dict(kind="uk_lowvis", centre=(-3.6, 3.4, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.45,
                      onto=("wing_",))),
    ("text", dict(text="ZK353", height=0.2, centre=(-4.2, 1.2, 2.3), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("fuselage",))),
]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("intake_splitter", (0.5, 0.04, 0.62), (3.2, 0, 1.68), m["paint"], hull, lods=MID)
    box("antiglare", (1.1, 0.55, 0.03), (6.0, 0, 3.03), m["dark"], hull, rot=(0, 0.12, 0), lods=MID)
    box("probe_fairing", (1.2, 0.12, 0.12), (5.0, -0.56, 3.0), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (-1.2, 0, 3.0), 0.22, m, hull)
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
