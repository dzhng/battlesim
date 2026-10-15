"""The test jet (test unit art, never game content): a generic single-engine
fighter in plain neutral grey, unmarked (and, with --wreck, its burnt wreck).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/test_jet.py -- [out.glb] [--wreck]

Built to the simulation's test jet, `units.test_jet` in
`fixtures/units/test/aircraft.json`: hull half extents [7.54, 5.02, 2.44]
(nose to nozzle, over the wing-tip rails, the ground to the fin's tip). It
reads as a fighter and as no roster jet: a single fin, a swept wing with a
conventional tailplane, side intakes, one nozzle, flying clean, and no
national markings or codes. Its forward cannon is fixed in the hull, so it
has no mount rig. The parts are the aircraft library's (`aircraft_parts`),
as a roster jet's are.

The default output is `assets/source/test/jet.glb`; with --wreck it writes
the wreck beside it (`jet_wreck.glb` and its thrown `jet_wreck_debris.glb`).
"""
import os
import sys
import zlib
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import aircraft_parts as A  # noqa: E402
import parts as P  # noqa: E402
from parts import box, finish, reset, script_args, triangles_by_tier  # noqa: E402
from vehicle_export import Vehicle, materials  # noqa: E402
from vehicle_parts import MID  # noqa: E402
from wreckage import WRECK_ARG, burn, export_wreck, scatter  # noqa: E402

REPO = Path(__file__).resolve().parents[3]
ARGS = script_args()
WRECK = WRECK_ARG in ARGS
POS = [a for a in ARGS if not a.startswith("--")]
OUT = os.path.abspath(POS[0]) if POS else str(REPO / "assets/source/test/jet.glb")

# sim authority: units.test_jet.body.hull.half_extents_m [7.54, 5.02, 2.44]
HALF = (7.54, 5.02, 2.44)
SCHEME = "neutral_grey"
WING_Z = 1.85
TIP_Y = 4.98

SPEC = dict(
    # A pointed radome, the cockpit high on the nose, a round body back to
    # the jet pipe.
    fuselage=[(7.3, 0.0, 1.86, 1.86), (6.8, 0.3, 1.62, 2.08, 1.84), (5.8, 0.5, 1.4, 2.3, 1.82, 2.2),
              (4.4, 0.6, 1.22, 2.42, 1.8, 2.4), (2.6, 0.68, 1.08, 2.5, 1.78, 2.6), (0.0, 0.76, 1.04, 2.52, 1.78, 2.8),
              (-3.0, 0.72, 1.14, 2.46, 1.8, 2.8), (-5.2, 0.62, 1.3, 2.36, 1.82, 2.6), (-6.8, 0.54, 1.4, 2.24, 1.82)],
    bodies=[
        # An intake trunk on each side of the cockpit, faired back into the body.
        *[(f"fuselage_intake_{s}", [(2.9, 0.3, 1.22, 2.02, 1.62, 2.4), (1.0, 0.34, 1.16, 2.06, 1.62, 2.4),
                                    (-1.6, 0.26, 1.24, 2.0, 1.62, 2.2), (-3.6, 0.0, 1.62, 1.62)],
           0.8 if s == "L" else -0.8, 14) for s in "LR"],
        # The spine the canopy fairs into, back to the fin.
        ("fuselage_spine", [(2.1, 0.4, 2.26, 2.8, 2.4, 2.4), (-0.6, 0.6, 2.16, 2.66, 2.3, 2.6),
                            (-4.0, 0.0, 2.36, 2.36)]),
    ],
    canopy=dict(x_front=5.35, x_back=2.05, sill=2.32, top=2.96, half_width=0.46, bows=(4.8,), peak=0.42, tail=0.7),
    # A plain swept wing, about 38 degrees at the leading edge, set mid-body.
    wing=[(0.7, 0.76, WING_Z, 4.4, 0.22), (-2.6, TIP_Y, WING_Z, 1.35, 0.05)],
    wing_hinge=(0.78,),
    # A conventional tailplane either side of the jet pipe.
    stab=[(-4.95, 0.6, 1.74, 2.1, 0.12), (-6.35, 2.7, 1.74, 0.9, 0.05)],
    # The one fin, its tip the frame's top.
    fins=[dict(root_x=-3.6, root_z=2.4, height=2 * HALF[2] - 2.4, root_chord=3.4, tip_chord=1.1, sweep_m=2.4)],
    intakes=[("rect", (2.92, 0.8, 1.62), 0.44, 0.7, 0.2), ("rect", (2.92, -0.8, 1.62), 0.44, 0.7, 0.2)],
    # The one nozzle; its exit is the frame's tail.
    nozzles=[dict(loc=(-6.8, 0.0, 1.82), r_front=0.5, r_exit=0.46, length=HALF[0] - 6.8, petals=14)],
    gear=[dict(name="nose", x=4.1, y=0.0, top=1.26, radius=0.26, width=0.15, light=True),
          dict(name="main_L", x=-1.6, y=1.3, top=1.18, radius=0.36, width=0.2, rake=0.1),
          dict(name="main_R", x=-1.6, y=-1.3, top=1.18, radius=0.36, width=0.2, rake=0.1)],
    nose_probe=((7.3, 0.0, 1.86), HALF[0] - 7.3),
    nav=dict(left=(-2.9, TIP_Y - 0.1, WING_Z + 0.05), right=(-2.9, -TIP_Y + 0.1, WING_Z + 0.05),
             tail=(-7.0, 0.0, 2.6)),
)


def build(v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # Empty wing-tip rails: the span's ends.
    A.mirrored(lambda side, s: box(f"wing_tip_rail_{s}", (1.4, 2 * (HALF[1] - TIP_Y), 0.1),
                                   (-3.25, side * TIP_Y, WING_Z - 0.02), m["dark"], hull, bevel=0.01))
    # The anti-glare panel ahead of the windscreen and the cannon's port.
    box("antiglare", (0.8, 0.5, 0.03), (5.85, 0, 2.24), m["dark"], hull, rot=(0, 0.16, 0), lods=MID)
    box("gun_port", (0.5, 0.04, 0.12), (4.6, 0.6, 2.0), m["black"], hull, lods=MID)
    A.blade_antenna("antenna_spine", (-1.2, 0, 2.66), 0.2, m, hull)
    A.blade_antenna("antenna_belly", (0.4, 0, 1.06), 0.2, m, hull, down=True)


def wreck(v):
    # It came down on its belly, the tail broken behind the wing and the
    # right wing folded; its litter kept within the wreck's footprint.
    A.crash(v, tail_x=-4.0, wing_y=2.8, wing_side=-1, seed=43, litter_reach=0.95)


reset()
P.SCORCH.clear()
variant = dict(id="test_jet", frame=dict(body_dimensions_m=[2 * h for h in HALF], mounts=[]))
v = Vehicle(variant, materials(SCHEME), SCHEME)
v.wreck = WRECK
build(v)
if WRECK:
    wreck(v)
    scatter((HALF[0], HALF[1]), v.mats, seed=zlib.crc32(b"test_jet"))
    burn()
finish(ao_distance=1.0)
counts = triangles_by_tier()
if not all(counts[k] > counts[k + 1] > 0 for k in range(3)):
    raise SystemExit(f"test_jet: tiers must reduce, got {counts}")
if WRECK:
    print("WRECK test_jet", counts, export_wreck(OUT), flush=True)
else:
    P.export(OUT)
    print("TEST_JET", counts, flush=True)
