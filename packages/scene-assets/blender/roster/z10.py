"""Z-10, from assets/references/z_10/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/z10.py -- [--wreck]

What the photos settle: a slim tandem attack helicopter, the gunner forward
and the pilot raised behind under angular flat-paned canopies, the sensor
turret in the nose, the cannon turret under the chin, engines either side of
the gearbox with exhausts turned out, stub wings with two stations each, the
tail with the fin, a stabilator and the tail rotor on the right; fixed main
wheels and a tail wheel. Dark olive.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the stub wings' stores, height to
the top of the rotor head): 14.15 x 4.32 x 3.85 m.

The cannon is the `chin` mount's rig; the missiles are fixed in the hull
(the `missiles` mount's muzzle is the right outer station's missile front).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_markings import near_shell  # noqa: E402
from aircraft_panes import framed_panels, panel_shell  # noqa: E402
from parts import box, cyl, flat_paint  # noqa: E402
from vehicle_export import rig, run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

# The chin cannon's elevation in its turret, degrees, and its stroke.
GUN_PITCH_DEG = (-50.0, 12.0)
GUN_STROKE_M = 0.06
SPEC = dict(
    fuselage=[(6.61, .28, 1.02, 1.52, 1.33, 4), (6.44, .39, .96, 1.59, 1.29, 4),
              (6.03, .54, .97, 1.78, 1.35, 3.5), (5.27, .60, .83, 1.64, 1.26, 3.8),
              (4.16, .63, .73, 1.79, 1.30, 3.8), (3.98, .65, .75, 1.97, 1.39, 3.8),
              (2.46, .69, .78, 2.16, 1.48, 4), (1.70, .77, .81, 2.58, 1.72, 3.4),
              (.15, .71, .89, 2.47, 1.69, 3.4), (-1.68, .51, 1.10, 2.35, 1.70, 3.2),
              (-3.88, .30, 1.30, 2.08, 1.69, 3), (-5.85, .21, 1.40, 2.08, 1.72, 3),
              (-7.08, .14, 1.68, 2.14, 1.87, 2.6)],
    wing=[(1.6, 0.55, 1.6, 1.2, 0.16), (1.4, 2.06, 1.52, 1.0, 0.12)],
    stab=[(-5.8, 0.2, 1.8, 0.9, 0.1), (-5.9, 1.3, 1.8, 0.7, 0.06)],
    fins=[dict(root_x=-5.6, root_z=1.95, height=1.5, root_chord=1.4, tip_chord=0.9, sweep_m=0.5, thick=0.16)],
    gear=[dict(name="main_L", x=3.22, y=1.0, top=.69, radius=0.32, width=0.18),
          dict(name="main_R", x=3.22, y=-1.0, top=.69, radius=0.32, width=0.18),
          dict(name="tail", x=-5.4, y=0.0, top=1.36, radius=0.16, width=0.1)],
    pylons=[((1.3, 1.1, 1.52), 0.8, 0.22), ((1.25, 1.85, 1.48), 0.8, 0.22)],
    stores=[("pod", (1.2, 1.1, 1.08), 1.6, 0.22), ("missile", (1.1, 2.0, 1.1), 1.7, 0.1)],
)
# How far forward the airframe's parts (placed in their drawing's frame)
# move to centre on the hull box.
AFT_M = -0.008


def access_panels(hull, m):
    # Flat-sided equipment bays and the access covers beneath both seats.
    shell = near_shell(("fuselage_LOD",))
    covers = [
        [(5.91,1.46),(5.75,1.56),(5.38,1.56),(5.32,1.21),(5.80,1.20),(5.91,1.46)],
        [(5.07,1.59),(4.45,1.65),(4.30,1.45),(4.40,1.07),(4.99,1.11),(5.07,1.59)],
        [(3.93,1.74),(3.66,1.83),(3.19,1.80),(3.19,1.47),(3.85,1.47),(3.93,1.74)],
        [(.91,1.89),(.88,1.26),(.28,1.27),(.34,1.90),(.91,1.89)],
        [(-.04,1.92),(-.12,1.29),(-.79,1.36),(-.75,1.95),(-.04,1.92)],
        [(-2.13,1.97),(-2.16,1.54),(-2.66,1.60),(-2.60,1.98),(-2.13,1.97)],
    ]
    for side, suffix in ((1,"L"),(-1,"R")):
        faces = []
        for outline in covers:
            for (x,z),(x1,z1) in zip(outline,outline[1:]):
                dx,dz=x1-x,z1-z
                length=math.hypot(dx,dz)
                steps=math.ceil(length/.14)
                previous=None
                for j in range(steps+1):
                    edge=[]
                    for width in (-.010,.010):
                        xx=x+dx*j/steps-width*dz/length
                        zz=z+dz*j/steps+width*dx/length
                        hit,normal,_,_=shell.ray_cast((xx,side*2,zz),(0,-side,0))
                        if hit is None or side*hit.y<.10:
                            raise ValueError("Access panel missed the side skin")
                        edge.append(tuple(hit+.004*normal))
                    if previous:
                        face=(*previous,edge[1],edge[0])
                        faces.append(face[::-1] if side<0 else face)
                    previous=edge
        # The long shoulder below the cockpit separates the flat upper
        # equipment bay from the rolled lower skin. Seat both edges on the
        # loft, raising only the ridge between them.
        chine = []
        previous = None
        for j in range(43):
            t = j / 42
            x = 6.13 - 8.30*t
            z = 1.36 + .39*t
            edge = []
            for dz, relief in ((-.060, .002), (.0, .036), (.042, .002)):
                point, normal, _, _ = shell.ray_cast((x, side*2, z+dz), (0,-side,0))
                if point is None:
                    raise ValueError(f"Missing fuselage chine at {x}, {z+dz}")
                edge.append(tuple(point + normal*(.002+relief*min(1.,j/3,(42-j)/3))))
            if previous:
                for k in (0, 1):
                    face = (previous[k], edge[k], edge[k+1], previous[k+1])
                    chine.append(face if side > 0 else face[::-1])
            previous = edge
        panel_shell("fuselage_shoulder_"+suffix, chine, m["paint"], hull)
        panel_shell("equipment_bay_seals_"+suffix,faces,m["canopy_frame"],hull)


def side_armor_details(hull, m):
    shell = near_shell(("fuselage_LOD",))
    # The Z-10's lower chine and engine-bay doors are visible at roster distance.
    # Project each seam onto the loft instead of placing it at a guessed y.
    for side, suffix in ((1, "L"), (-1, "R")):
        for index, (x, z0, z1) in enumerate(((5.72, 1.08, 1.72), (4.84, 1.06, 1.82),
                                               (3.62, 1.15, 1.94), (1.46, 1.28, 2.02),
                                               (-.20, 1.39, 2.08))):
            points = []
            for z in (z0, z1):
                point, normal, _, _ = shell.ray_cast((x, side * 2.0, z), (0, -side, 0))
                if point is not None:
                    points.append(tuple(point + normal * .006))
            if len(points) == 2:
                A.line(f"armor_seam_{suffix}_{index}", points[0], points[1], m["canopy_frame"], hull,
                       width=.018, height=.018, lods=A.NEAR)
        # A shallow belly break gives the lower armored tub a readable edge.
        points = []
        for x in (6.02, 5.35, 4.55):
            point, normal, _, _ = shell.ray_cast((x, side * 2.0, 1.02), (0, -side, 0))
            if point is not None:
                points.append(tuple(point + normal * .007))
        if len(points) == 3:
            A.line(f"lower_chine_{suffix}", points[0], points[1], m["dark"], hull,
                   width=.024, height=.024, lods=A.MID)
            A.line(f"lower_chine_{suffix}_aft", points[1], points[2], m["dark"], hull,
                   width=.024, height=.024, lods=A.MID)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # Keep the PLA stores in the same low-contrast olive family as the
    # airframe. The donor weapon material was too bright and read as a clean
    # white placeholder beside the dark reference finish.
    m["store"] = m["dark"]
    # Stations follow the stepped cockpit on the two side elevations. The
    # greater width at the sill gives the side panes their inward lean.
    glass = flat_paint("z10_glazing", (.015, .025, .032), rough=.16, metal=.16, grime=0,
                       coverage=("blended", .66), role="glass")
    panes, roofs, cheeks = [], [], []
    for side in (-1, 1):
        def pt(x, width, z):
            return (x, side*width, z)
        for front, brow, aft, sill_front, sill_aft in [
            (pt(5.99,.39,1.76), pt(4.99,.43,2.53), pt(4.05,.45,2.60),
             pt(5.39,.60,1.63), pt(4.05,.62,1.88)),
            (pt(4.05,.45,2.60), pt(3.13,.47,3.06), pt(2.39,.48,3.09),
             pt(3.98,.64,1.95), pt(2.39,.65,2.20)),
        ]:
            fs = [[pt(front[0],0,front[2]+.04), front, brow, pt(brow[0],0,brow[2]+.04)],
                  [front,sill_front,sill_aft,aft,brow]]
            panes.extend(f if side>0 else f[::-1] for f in fs)
            roof = [pt(brow[0],0,brow[2]+.04),brow,aft,pt(aft[0],0,aft[2]+.04)]
            roofs.append(roof if side>0 else roof[::-1])
            cheek = [sill_front,pt(sill_front[0],.46,sill_front[2]-.22),
                     pt(sill_aft[0],.47,sill_aft[2]-.23),sill_aft]
            cheeks.append(cheek if side>0 else cheek[::-1])
    framed_panels(panes, glass, m["paint"], hull, .09)
    panel_shell("cockpit_roof", roofs, m["paint"], hull)
    panel_shell("cockpit_sills", cheeks, m["paint"], hull)
    A.body("transmission_deck", [(2.40,.49,2.14,3.10,2.62,3.5),
                                 (2.06,.56,2.07,3.23,2.68,3.2),
                                 (1.35,.60,2.12,3.18,2.72,3.1),
                                 (.53,.56,2.20,2.93),(-.40,.40,2.29,2.71)],
           m["paint"], hull, seg=18)
    if not v.wreck:
        for name,x,z in (("gunner",4.73,1.73),("pilot",3.07,2.17)):
            v.crew.append((name,hull,(x,0,z),
                           ((x+.32,.18,z+.25),(x+.32,-.16,z+.23)),
                           ((x+.53,.16,z-.35),(x+.53,-.16,z-.35))))
    if not v.wreck:
        access_panels(hull, m)
        side_armor_details(hull, m)
    # The optical head hangs from the overhanging nose shelf.
    A.body("nose_sight_shelf", [(6.99,.20,1.41,1.61), (6.85,.30,1.38,1.65),
                                (6.52,.35,1.32,1.65), (6.22,.32,1.31,1.58)],
           m["paint"], hull, seg=20)
    cyl("nose_sight_collar", .25, .13, (6.82,0,1.38), "Z", m["dark"], hull, seg=24)
    A.sensor_ball("sensor", (6.82,0,1.15), .265, m, hull, window=.79)
    A.chin_gun(v, rig(v.frame, v.root), m, GUN_PITCH_DEG, GUN_STROKE_M, drum=0.24)
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(2.23,.22,2.29,2.75,2.52,3.4),
                                (2.06,.34,2.10,2.85,2.51,3.1),
                                (1.26,.40,2.07,2.89,2.52,3.2),
                                (-.50,.38,2.00,2.76,2.43,3.0),
                                (-1.19,.30,2.05,2.64,2.36,3.0)],
               m["paint"], hull, seg=18, loc=(0,side*.66,0))
        inlet_y = side*.70
        cyl("intake_lip_"+k, .245, .14, (2.25,inlet_y,2.53), "X",
            m["paint"], hull, seg=24, caps=False)
        cyl("intake_throat_"+k, .238, .014, (2.245,inlet_y,2.53), "X",
            m["black"], hull, seg=24)
        for row in range(-2, 3):
            z = row*.075
            half_span = math.sqrt(.229**2-z*z)
            box(f"intake_vane_{k}_{row}", (.028,2*half_span,.018),
                (2.275,inlet_y,2.53+z), m["dark"], hull, lods=MID)
        # The Z-10's short exhausts have a visible petalled heat shield at
        # roster distance, rather than ending as a plain dark cylinder.
        A.nozzle("engine_exhaust_"+k,(-1.17,side*.67,2.34),.256,.26,.41,m,hull,petals=8)
        # The gear pivots at the shoulder, with a raked load path to the fork.
        A.line("main_gear_leg_"+k,(3.87,side*.61,1.42),(3.22,side*1.00,.744),
               m["dark"],hull,width=.108,height=.11,lods=A.ALL)
        A.line("main_gear_stay_"+k,(3.09,side*.58,1.18),(3.26,side*.97,.759),
               m["dark"],hull,width=.060,height=.065,lods=MID)
        A.store(f"atgm_{k}", "missile", (1.1, side * 2.0, 1.32), 1.7, 0.1, m, hull)
    A.rotor("main", (1.71,0,3.55),6.5,5,.46,m,hull,hub=.38,mast=.37,droop=.025,phase=.4)
    A.rotor("tail", (-6.65, -0.24, 3.0), 1.4, 4, 0.24, m, hull, hub=0.13, mast=0.0, droop=0.0,
            rot=(math.pi / 2, 0, 0), thick=0.12)
    # Small IFF and belly blade antennas break up the otherwise clean shell
    # and give the attack airframe the external fittings shown in the photos.
    A.blade_antenna("iff_antenna", (0.70, 0.0, 2.78), .34, m, hull, chord=.14)
    A.blade_antenna("belly_antenna", (0.92, 0.0, .86), .18, m, hull, chord=.12, down=True)

    # The PLA's star on the boom, a red number on the nose.
    A.markings(v, [
        ("insignia", dict(kind="cn_star", centre=(-3.5, 1.2, 1.68), normal=(0, 1, 0), up=(0, 0, 1), size=0.3, onto=("fuselage",))),
        ("text", dict(text="07", height=0.36, centre=(4.2, 1.2, 1.4), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
    ])
    # The airframe as drawn runs off the frame's middle; the frame (and the
    # mounts' pivots in it) is centred on the hull box.
    v.hull.location.x += AFT_M


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.4, tail_yaw=-0.42, tail_drop=0.1, seed=10)


run("z10", "nato_helicopter_green", build, wreck, crew_army="pla_type21", references="assets/references/z_10/references.json")
