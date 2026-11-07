"""F-35A and F-35B Lightning II, from assets/references/f_35_lightning_ii/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f35.py -- [--variant=<card>] [--wreck]

What the photos settle: a short, deep, chined body, the canopy high on it
with one aft frame, caret intakes behind the cockpit, the trapezoid wing and
stabilators, twin fins canted outward, one big engine nozzle; the sensor
window under the nose. The A has the gun's bulge on the left shoulder and a
round nozzle; the B (short take-off, vertical landing) has the lift-fan doors
behind the canopy, the auxiliary inlet doors above the engine and the
swivelling three-bearing nozzle, no gun. Both in the dark grey radar
coating.

The references' dimensions (Lockheed Martin figures agree):
F-35A 15.7 x 10.7 x 4.38 m; F-35B 15.6 x 10.7 x 4.36 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_skin import Probe  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

FA, FB = "f_35a", "us_f_35_lightning_ii_f_35b"

# The wing and stabilators lie on the chine line, the edge the body's flat
# top meets its sloped sides at (the side photo: 2.2 m up).
WING_Z = 2.2
# The fins stand on the tail booms' tops, canted out as the front photo shows.
FIN_Y, FIN_Z, FIN_CANT = 1.32, 2.45, 0.38


def _down(rows, at, dz):
    """Rows with the heights at indices `at` (in each row, or in its
    stations' rows for a body) lowered by `dz`."""
    return [tuple(r - dz if i in at and isinstance(r, float) else r for i, r in enumerate(row)) for row in rows]


def spec(variant):
    s = _spec(variant)
    # The three-view (public domain) stands the body 0.12 m lower under the
    # same fin tips than the lofted model did: everything but the fins down.
    dz = DROP
    s["fuselage"] = _down(s["fuselage"], (2, 3, 4), dz)
    s["bodies"] = [(n, _down(st, (2, 3, 4), dz), *rest) for n, st, *rest in s["bodies"]]
    s["wing"], s["stab"] = _down(s["wing"], (2,), dz), _down(s["stab"], (2,), dz)
    s["canopy"] = dict(s["canopy"], sill=s["canopy"]["sill"] - dz, top=s["canopy"]["top"] - dz)
    s["fins"] = [dict(f, root_z=f["root_z"] - dz, height=f["height"] + dz / math.cos(FIN_CANT)) for f in s["fins"]]
    s["nozzles"] = [dict(n, loc=(n["loc"][0], n["loc"][1], n["loc"][2] - dz)) for n in s["nozzles"]]
    s["gear"] = [dict(g, top=g["top"] - dz) for g in s["gear"]]
    return s


DROP = 0.12


def _spec(variant):
    b = variant["id"] == FB
    top = 4.36 if b else 4.38
    fin = dict(root_x=-4.0, root_z=FIN_Z, height=(top - FIN_Z) / math.cos(FIN_CANT), root_chord=2.45,
               tip_chord=1.3, sweep_m=2.0, cant=FIN_CANT)
    # The ridge behind the canopy: the B's is higher, over its lift fan.
    ridge = (3.0, 3.34, 3.24) if b else (2.96, 3.2, 3.16)
    return dict(
        # One wide chined body (`body` power under 2 is a diamond), its
        # ridge the spine: from the short pointed nose its chine runs back
        # level at 2.2-2.3 m while its top rises to the canopy's sill and
        # the spine behind it; aft of the cockpit it is the wide body the
        # intakes and wing grow out of, narrowing to the nozzle's shroud.
        fuselage=[(7.82, 0.0, 2.16, 2.16), (7.3, 0.34, 1.9, 2.38, 2.14, 1.6), (6.6, 0.5, 1.64, 2.62, 2.18, 1.6),
                  (5.8, 0.6, 1.48, 2.74, 2.24, 1.7), (4.8, 0.76, 1.36, 2.84, 2.3, 1.7),
                  (3.6, 1.0, 1.28, ridge[0], 2.34, 1.6), (2.6, 1.55, 1.3, ridge[1], 2.34, 1.5),
                  (1.0, 1.8, 1.3, ridge[2], 2.3, 1.5), (-1.5, 1.8, 1.3, 3.04, 2.26, 1.5),
                  (-3.5, 1.6, 1.36, 2.86, 2.22, 1.5), (-4.8, 1.35, 1.36, 2.7, 2.1, 1.8),
                  (-5.7, 1.0, 1.28, 2.64, 2.0, 2.2)],
        canopy=dict(x_front=5.8, x_back=3.4 if b else 3.15, sill=2.64, top=3.37, half_width=0.5, bows=(),
                    peak=0.42, tail=0.75),
        bodies=[# The deep belly under it: the weapon bays' flat floor a metre
                # up, its sides sloped in below the chine, rising fore and aft.
                ("fuselage_belly", [(3.3, 0.9, 1.3, 2.1, 2.0, 2.8), (2.2, 1.35, 1.08, 2.1, 1.95, 2.8),
                                    (0.6, 1.45, 1.0, 2.1, 1.95, 2.8), (-1.5, 1.45, 0.98, 2.1, 1.95, 2.8),
                                    (-3.5, 1.3, 1.12, 2.1, 1.95, 2.8), (-5.0, 1.0, 1.45, 2.1, 1.95, 2.6),
                                    (-5.4, 0.0, 1.8, 1.8)], 0.0, 24),
                # The caret intakes' trunks each side of the cockpit, their
                # fronts raked back from the top under the mouths, their
                # outer walls sloped in to the belly, sinking aft into the
                # body.
                *[(f"fuselage_intake_{k}", [(3.42, 0.34, 1.96, 2.5, 2.36, 2.6), (3.1, 0.36, 1.3, 2.5, 2.36, 2.6),
                                            (2.2, 0.4, 1.2, 2.55, 2.36, 2.6), (1.0, 0.3, 1.3, 2.5, 2.3, 2.6),
                                            (0.2, 0.0, 1.9, 1.9)],
                   side * 1.32) for side, k in ((1, "L"), (-1, "R"))],
                # The tail booms beside the nozzle, under the body's flanks
                # forward, carrying the fins and stabilators aft of it.
                *[(f"tail_boom_{k}", [(-1.8, 0.3, 1.9, 2.3, 2.15, 3.0), (-3.6, 0.5, 1.62, 2.5, 2.2, 3.4),
                                      (-5.6, 0.5, 1.66, 2.5, 2.2, 3.4), (-6.25, 0.38, 1.9, 2.42, 2.2, 3.0),
                                      (-6.45, 0.0, 2.16, 2.16)], side * 1.0)
                  for side, k in ((1, "L"), (-1, "R"))]],
        wing_hinge=(0.1, (0.8, 0, 1)),
        nav=dict(left=(-3.0, 5.2, WING_Z - 0.1), right=(-3.0, -5.2, WING_Z - 0.1)),
        wing=[(0.8, 1.5, WING_Z, 5.66, 0.3), (-2.75, 5.35, WING_Z - 0.08, 1.3, 0.06)],
        stab=[(-4.8, 1.2, WING_Z, 3.02, 0.12), (-6.0, 3.6, WING_Z - 0.02, 1.22, 0.04)],
        fins=[dict(fin, y=FIN_Y), dict(fin, y=-FIN_Y)],
        nozzles=[] if b else [dict(loc=(-5.5, 0.0, 1.96), r_front=0.64, r_exit=0.56, length=0.8, petals=14)],
        # The main legs come down under the wing roots, 4.3 m apart (the
        # front photo), the nose leg under the cockpit.
        gear=[dict(name="nose", x=3.75, y=0.0, top=1.5, radius=0.3, width=0.17, door=(0.9, 0.45), light=True),
              dict(name="main_L", x=-2.5, y=2.15, top=2.05, radius=0.4, width=0.24, door=(1.2, 0.6), rake=0.1),
              dict(name="main_R", x=-2.5, y=-2.15, top=2.05, radius=0.4, width=0.24, door=(1.2, 0.6), rake=0.1)],
    )


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    # The electro-optical targeting window under the chin, the distributed
    # aperture windows round the nose.
    box("eots_window", (0.5, 0.36, 0.14), (6.4, 0, 1.6), m["glass"], hull, rot=(0, 0.3, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"das_window_{k}", (0.18, 0.03, 0.14), (5.2, side * 0.66, 2.3), m["glass"], hull,
                                   lods=NEAR))
    box("antiglare", (0.9, 0.5, 0.03), (6.4, 0, 2.64), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"bay_seam_{k}", (3.4, 0.03, 0.03), (0.4, side * 0.5, 1.0), m["dark"], hull,
                                   lods=NEAR))
    # Eielson's AK (the side photo) on the A, the Marines' VK on the B, in
    # the low-visibility grey both wear.
    b = variant["id"] == FB
    fin_n, fin_up = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT)), (0, math.sin(FIN_CANT), math.cos(FIN_CANT))

    def on_fin(x, up):
        return (x, FIN_Y + up * fin_up[1] + 0.3, FIN_Z + up * fin_up[2])
    rows = [
        ("insignia", dict(kind="us_lowvis", centre=(-5.0, 1.5, 1.9), normal=(0, 1, 0), up=(0, 0, 1), size=0.3,
                          onto=("tail_boom",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.2, 3.8, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.45, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.2, -3.8, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.45, onto=("wing_R",), mirror=False)),
        ("text", dict(text="VK" if b else "AK", height=0.48, centre=on_fin(-5.6, 1.0),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",), colour="lowvis_light")),
        ("text", dict(text="169164" if b else "AF 19 5458", height=0.12,
                      centre=on_fin(-5.4, 0.45), normal=fin_n, up=fin_up,
                      onto=("tail_fin_",), colour="lowvis_light")),
    ]
    if b:
        rows.append(("text", dict(text="MARINES", height=0.24, centre=(-4.4, 1.5, 1.88), normal=(0, 1, 0),
                                  up=(0, 0, 1), onto=("tail_boom",), colour="lowvis_light")))
    A.markings(v, rows)
    # The body's top under (x, y), for what lies on its curved skin.
    skin = Probe(hull, ("fuselage",)).over

    if b:
        # The lift-fan door behind the canopy, its outline a dark seam on
        # the hump it lies on; the auxiliary inlet doors aft of it, two dark
        # panels either side of the ridge; the three-bearing swivel nozzle,
        # straight aft.
        x0, x1, w = 3.3, 1.4, 0.5
        ring = ([(x0, w * (1 - 2 * t / 6)) for t in range(6)] + [(x0 - (x0 - x1) * t / 12, -w) for t in range(12)]
                + [(x1, -w + 2 * w * t / 6) for t in range(6)] + [(x1 + (x0 - x1) * t / 12, w) for t in range(12)])
        pts = [(x, y, skin(x, y) + 0.008) for x, y in ring]
        for k, (a, c) in enumerate(zip(pts, pts[1:] + pts[:1])):
            A.line(f"lift_fan_door_seam_{k}", a, c, m["dark"], hull, width=0.07, height=0.03, lods=MID)

        def aux(side, k):
            x, y = 0.75, side * 0.2
            roll = math.atan2(skin(x, y + 0.15) - skin(x, y - 0.15), 0.3)
            pitch = math.atan2(skin(x + 0.45, y) - skin(x - 0.45, y), 0.9)
            box(f"aux_inlet_{k}", (0.9, 0.28, 0.03), (x, y, skin(x, y) + 0.01), m["dark"], hull,
                rot=(roll, -pitch, 0), lods=MID)
        A.mirrored(aux)
        for k, (x, r, l) in enumerate(((-5.5, 0.62, 0.45), (-5.95, 0.59, 0.45), (-6.4, 0.56, 0.45))):
            cyl(f"swivel_duct_{k}", r, l, (x - l / 2, 0, 1.96), "X", m["nozzle"], hull, seg=22, r2=r * 1.03)
            cyl(f"swivel_seam_{k}", r * 1.05, 0.03, (x - l, 0, 1.96), "X", m["dark"], hull, seg=22, lods=NEAR)
        cyl("swivel_bore", 0.48, 0.04, (-6.83, 0, 1.96), "X", m["black"], hull, seg=18, lods=MID)
    else:
        # The gun's fairing on the left shoulder behind the canopy.
        A.body("gun_bulge", [(x, hw, skin(x, 0.95) - 0.08, skin(x, 0.95) + rise, skin(x, 0.95))
                             for x, hw, rise in ((3.0, 0.0, 0.0), (2.6, 0.16, 0.06), (1.0, 0.16, 0.05), (0.4, 0.0, 0.0))],
               m["paint"], hull, seg=10, loc=(0, 0.95, 0), lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-3.8, wing_y=3.2, wing_side=-1, tail_yaw=-0.25, seed=35)


run("f_35_lightning_ii", "us_gunship_grey", build, wreck, script="f35.py")
