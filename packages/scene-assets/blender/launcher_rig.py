"""Attach a launcher's hull-frame geometry to its slew ring and deployment hinge."""
import bpy
from mathutils import Vector

from parts import empty
from vehicle_export import rig


def launcher(v, pivot, elevation, pack, base=(), mount="launcher", vertical=False, turn_x=0.0):
    """The pack pitches about pivot. Only the turntable and pack slew."""
    if v.wreck:
        return
    parent = v.hull if vertical else rig(v.frame, v.root)[mount][0]
    bpy.context.view_layer.update()
    hinge = empty("deploy_launcher", parent.matrix_world.inverted() @ Vector(pivot), parent,
                  props={"deploy_start": 0.0, "deploy_end": 1.0, "deploy_turn_x": turn_x, "deploy_turn_y": -elevation})
    bpy.context.view_layer.update()
    for part in list(v.hull.children):
        if part.name.startswith(pack):
            owner = hinge
        elif part.name.startswith(base):
            owner = parent
        else:
            continue
        # All parts were authored in hull space. Change basis once at rest,
        # including lofts whose vertices already contain their hull offsets.
        world = part.matrix_world.copy()
        part.parent = owner
        part.matrix_world = world
    return hinge
