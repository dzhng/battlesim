"""J-10C, from assets/references/j_10_vigorous_dragon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/j10.py -- [--wreck]

What the photos settle: a single-engine canard delta. The pointed nose and
bubble canopy, the chin intake under the cockpit (the C's diverterless bump,
where the A had a splitter plate), canards just behind the cockpit, the delta
wing, one tall fin, two ventral fins canted outward under the tail, one
nozzle. Light grey.

The references' dimensions (published figures): length 16.9 m,
span 9.75 m, height 5.43 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the airframe alone from our own renders of the lofted
J-10C this script drew before (both sides, top and bottom). It stood 0.2 m
high with its fin short (`lower`) and its wings 0.25 m long a side
(`span`); the canopy is fitted to its hump from the lofted canopy's plan.
The chin intake's dark mouth, the nozzle (the skin's own cut away inside
it), the thin ventral fins it lost, the gear, stores and antennas are
measured on it or seated on it (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/j_10_vigorous_dragon/eastern_j_10_vigorous_dragon_j_10c.glb"
WING_Z = 1.9
# The canopy's plan: the lofted canopy's outline with the windscreen's foot
# where the skin's hump starts (6.25), the glass ending on the spine at 3.9,
# its opaque tail faired in by 2.9; the windscreen's bow at 5.75.
CANOPY_GLASS = framed_plan(x_front=6.25, x_back=3.9, half_width=0.46, peak=0.45, tail=0.75)
CANOPY_TAIL = [CANOPY_GLASS[-1], (3.5, 0.2), (3.2, 0.1), (2.9, 0.02)]
# The chin intake's dark mouth on the skin's flat face (x 4.3), inside its lips.
MOUTH = ((-0.48, 1.05), (0.48, 1.05), (0.48, 1.6), (-0.48, 1.6))
# The nozzle in place of the skin's open pipe (cut inside its radius).
NOZZLE = dict(loc=(-7.05, 0.0, 2.05), r_front=0.58, r_exit=0.52, length=1.3, petals=14)
NOZZLE_CUT = ((-8.6, -7.15), (-0.55, 0.55), (1.5, 2.6))
# The ventral fins under the tail, canted out; the skin lost them.
VENTRAL = dict(root_x=-4.2, height=0.7, root_chord=1.6, tip_chord=0.9, sweep_m=0.5, rudder=False)
SPEC = dict(
    # The skin's delta and canards (`wing_canard_*`, kept from the wreck's
    # fold) and its fin split off by name.
    skin=dict(path=SKIN, length=16.82, top=5.43, x=0.04, lower=(0.2, 3.75), span=(1.0, 4.87), cut=[NOZZLE_CUT],
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(5.75,)),
              parts=[("wing_canard_L", ((3.4, 5.6), (0.78, 3.0), (1.75, 2.6))),
                     ("wing_canard_R", ((3.4, 5.6), (-3.0, -0.78), (1.75, 2.6))),
                     ("wing_L", ((-8.0, 3.4), (0.95, 6.0), (1.4, 2.4))),
                     ("wing_R", ((-8.0, 3.4), (-6.0, -0.95), (1.4, 2.4))),
                     ("tail_fin_0", ((-8.5, -2.5), (-0.35, 0.35), (2.78, 9.0)))]),
    nozzles=[NOZZLE],
)
# Gear (x, y, radius, width, extras), each leg's top seated under the skin.
GEAR = [("nose", 5.0, 0.0, 0.26, 0.15, dict(door=(0.7, 0.4), light=True)),
        ("main_L", -0.8, 1.3, 0.38, 0.2, dict(door=(1.0, 0.55), rake=0.1)),
        ("main_R", -0.8, -1.3, 0.38, 0.2, dict(door=(1.0, 0.55), rake=0.1))]
# The wing pylons (x, y, length, depth) and what hangs under each: the tank
# inboard, a PL-15 outboard; a PL-10 on the outer rail.
PYLONS = [((-0.6, 2.2, 1.1, 0.3), ("tank", -0.1, 4.0, 0.36)), ((-1.4, 3.3, 0.9, 0.24), ("missile", 0.0, 3.7, 0.1)),
          ((-2.6, 4.3, 0.7, 0.12), ("missile", 0.0, 2.9, 0.08))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for k, side in ((1, 1), (2, -1)):
        x = VENTRAL["root_x"] - VENTRAL["root_chord"] / 2
        A.fin(f"tail_fin_{k}", root_z=shell.under(x, side * 0.55) + 0.04, y=side * 0.55, cant=side * 2.7, mats=m,
              parent=hull, **VENTRAL)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(PYLONS):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull, rail=kind == "missile")
    intake_mouth("intake_mouth", shell, m["black"], hull, 1, MOUTH)
    # The wing-tip lights mid-chord on the delta's narrow tips.
    chord = shell.chord(4.6, 0.0, -8.45)
    tip_x = (max(chord) + min(chord)) / 2
    fin = Probe(hull, ("tail_fin_0",))
    A.nav_lights(m, hull, tail=(-7.5, 0.0, fin.over(-7.45, 0.0) - 0.35),
                 **{k: (tip_x, side * 4.6, shell.over(tip_x, side * 4.6) + 0.01) for k, side in (("left", 1), ("right", -1))})
    A.blade_antenna("antenna_spine", (0.2, 0, shell.over(0.2, 0)), 0.22, m, hull)
    # The PLA's red star and bars aft on the body and on the wings, the
    # photo's red 05 on the nose and the fin.
    A.markings(v, [
        ("insignia", dict(kind="cn_star", centre=(-3.9, 1.2, 2.15), normal=(0, 1, 0), up=(0, 0, 1), size=0.6,
                          onto=("fuselage",))),
        ("insignia", dict(kind="cn_star", centre=(-1.4, 3.0, WING_Z + 0.3), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_",))),
        ("text", dict(text="05", height=0.42, centre=(6.6, 1.0, 2.2), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="red")),
        ("text", dict(text="05", height=0.4, centre=(-5.6, 0.3, 4.4), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="red")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=10)


run("j_10_vigorous_dragon", "chinese_air_light_grey", build, wreck, script="j10.py")
