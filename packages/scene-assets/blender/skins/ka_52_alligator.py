"""Prepare 42manako's CC BY "Russian Ka-52 Alligator" as the Ka-52's fitted donor.

    blender -b --python packages/scene-assets/blender/skins/ka_52_alligator.py -- <source.glb> \
        assets/source/skins/ka_52_alligator/ka52-airframe.glb

The source is the Sketchfab download its pin names (`source_sha256`); repin
the output's sha256 after a rerun.

Bakes the rig's rest pose, drops the rotors (column, hubs, blur), the alpha
cards, the painted-on decals and the nose's pitot boom (thicker than dressing
may stand off the frame, and absent from the photos), turns the nose to +x,
scales the airframe to the catalog's 13.65 m and centres it there, grounds
the wheels, and joins one object per material, each with its
`material_role`, its textures at the game's 1024 px.
"""
import math
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

src, out = sys.argv[sys.argv.index('--') + 1:][:2]
PITOT_X = 0.222  # where the source's pitot boom leaves the radome, nose forward
LENGTH_M = 13.65
MAST = Vector((0.0165, 0.0))  # the coaxial column's axis
ROOF_Z = 0.108  # where the column leaves the fuselage roof
DROP = {"Object_22", "Object_23", "Object_12"}  # hubs and swashplates, rotor blur, alpha cards
PARTS = {"ka52_01_ru_co": ("fuselage", "atlas"), "ka52_02_ru_co": ("airframe", "atlas"),
         "ka52_glass_ca": ("canopy_glass", "glass"), "ka52_weapons_co": ("stores", "atlas")}

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
depsgraph = bpy.context.evaluated_depsgraph_get()
joined = {}
for o in list(bpy.context.scene.objects):
    if o.type != 'MESH' or o.name in DROP or "rotor" in o.name or not o.data.materials:
        continue
    material = o.data.materials[0]
    if material.name not in PARTS:
        continue
    evaluated = o.evaluated_get(depsgraph)
    mesh = bmesh.new()
    mesh.from_object(evaluated, depsgraph)
    mesh.transform(Matrix.Rotation(math.pi / 2, 4, 'Z') @ evaluated.matrix_world)
    # The column above the roof belongs to the rotors the roster script fits.
    column = [f for f in mesh.faces if all(v.co.z > ROOF_Z and (v.co.xy - MAST).length < .025 for v in f.verts)]
    bmesh.ops.delete(mesh, geom=column, context='FACES')
    pitot = [f for f in mesh.faces if all(v.co.x > PITOT_X and abs(v.co.y) < .004 for v in f.verts)]
    bmesh.ops.delete(mesh, geom=pitot, context='FACES')
    joined.setdefault((PARTS[material.name][0], material.name), []).append(mesh)

# Fit the airframe to the catalog length, centred, wheels on the ground.
body = [v.co for pieces in joined.values() for p in pieces for v in p.verts]
nose, tail = max(c.x for c in body), min(c.x for c in body)
ground = min(c.z for c in body)
scale = LENGTH_M / (nose - tail)
fit = Matrix.Diagonal((scale, scale, scale, 1)) @ Matrix.Translation((-(nose + tail) / 2, 0, -ground))
for pieces in joined.values():
    for piece in pieces:
        piece.transform(fit)
for o in list(bpy.data.objects):
    bpy.data.objects.remove(o, do_unlink=True)
for (name, material), pieces in joined.items():
    whole = bmesh.new()
    for piece in pieces:
        data = bpy.data.meshes.new("tmp")
        piece.to_mesh(data)
        whole.from_mesh(data)
        bpy.data.meshes.remove(data)
        piece.free()
    data = bpy.data.meshes.new(name)
    whole.to_mesh(data)
    whole.free()
    data.materials.append(bpy.data.materials[material])
    obj = bpy.data.objects.new(name, data)
    obj["material_role"] = PARTS[material][1]
    bpy.context.scene.collection.objects.link(obj)
    lo = [min(v.co[i] for v in data.vertices) for i in range(3)]
    hi = [max(v.co[i] for v in data.vertices) for i in range(3)]
    print(name, len(data.polygons), "faces from", [round(c, 3) for c in lo], "to", [round(c, 3) for c in hi])
for image in bpy.data.images:
    if max(image.size) > 1024:
        image.scale(*(min(1024, side) for side in image.size))
        image.pack()
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_extras=True, export_yup=True,
                          export_animations=False, export_skins=False)
