"""CH-47F Chinook and Chinook HC6, from assets/references/ch_47_chinook/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/chinook.py -- [--variant=<card>] [--wreck]

What the photos settle: the long box fuselage with its rounded cockpit nose,
the forward rotor pylon over the cockpit and the tall aft pylon carrying
the rear rotor and the two engines on its flanks, the fuel sponsons along
the lower sides, the loading ramp under the tail, two three-blade rotors
turning in tandem, four wheels (twin forward, single aft). The CH-47F in
the Army's tan, as its photos show; the RAF's HC6 in its dark green.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the sponsons, height to the top
of the rear rotor head): 15.87 x 3.78 x 5.68 m, both.

The gun in the forward left cabin door, swung out on its pintle, is the
`door` mount's HMG rig.
"""
import math
import os
import sys



sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from aircraft_markings import near_shell  # noqa: E402
from aircraft_panes import panel_shell  # noqa: E402
from parts import box, flat_paint  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

F, HC6 = "us_ch_47_chinook_ch_47f", "europe_ch_47_chinook_hc6"
SCHEME = {F: "us_desert_tan", HC6: "nato_helicopter_green"}


def cabin_seals(hull, seal):
    skin = near_shell(("fuselage_chinook",))
    for side, suffix in ((1, "L"), (-1, "R")):
        faces = []
        def on_tank(x, z):
            hit = skin.ray_cast((x, 2.5 * side, z), (0, -side, 0))[0]
            if hit is None:
                raise ValueError(f"No fuel sponson at {x}, {z}")
            return (x, hit.y + side * .004, z)

        for x in (-4.02, -2.28, -.35, 1.64, 3.52):
            previous = None
            for z in (.82, .88, .98, 1.10, 1.22, 1.34, 1.44, 1.51):
                edge = (on_tank(x-.009, z), on_tank(x+.009, z))
                if previous:
                    faces.append((*previous, edge[1], edge[0]))
                previous = edge
        # The window centres and rake follow the pinned glass on both sides.
        for x in (-3.365, -1.22, .815, 2.775):
            def lip(i, radius):
                angle = math.tau * (i % 32) / 32
                dz = radius * math.sin(angle)
                return (x + radius * math.cos(angle), side * (1.375 - .054 * dz), 2.07 + dz)
            for i in range(32):
                faces.append((lip(i, .256), lip(i, .274), lip(i+1, .274), lip(i+1, .256)))
        panel_shell("sponson_joints_and_window_seals_" + suffix,
                    [f[::-1] for f in faces] if side > 0 else faces, seal, hull)


def build(variant, v):
    mats = A.fit(v)
    # Keep the cockpit panes dark and reflective without turning them into a
    # white opaque block in the flank view. Lower coverage exposes cockpit
    # depth behind the framed glazing.
    mats["canopy"] = flat_paint("chinook_panes", (0.010, 0.018, 0.022), rough=0.24,
                               grime=0, coverage=("blended", 0.46), role="glass")
    mats["radome"] = flat_paint("radome_finish", (0.035, 0.041, 0.040), rough=0.68, grime=0.10,
                               role="paint")
    if variant["id"] == F:
        mats["radome"] = flat_paint("army_nose_fairing", (.134, .122, .081), rough=.64, grime=.08, role="paint")
    fitted_airframe(v, "chinook", mats)
    if not v.wreck:
        gasket = flat_paint("chinook_panel_seals", (.050, .046, .033), rough=.84, grime=0, role="paint")
        cabin_seals(v.hull, gasket)
    # The door opening crosses the rounded shoulder of the forward cabin.
    # The outer lip follows the skin; its return exposes the thickness of the
    # sill and jambs and leaves the pilot's side window ahead of the opening.
    for side, suffix in ((1, "L"), (-1, "R")):
        outline = [(4.567, 1.36, 1.31), (5.56, 1.34, 1.31), (6.057, 1.29, 1.31),
                   (6.057, 1.30, 1.92), (6.057, 1.24, 2.51), (5.56, 1.28, 2.51),
                   (4.567, 1.31, 2.51), (4.567, 1.37, 1.92)]
        outer = [(x, side * y, z) for x, y, z in outline]
        inner = [(x, side * (y - 0.13), z) for x, y, z in outline]
        returns = [(outer[i - 1], outer[i], inner[i], inner[i - 1]) for i in range(len(outer))]
        panel_shell("cabin_door_lining_" + suffix, returns if side == 1 else
                    [face[::-1] for face in returns], mats["dark"], v.hull)
    # Each nacelle ends in a heat-darkened, stepped exhaust. Its flange
    # meets the rear of the cowling and the inner liner recedes into it.
    for side, suffix in ((1, "L"), (-1, "R")):
        A.nozzle("exhaust_" + suffix, (-5.65, side * 1.24, 3.52),
                        0.36, 0.27, 0.51, mats, v.hull)
    # The donor's hub centres in the registered frame, with the stock blade
    # names and rig hierarchy that the battle renderer and wreck builder use.
    for name, x, z, phase in (("front", 6.293, 4.17, 0.3), ("rear", -5.900, 5.63, 0.3 + math.pi / 3)):
        A.rotor(name, (x, 0, z), 9.14, 3, 0.68, mats, v.hull, hub=0.40, mast=0.25, phase=phase)
    A.door_guns(v, rig(v.frame, v.root), mats, (-60.0, 20.0), inboard=0.2)
    if not v.wreck:
        # A short visible brace seats the forward gun's pintle into the door
        # jamb. The donor opening is deep and black, so the post otherwise
        # reads as a loose block suspended in the aperture.
        A.line("door_gun_pintle_jamb_brace", (5.21, 1.16, 1.48), (5.21, 1.30, 1.48),
               mats["steel"], v.hull, width=.075, height=.075, lods=A.MID)
        A.line("door_gun_pintle_sill_brace", (5.30, 1.30, 1.48), (5.30, 1.30, 1.32),
               mats["steel"], v.hull, width=.06, height=.06, lods=A.MID)
    if variant["id"] == F:
        # The side photograph places the title on the fuel tank, beneath the windows.
        A.markings(v, [("text", dict(text="UNITED STATES ARMY", height=0.17,
                                         centre=(0, 1.87, 1.25), normal=(0, 1, 0),
                                         up=(0, 0, 1), onto=("fuselage",), colour="black"))])


def wreck(variant, v):
    A.rotorcraft_crash(v, lifting=("front", "rear"), tail_x=-4.6, tail_yaw=0.12, tail_drop=0.05, seed=47)


run("chinook", SCHEME, build, wreck, references="assets/references/ch_47_chinook/references.json",
    authoring="Both variants adapted from mgg8’s CC0 Bravo November airframe (blender/skins/chinook.json). "
              "The CH-47F carries Army paint and lettering. The HC6 retains the RAF finish.")
