"""F/A-18E Super Hornet and EA-18G Growler, from assets/references/f_a_18_super_hornet/
and assets/references/ea_18g_growler/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/super_hornet.py -- [--variant=<card>] [--wreck]

What the photos settle: one airframe. Long leading-edge extensions from the
cockpit to the wing root, the caret intakes tucked under them, a shoulder
wing with dogtooth, twin fins canted out, stabilators below them, two
nozzles close together; tall twin nose wheels and main gear on the body's
flanks. The E is a single seat with wing-tip AIM-9s, tanks and missiles;
the Growler has the two-seat canopy, ALQ-218 receiver pods on the wing tips
and ALQ-99 jamming pods (with their ram-air turbines) under the wings and
the body. Both in the Navy's tactical greys.

The references' dimensions (Navy fact file figures agree): length
18.31 m, span 13.62 m over the tip stores, height 4.88 m.

Each one's shell is its own generated skin (`aircraft_skin`, pinned in
`skins/`): Hunyuan3D's model, given side, top and bottom renders of the
guide (`guides/super_hornet.py`, the lofted airframe with its wing and LEX
measured 0.8 m further aft on the public-domain F/A-18F plan silhouette),
whose outline it matches to about 0.1 m once the Growler's is set down and
both spans set (`SKIN`). The skin
carries the LEX, the wings, the canted fins and the caret intakes; the
canopy is fitted to its hump from the old canopy's plan (`CANOPY`), and the
nozzles, the intakes' dark mouths, the gear, the stores and everything else
are still built here, seated on the skin where they touch it (`Probe`).
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

E, G = "us_f_a_18_super_hornet_f_a_18e", "us_ea_18g_growler_electronic_warfare_aircraft"

WING_Z = 2.45
# The nose's tip and the tail's last point (the nozzles' and fins' trailing edges).
NOSE_X, TAIL_X = 9.28, -8.9
# The stations are not centred on the length (`build` stands the airframe
# centred): the skin is centred here, in their frame.
MID_X = (NOSE_X + TAIL_X) / 2
# Each its own skin; the Growler's sits 0.55 m high on its guide's outline
# (its fins came out short): down, the fins' tips held; both spans set to
# the guide's.
SKIN = {E: dict(path="assets/source/skins/f_a_18_super_hornet/us_f_a_18_super_hornet_f_a_18e.glb",
                span=(2.5, 6.74)),
        G: dict(path="assets/source/skins/ea_18g_growler/us_ea_18g_growler_electronic_warfare_aircraft.glb",
                lower=(0.55, 4.0), span=(2.5, 6.65))}
# The skin's parts by box (x, y, z): the wings out from the body's flanks,
# the canted fins above the body's back.
PARTS = [("wing_L", ((-4.5, 1.5), (1.6, 7.5), (1.95, 2.95))), ("wing_R", ((-4.5, 1.5), (-7.5, -1.6), (1.95, 2.95))),
         ("tail_fin_0", ((-8.95, -3.0), (0.3, 2.5), (3.0, 9.0))), ("tail_fin_1", ((-8.95, -3.0), (-2.5, -0.3), (3.0, 9.0)))]
# The canopy's plan (the framed canopy's outline as `canopy` drew it on the
# lofted airframe), its opaque tail fairing onto the spine behind it, and
# its bows: the windscreen's arch, and the Growler's between its two seats.
CANOPY = {E: dict(x_front=5.95, x_back=2.9, half_width=0.5, peak=0.4, bows=(5.15,)),
          G: dict(x_front=5.95, x_back=2.0, half_width=0.5, peak=0.4, bows=(5.15, 3.5))}
# The two nozzles, built in place of the skin's (cut inside their radius).
NOZZLES = [dict(loc=(-7.65, y, 1.96), r_front=0.5, r_exit=0.46, length=0.98, petals=14) for y in (0.5, -0.5)]
NOZZLE_CUT = [((-9.0, -7.75), (y - 0.44, y + 0.44), (1.52, 2.4)) for y in (0.5, -0.5)]
# The skin's launch rails at the wing tips (the lofted model's, drawn into
# the skin and fused with the tip into a deep plate) pressed up into a thin
# tip; the tip stores carry their own rails.
TIP_PRESS = [(((-5.3, 0.6), tuple(sorted((s * 5.7, s * 8.0))), (1.0, 1.94)), 1.94) for s in (1, -1)]
# The guide's tip rails, drawn into each skin running on ahead of the tips'
# leading edge: cut there, as each tip store carries its own rail.
RAIL_CUT = [((-2.3, 0.5), tuple(sorted((s * 6.5, s * 8.0))), (1.8, 2.4)) for s in (1, -1)]
# Each skin's tip span, where the tip stores hang at the tip chord's middle.
TIP = {E: 6.74, G: 6.65}
# The wing pylons (x, y, length, depth) from the skin's underside, and what
# hangs under each: the tanks inboard, the AIM-120s (E) or jammer pods (G)
# outboard; the tips' AIM-9s (E) or receiver pods (G).
PYLONS = [(-1.2, 2.6, 1.2, 0.35), (-1.8, 4.0, 1.1, 0.3)]
STORES = {E: [("tank", -0.1, 4.7, 0.42), ("missile", 0.0, 3.65, 0.09)],
          G: [("tank", -0.1, 4.7, 0.42), ("jammer", 0.1, 4.6, 0.36)]}
TIP_STORE = {E: ("missile", 3.0, 0.065, True), G: ("pod", 3.0, 0.23, False)}


def _canopy(c):
    glass = framed_plan(c["x_front"], c["x_back"], c["half_width"], peak=c["peak"])
    x_back, hw = glass[-1]
    return dict(glass=glass, tail=[(x_back, hw), (x_back - 0.4, hw * 0.55), (x_back - 0.8, 0.03)], bows=c["bows"])


def spec(variant):
    s = dict(nozzles=NOZZLES)
    s["skin"] = dict(length=NOSE_X - TAIL_X, top=4.87, x=MID_X, canopy=_canopy(CANOPY[variant["id"]]),
                     cut=NOZZLE_CUT + RAIL_CUT, press=TIP_PRESS, parts=PARTS, **SKIN[variant["id"]])
    return s


# The caret intakes' mouths, inside the lips of each skin's flat intake
# faces (raked back from the top lip): `aircraft_skin.intake_mouth` corners.
# The Growler's skin drew its faces wider.
MOUTH = {E: ((0.84, 1.5), (1.34, 1.5), (1.34, 2.1), (0.84, 2.1)),
         G: ((0.84, 1.55), (1.38, 1.55), (1.38, 2.25), (0.84, 2.25))}


# Wheelbase and track as the photos give them; each leg's top on the skin's underside.
GEAR = [dict(name="nose", x=3.9, y=0.0, radius=0.33, width=0.17, wheels=2, spread=0.26, door=(0.9, 0.5), light=True),
        dict(name="main_L", x=-1.6, y=1.56, radius=0.5, width=0.25, door=(1.2, 0.55), rake=0.1),
        dict(name="main_R", x=-1.6, y=-1.56, radius=0.5, width=0.25, door=(1.2, 0.55), rake=0.1)]


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    shell = Probe(hull)
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH[variant["id"]]))
    # The wing's stations on the skin: each pylon from the wing's underside,
    # its store under it, and the tips' stores at the tips' mid-thickness.
    v_id = variant["id"]
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(zip(PYLONS, STORES[v_id])):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull, rail=kind == "missile")
    kind, s_len, radius, rail = TIP_STORE[v_id]
    tip = TIP[v_id]
    tip_x = sum(shell.chord(tip - 0.15, 2.0, -6.0)) / 2
    for side, s in ((1, "L"), (-1, "R")):
        mid = (shell.under(tip_x, side * (tip - 0.15)) + shell.over(tip_x, side * (tip - 0.15))) / 2
        A.store(f"store_tip_{s}", kind, (tip_x, side * tip, mid - (0.1 if rail else 0.0)), s_len, radius, m, hull,
                rail=rail)
    A.nav_lights(m, hull, tail=(-8.6, 0.0, 1.96),
                 **{k: (tip_x, side * (tip - 0.3), shell.over(tip_x, side * (tip - 0.3)) + 0.01)
                    for k, side in (("left", 1), ("right", -1))})
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    if variant["id"] == G:
        # The centreline jamming pod hung close under the belly.
        A.store("store_jammer_belly", "jammer", (-1.0, 0.0, shell.under(-1.0, 0.0) - 0.37), 4.6, 0.36, m, hull)
    else:
        box("gun_port", (0.25, 0.1, 0.12), (8.4, 0, shell.over(8.4, 0)), m["black"], hull, lods=NEAR)
    # The nose leg's catapult launch bar, stowed up and forward off the strut.
    nose_leg = next(o for o in hull.children if o.name == "gear_nose")
    box("gear_nose_launch_bar", (0.9, 0.07, 0.07), (0.45, 0, -0.78), m["gear"], nose_leg, rot=(0, -0.55, 0),
        lods=MID)
    # The refuelling probe's door, antiglare.
    box("probe_door", (0.8, 0.12, 0.03), (6.6, -0.46, shell.over(6.6, -0.46)), m["dark"], hull, rot=(0.5, 0, 0),
        lods=NEAR)
    box("antiglare", (0.9, 0.5, 0.03), (6.5, 0, shell.over(6.5, 0)), m["dark"], hull, rot=(0, 0.1, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-1.0, 0, shell.over(-1.0, 0)), 0.24, m, hull)
    A.blade_antenna("antenna_belly", (0.4, 0, shell.under(0.4, 0)), 0.2, m, hull, down=True)
    # The arrestor hook under the tail.
    box("arrestor_hook", (1.6, 0.08, 0.08), (-6.8, 0, shell.under(-6.8, 0) - 0.05), m["steel"], hull,
        rot=(0, 0.12, 0), lods=MID)
    # The Navy's low-visibility greys: the insignia on the intake below the
    # LEX, NAVY aft, the squadron on the LEX and its air wing's code on the fins.
    code, squadron, modex = ("NL", "VAQ-133", "500") if variant["id"] == G else ("AJ", "VFA-31", "200")
    fin_n, fin_up = (0, math.cos(0.35), -math.sin(0.35)), (0, math.sin(0.35), math.cos(0.35))
    A.markings(v, [
        ("insignia", dict(kind="us_lowvis", centre=(4.6, 1.3, 1.95), normal=(0, 1, 0), up=(0, 0, 1), size=0.36,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.8, 4.2, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.8, -4.2, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.55, onto=("wing_R",), mirror=False)),
        ("text", dict(text="NAVY", height=0.3, centre=(-4.6, 1.5, 2.05), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",))),
        ("text", dict(text=squadron, height=0.2, centre=(2.4, 1.6, 2.62), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage", "wing_"))),
        ("text", dict(text=modex, height=0.26, centre=(7.6, 1.0, 2.3), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",))),
        ("text", dict(text=code, height=0.6, centre=(-5.4, 0.78 + 1.5 * fin_up[1] + 0.3, 2.5 + 1.5 * fin_up[2]),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",))),
    ])
    # The stations run from the nose at 9.28 m to the tail at -8.9 m: the
    # airframe stands centred on its length, as its frame's hull is.
    hull.location.x = -MID_X


def wreck(variant, v):
    # Its litter kept within the wreck's footprint on this wide a span.
    A.crash(v, tail_x=-5.0, wing_y=3.6, wing_side=-1, tail_yaw=-0.3, seed=18, litter_reach=0.85)


run("f_a_18_super_hornet", "us_compass_grey", build, wreck, appearances=[E, G], script="super_hornet.py",
    receipt_file="assets/source/roster/f_a_18_super_hornet/super_hornet.source-receipt.json",
    references=[f"assets/references/{f}/references.json" for f in ("ea_18g_growler", "f_a_18_super_hornet")])
