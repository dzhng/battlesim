"""Attach a launcher's hull-frame geometry to its slew ring, and its pack to
its deployment hinge or, for a pack laid with its gun, to the gun."""
import math

import bpy
from mathutils import Euler, Vector

from parts import empty
from vehicle_export import rig


def launcher(v, pivot, elevation, pack, base=(), mount="launcher", vertical=False, turn_x=0.0, pitch_with_deploy=False):
    """The pack pitches about pivot. Only the turntable and pack slew."""
    if v.wreck:
        return
    nodes = None if vertical else rig(v.frame, v.root)[mount]
    parent = nodes[0] if nodes else v.hull
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
    if pitch_with_deploy:
        gun = nodes[1]
        gun["pitch_min_deg"] = gun["pitch_max_deg"] = 0.0
        # The catalog marker is the firing position. Fold it into the model's
        # bind pose so deployment returns it to that point, including in yaw.
        tip = nodes[2]
        turn = Euler((math.radians(turn_x), -math.radians(elevation), 0.0)).to_quaternion()
        local_tip = hinge.matrix_world.inverted() @ tip.matrix_world.translation
        folded = hinge.matrix_world @ (turn.inverted() @ local_tip)
        world = gun.matrix_world.copy()
        world.translation.z = folded.z
        gun.parent = hinge
        gun.matrix_world = world
        tip.location = world.inverted() @ folded
    return hinge


def laid_with_gun(v, pack, base, mount="launcher", pitch_deg=(-5.0, 70.0), trunnion=0.0):
    """A short-range air defence turret, which fires on the move: `pack`
    parts (its gun, and a missile pack laid with it) pitch with the gun as it
    follows its target, about trunnions `trunnion` ahead of the turret's
    pivot; `base` parts slew with the turret, and no deployment moves
    anything. The mount's muzzle is the gun's, at rest."""
    if v.wreck:
        return
    turret, gun, _, _ = rig(v.frame, v.root, {mount: trunnion})[mount]
    gun["pitch_min_deg"], gun["pitch_max_deg"] = pitch_deg
    bpy.context.view_layer.update()
    for part in list(v.hull.children):
        if part.name.startswith(pack):
            owner = gun
        elif part.name.startswith(base):
            owner = turret
        else:
            continue
        world = part.matrix_world.copy()
        part.parent = owner
        part.matrix_world = world
