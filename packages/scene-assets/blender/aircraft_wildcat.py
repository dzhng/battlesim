"""Fuselage openings, glazing and external fittings for the Army Wildcat."""
import bpy

import aircraft_parts as A
from aircraft_panes import framed_panels, panel_shell
from parts import box, cyl, flat_paint
from vehicle_parts import MID


def fit_wildcat(hull, m):
    bpy.ops.mesh.primitive_cube_add(size=1, location=(1.96, 0, 1.43))
    opening = bpy.context.object
    opening.dimensions = (1.70, 3.0, 1.32)
    bpy.context.view_layer.update()
    for tier in A.ALL:
        shell = bpy.data.objects[f"fuselage_LOD{tier}"]
        cut = shell.modifiers.new("sliding door opening", "BOOLEAN")
        cut.object = opening
        bpy.context.view_layer.objects.active = shell
        bpy.ops.object.modifier_apply(modifier=cut.name)
    bpy.data.objects.remove(opening, do_unlink=True)
    windscreen, roof = [], []
    for side, suffix in ((1, "L"), (-1, "R")):
        # The sliding door is parked aft of the through-cabin opening.
        box(f"cabin_door_{suffix}", (1.35, .055, 1.18), (.53, side*.92, 1.46),
            m["paint"], hull, bevel=.035, lods=MID)
        box(f"door_glass_{suffix}", (.84, .018, .48), (.56, side*.953, 1.73),
            m["glass"], hull, bevel=.025, lods=MID)
        brow = (4.44, side*.70, 2.27)
        corner = (5.33, side*.76, 1.45)
        sill = (4.48, side*.89, 1.31)
        rear = (3.15, side*.92, 1.29)
        rail = (3.15, side*.83, 2.21)
        faces = [[(5.52, 0, 1.42), corner, brow, (4.52, 0, 2.30)],
                 [corner, sill, brow], [sill, rear, rail, brow]]
        # Windshield normals face forward; door glass faces outboard.
        windscreen.extend(f if side > 0 else f[::-1] for f in faces)
        face = [(4.52, 0, 2.33), brow, rail, (3.10, 0, 2.28)]
        roof.append(face if side > 0 else face[::-1])
        # The sponson spans from the cabin skin to the top of the leg.
        stubs = A.surface("gear_stub_" + suffix,
                  [(-.10, side*.70, .91, .64, .19), (-.16, side*1.19, .90, .60, .18),
                   (-.18, side*1.49, .88, .52, .16)], m["paint"], hull)
        for tier, stub in zip(A.ALL, stubs):
            shell = bpy.data.objects[f"fuselage_LOD{tier}"]
            join = shell.modifiers.new("integral gear sponson", "BOOLEAN")
            join.operation = "UNION"
            join.object = stub
            bpy.context.view_layer.objects.active = shell
            bpy.ops.object.modifier_apply(modifier=join.name)
            bpy.data.objects.remove(stub, do_unlink=True)
        # External runner and recessed latch on the sliding cabin door.
        box("door_runner_" + suffix, (2.48, .050, .040), (1.52, side*.920, 2.046),
            m["gear"], hull, bevel=.012, lods=MID)
        box("door_handle_" + suffix, (.18, .032, .038), (.99, side*.962, 1.48),
            m["dark"], hull, bevel=.012, lods=MID)
        box("cabin_step_" + suffix, (1.40, .17, .069), (1.93, side*.955, .73),
            m["gear"], hull, bevel=.018, lods=MID)
        A.body("engine_cowling_" + suffix,
               [(.99, .25, 2.35, 2.89), (.80, .34, 2.26, 2.98, 2.61, 2.8),
                (.06, .37, 2.23, 2.96, 2.61, 3), (-.82, .32, 2.27, 2.89),
                (-1.12, .26, 2.36, 2.85)], m["paint"], hull, loc=(0, side*.68, 0))
        A.round_intake("engine_inlet_" + suffix, (.99, side*.68, 2.62),
                       .222, .075, m, hull)
        A.nozzle("engine_exhaust_" + suffix, (-1.10, side*.68, 2.62),
                 .25, .30, .52, m, hull)
        A.surface("tail_stab_" + suffix,
                  [(-4.56, side*.15, 1.72, .62, .08),
                   (-4.79, side*1.21, 1.74, .47, .07)], m["paint"], hull)
        A.surface("tail_endplate_" + suffix,
                  [(-5.01, side*1.21, 1.21, .32, .045),
                   (-4.68, side*1.21, 1.74, .60, .052),
                   (-4.96, side*1.21, 2.25, .34, .045)], m["paint"], hull)
        box("pilot_seat_" + suffix, (.24, .50, .68), (3.68, side*.42, 1.61),
            m["cockpit"], hull, bevel=.06, lods=MID)
    glazing = flat_paint("wildcat_glazing", (.010, .020, .028), rough=.16, grime=0,
                         role="glass")
    shell = bpy.data.objects["fuselage_LOD0"]
    lower_panes = []
    for side in (-1, 1):
        corners = []
        for x, z in ((4.78, 1.17), (4.69, .94), (4.26, .94), (4.20, 1.14), (4.30, 1.23)):
            hit, point, _, _ = shell.ray_cast((x, side*1.6, z), (0, -side, 0))
            if not hit:
                raise ValueError("Lower pilot door window extends beyond the airframe")
            corners.append((x, point.y+side*.012, z))
        lower_panes.append(corners if side > 0 else corners[::-1])
    panel_shell("lower_door_glass", lower_panes, m["glass"], hull)
    A.body("instrument_coaming", [(5.50, .62, 1.30, 1.46), (5.23, .72, 1.29, 1.48),
                                 (4.96, .74, 1.28, 1.49)], m["cockpit"], hull, seg=16)
    # The attack cockpit needs a readable instrument deck behind the glazing.
    # Keep the panel shallow and seated on the coaming so it survives the
    # roster camera without turning into a floating black slab.
    box("instrument_panel", (.46, 1.05, .08), (4.78, 0, 1.47), m["dark"], hull,
        bevel=.025, rot=(0, -.08, 0), lods=MID)
    for side, suffix in ((1, "L"), (-1, "R")):
        box(f"mfd_{suffix}", (.13, .22, .035), (4.67, side * .27, 1.49), m["glass"], hull,
            bevel=.012, rot=(0, -.08, 0), lods=MID)
    framed_panels(windscreen, glazing, m["paint"], hull, .10)
    # The Wildcat has a visible centre mullion between the two forward panes.
    # A solid post keeps the windscreen readable when the glazing is shaded
    # against the sky, while the side frames remain part of the pane shell.
    box("windscreen_center_post", (.065, .055, 1.08), (5.43, 0, 1.78), m["paint"], hull,
        bevel=.018, lods=MID)
    for index, x in enumerate((1.57, 2.32)):
        # Forward-facing folding seats bolt to the cabin floor.
        box(f"troop_seat_base_{index}", (.48, .54, .32), (x, -.32, .93),
            m["dark"], hull, bevel=.032, lods=MID)
        box(f"troop_seat_canvas_{index}", (.49, .55, .09), (x, -.32, 1.135),
            m["cockpit"], hull, bevel=.033, lods=MID)
        box(f"troop_seat_back_{index}", (.09, .52, .48), (x-.205, -.32, 1.415),
            m["cockpit"], hull, bevel=.035, lods=MID)
    panel_shell("cockpit_roof", roof, m["paint"], hull)
    cyl("sight_pedestal", .17, .09, (6.18, 0, 1.275), "Z", m["paint"], hull)
    A.sensor_ball("nose_sight", (6.18, 0, 1.52), .24,
                  {**m, "dark": m["paint"]}, hull)
