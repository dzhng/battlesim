"""NH90 TTH, AW101 Merlin HC4 and AW159 Wildcat AH1, from
assets/references/nh90/, assets/references/aw101_merlin/ and
assets/references/aw159_wildcat/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/utility_helis.py -- [--variant=<card>] [--wreck]

Three European utility rotorcraft drawn by one script (each is its own
variant; they share only the parts library). What the photos settle:

- NH90 TTH: the deep cabin with its big sliding door, the rear ramp under
  the up-swept tail, the roof fairing over two engines, sponsons carrying the
  main wheels, the twin nose wheel, four main blades, the tail rotor on the
  fin's left.
- Merlin HC4: a long cabin and rear ramp, three engines on the roof (two
  forward, one behind the gearbox), sponsons with the main wheels, the twin
  nose wheel, five main blades, the tall canted tail pylon and its rotor on
  the left with a stabiliser on the right; folded-blade navy heritage, the
  Commando's dark scheme.
- Wildcat AH1: the compact cabin and ball sight above a short nose shelf,
  the long tail boom, swept tail pylon and twin end plates on the stabiliser,
  the four BERP-tipped blades, the main gear legs splayed out; Army grey.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the widest fixed part, height to
the top of the rotor head): NH90 16.13 x 3.6 x 4.33 m; Merlin 19.53 x 4.52 x
4.95 m; Wildcat 13.0 x 3.0 x 3.73 m.

Each carries a heavy machine gun swung out of its left cabin door on a
pintle: the `door` mount's HMG rig.
"""
import math
import os
import sys

import bmesh
import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from aircraft_panes import panel_shell  # noqa: E402
from aircraft_wildcat import fit_wildcat  # noqa: E402
from parts import box, cyl, flat_paint  # noqa: E402
from vehicle_export import rig, run  # noqa: E402
from vehicle_parts import MID, wheel_node  # noqa: E402

NH90, MERLIN, WILDCAT = "europe_nh90_tth", "europe_aw101_merlin_hc4", "europe_aw159_wildcat_ah1"
# The door gun's elevation on its pintle, degrees.
DOOR_PITCH_DEG = (-60.0, 20.0)
WILDCAT_SPEC = dict(
        fuselage=[(6.48, .23, 1.07, 1.21, 1.14, 3.3), (6.29, .48, .91, 1.25),
                  (5.60, .70, .71, 1.43, 1.09, 3.3), (4.85, .86, .52, 1.28, .99, 3.6),
                  (3.28, .91, .45, 1.33, 1.02, 4), (3.10, .92, .45, 2.22, 1.33, 4.3),
                  (.20, .90, .50, 2.22, 1.34, 4.2), (-1.16, .64, .90, 2.0, 1.48, 3.0),
                  (-2.25, .37, 1.32, 1.88), (-5.43, .21, 1.56, 1.94),
                  (-6.49, .18, 1.61, 2.64, 1.96)],
        bodies=[("fuselage_roof", [(3.36, .48, 2.11, 2.25), (2.75, .73, 2.07, 2.57),
                                   (1.83, .77, 2.08, 2.91, 2.55, 3), (.23, .71, 2.03, 2.96),
                                   (-1.46, .55, 2.00, 2.70), (-1.72, .30, 2.04, 2.47)])],
        fins=[dict(root_x=-5.2, root_z=1.8, height=1.25, root_chord=1.0, tip_chord=0.7, sweep_m=0.6, thick=0.16)],
        gear=[dict(name="nose", x=4.4, y=0.0, top=0.6, radius=0.23, width=0.14, wheels=2, spread=0.26),
              dict(name="main_L", x=-0.4, y=1.38, top=0.8, radius=0.32, width=0.2, strut_radius=.082),
              dict(name="main_R", x=-0.4, y=-1.38, top=0.8, radius=0.32, width=0.2, strut_radius=.082)],
)


def merlin_cabin(mats, hull):
    # The donor is split at the two door apertures. Carry each cut edge
    # inward to its jamb, with the floor flush with the bottom sill.
    outline = [(1.55, 1.34, 1.08), (3.15, 1.34, 1.08),
               (3.15, 1.37, 1.47), (3.15, 1.37, 1.88),
               (3.15, 1.32, 2.30), (3.15, 1.18, 2.60),
               (1.55, 1.18, 2.60), (1.55, 1.32, 2.30),
               (1.55, 1.37, 1.88), (1.55, 1.37, 1.47)]
    for side, suffix in ((1, "L"), (-1, "R")):
        skin = [(x, side*y, z) for x, y, z in outline]
        jamb = [(x, side*(y-.10), z) for x, y, z in outline]
        faces = [(skin[i-1], skin[i], jamb[i], jamb[i-1]) for i in range(len(skin))]
        for name, strips in (("sill", faces[1:2]), ("forward", faces[2:6]),
                             ("header", faces[6:7]), ("aft", faces[7:] + faces[:1])):
            panel_shell(f"cabin_jamb_{suffix}_{name}",
                        [face[::-1] for face in strips] if side > 0 else strips, mats["paint"], hull)
    box("cabin_floor", (3.85, 2.52, .07), (2.27, 0, 1.047), mats["dark"], hull)
    # Two folding seats face forward from the aft cabin bay. Leave the
    # centre aisle and the space between the two doors open.
    for side, suffix in ((1, "L"), (-1, "R")):
        y = side * .73
        box("seat_pan_" + suffix, (.49, .48, .08), (1.22, y, 1.48),
            mats["canvas"], hull, bevel=.035, lods=MID)
        box("seat_back_" + suffix, (.075, .48, .52), (.997, y, 1.76),
            mats["canvas"], hull, rot=(0, -.12, 0), bevel=.034, lods=MID)
        for sign in (-1, 1):
            foot_y = y + sign*.18
            A.line(f"seat_leg_{suffix}_{sign}", (1.13, foot_y, 1.08),
                   (1.32, foot_y, 1.46), mats["steel"], hull,
                   width=.04, height=.04, lods=MID)
        box(f"cabin_door_sill_{suffix}", (1.55, .09, .07), (2.35, side * 1.31, 1.08),
            mats["steel"], hull, bevel=.015, lods=MID)
        box(f"cabin_door_grab_{suffix}", (.06, .06, .52), (2.70, side * 1.25, 1.63),
            mats["steel"], hull, bevel=.012, lods=MID)


def merlin_surface_details(mats, hull):
    """Break up the long Merlin shell with seated access panels and fasteners."""
    for side, suffix in ((1, "L"), (-1, "R")):
        y = side * 1.39
        box(f"merlin_access_panel_{suffix}", (1.02, .035, .58), (-.10, y, 2.03),
            mats["panel"], hull, bevel=.022, lods=MID)
        for index, x in enumerate((-.48, .28)):
            A.line(f"merlin_panel_seam_{suffix}_{index}", (x, side * 1.415, 1.77),
                   (x, side * 1.415, 2.29), mats["dark"], hull,
                   width=.022, height=.016, lods=MID)
        for index, x in enumerate((-.42, .20)):
            cyl(f"merlin_panel_fastener_{suffix}_{index}", .022, .008, (x, side * 1.43, 2.03),
                "Y", mats["steel"], hull, seg=8, lods=MID)


# The donor is posed with its cargo ramp lowered from its hinge at the aft
# cabin floor, its loading toes resting on the ground. Closed, the leaf is the
# aft belly, climbing from that hinge to the tail boom's underside; the toes
# have nothing to rest on and go.
RAMP_HINGE = Vector((-0.25, 0, 0.87))  # in the donor's frame, metres
RAMP_LOWERED_TIP = Vector((-2.40, 0, 0.10))  # the leaf's edge where the toes hinge
RAMP_CLOSED_TIP = Vector((-3.00, 0, 1.89))  # the boom's underside, where the aperture ends


def close_nh90_ramp(part):
    """Swing the donor's lowered ramp leaf (and its rams) shut on its hinge."""
    hinge = RAMP_HINGE
    lowered, closed = RAMP_LOWERED_TIP - hinge, RAMP_CLOSED_TIP - hinge
    along, along_closed = lowered.normalized(), closed.normalized()
    up, up_closed = along.cross(Vector((0, 1, 0))), along_closed.cross(Vector((0, 1, 0)))
    stretch = closed.length / lowered.length

    def on_ramp(co):
        return -2.85 < co.x < -0.15 and abs(co.y) < 0.97 and co.z < hinge.z + (co.x - hinge.x) * 0.37 + 0.15

    mesh = bmesh.new()
    mesh.from_mesh(part.data)
    to_world, to_local = part.matrix_world, part.matrix_world.inverted()
    leaf = [face for face in mesh.faces if all(on_ramp(to_world @ vert.co) for vert in face.verts)]
    toes = [face for face in leaf if any((to_world @ vert.co).x < RAMP_LOWERED_TIP.x - .01 for vert in face.verts)]
    bmesh.ops.delete(mesh, geom=toes, context='FACES')
    leaf = [face for face in leaf if face.is_valid]
    if leaf:
        geom = bmesh.ops.split(mesh, geom=leaf)["geom"]
        for vert in (item for item in geom if isinstance(item, bmesh.types.BMVert)):
            offset = to_world @ vert.co - hinge
            d, h = offset.dot(along), offset.dot(up)
            moved = hinge + along_closed * d * stretch + up_closed * h
            moved.y = offset.y
            vert.co = to_local @ moved
    mesh.to_mesh(part.data)
    mesh.free()


def nh90(v):
    mats = A.fit(v)
    mats["paint"] = flat_paint("nh90_olive", (.042, .060, .034), rough=.67, grime=.08, role="paint")
    mats["panel"] = flat_paint("nh90_panel", (.055, .073, .043), rough=.55, grime=.05, role="paint")
    mats["paint"]["paint_edge_scale"] = 3.0
    mats["canopy"] = flat_paint("nh90_glass", (.011, .023, .028), rough=.14, grime=0,
                               coverage=("blended", .30), role="glass")
    fitted_airframe(v, "nh90", mats, adjust=close_nh90_ramp)
    for station in ("nose_L", "nose_R", "main_L", "main_R"):
        tyre = bpy.data.objects[f"tyre_{station}_LOD0"]
        corners = [Vector(corner) for corner in tyre.bound_box]
        centre = (corners[0] + corners[6]) / 2
        axle = wheel_node("wheel_" + station, centre, centre.z, v.hull)
        if station.startswith("nose"):
            axle.location.y += math.copysign(.064, centre.y)
        for part in ("tyre", "rim"):
            for tier in A.ALL:
                obj = bpy.data.objects[f"{part}_{station}_LOD{tier}"]
                obj.parent = axle
                obj.location = -centre
    A.rotor("main", (2.51, 0, 4.05), 8.15, 4, .60, mats, v.hull, hub=.38, mast=.48, droop=.025, phase=.30)
    A.rotor("tail", (-7.82, .35, 3.75), 1.6, 4, .27, mats, v.hull, hub=.15, mast=.32, droop=0,
            rot=(-math.pi / 2, 0, 0))
    A.door_guns(v, rig(v.frame, v.root), mats, DOOR_PITCH_DEG, inboard=.20)
    A.markings(v, [
        ("insignia", dict(kind="it", centre=(-3.6, .9, 2.15), normal=(0, 1, 0), up=(0, 0, 1),
                          size=.24, onto=("fuselage",))),
        ("text", dict(text="ESERCITO", height=.12, centre=(-4.45, 1.12, 2.3), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("fuselage",), colour="black")),
    ])


def build(variant, v):
    vid = variant["id"]
    if vid == MERLIN:
        mats = A.fit(v)
        mats["panel"] = flat_paint("merlin_panel_green", (.055, .069, .048), rough=.52, grime=.05, role="paint")
        fitted_airframe(v, "aw101_merlin", mats)
        merlin_cabin(mats, v.hull)
        merlin_surface_details(mats, v.hull)
        A.rotor("main", (3.245, 0, 4.39), 9.3, 5, 0.64, mats, v.hull, hub=0.42,
                mast=0.42, droop=0.025, phase=0.2, tip_chord=0.78)
        A.rotor("tail", (-8.452, 1.09, 4.14), 2.0, 4, 0.26, mats, v.hull,
                hub=0.15, mast=0.0, droop=0.0, rot=(-math.pi / 2, 0, 0))
        A.door_guns(v, rig(v.frame, v.root), mats, DOOR_PITCH_DEG, inboard=0.15)
        return
    if vid == NH90:
        nh90(v)
        return
    m = A.jet(v, WILDCAT_SPEC)
    hull = v.hull
    m["blade"] = m["dark"]
    fit_wildcat(hull, m)
    # The Wildcat's compact attack fit has a real pair of stub wings.  Keep
    # the roots buried in the shoulder fairings and carry a visible outboard
    # pylon and sensor pod on each side so the attack role reads in profile.
    for side, suffix in ((1, "L"), (-1, "R")):
        A.surface(f"stub_wing_{suffix}",
                  [(1.18, side * .90, 1.91, .62, .13),
                   (.18, side * 1.25, 1.87, .72, .12),
                   (-.86, side * 1.45, 1.80, .42, .10)],
                  m["paint"], hull)
        A.pylon(f"wildcat_pylon_{suffix}", (.05, side * 1.34, 1.78), .62, .20, m, hull)
        A.store(f"wildcat_pod_{suffix}", "pod", (-.16, side * 1.34, 1.45), 1.15, .16, m, hull,
                fins=False, rail=True)
    radius, blades, chord, hub_z = 6.4, 4, .5, 3.45
    cyl("mast", 0.18, 0.8, (0.3, 0, hub_z - 0.45), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, hub_z), radius, blades, chord, m, hull, hub=0.42, mast=0.0, droop=0.025, phase=0.2,
            tip_chord=chord * 1.2)
    at, r = (-6.2, .25, 2.6), .86
    A.rotor("tail", at, r, 4, 0.26, m, hull, hub=0.15, mast=0.0, droop=0.0, rot=(-math.pi / 2, 0, 0), thick=0.16)
    A.blade_antenna("antenna_belly", (-0.8, 0, .82), 0.25, m, hull, down=True)
    A.door_guns(v, rig(v.frame, v.root), m, DOOR_PITCH_DEG, inboard=0.15)

    A.markings(v, [
        ("insignia", dict(kind="uk_lowvis", centre=(-3.0, 1.2, 1.62), normal=(0, 1, 0), up=(0, 0, 1), size=0.22, onto=("fuselage",))),
        ("text", dict(text="ARMY", height=0.22, centre=(-4.6, 1.2, 1.55), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ("text", dict(text="ZZ387", height=0.18, centre=(-1.0, 1.2, 1.4), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
    ])
    # The airframe as drawn runs off the frame's middle; the frame (and the
    # mounts' pivots in it) is centred on the hull box.
    v.hull.location.x += .025


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-3.6 if variant["id"] != WILDCAT else -2.2, tail_yaw=0.4, tail_drop=0.08, seed=90)


# The Army's Wildcats fly in grey, as their photos show; NH90 and Merlin in green.
SCHEME = {NH90: "nato_helicopter_green", MERLIN: "nato_helicopter_green", WILDCAT: "nato_air_grey"}

run("utility_helis", SCHEME, build, wreck,
    references=[f"assets/references/{f}/references.json" for f in ("nh90", "aw101_merlin", "aw159_wildcat")],
    authoring="Merlin HC4 adapted from mgg8’s CC0 EH101 Merlin HM.1 (blender/skins/aw101_merlin.json). "
              "NH90 adapted from davidmarton1987’s CC BY 4.0 NH90 BundesHeer Gear out. Wildcat built from its references.")
