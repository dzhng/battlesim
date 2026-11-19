"""Build a rig-ready aircraft shell from its pinned, UV-mapped donor meshes."""
import bpy
import bmesh
import numpy as np

import skin_pins
import textures
from parts import BASE, TIERS, flat_paint


def _atlas(source, family):
    """Preserve the donor's UVs, albedo and panel normals through the game bake."""
    shader = source.node_tree.nodes.get("Principled BSDF")
    diffuse = shader.inputs["Base Color"].links[0].from_node.image
    normal = shader.inputs["Normal"]
    if normal.is_linked:
        image = normal.links[0].from_node.inputs["Color"].links[0].from_node.image
        normal_png = image.packed_file.data
    else:
        normal_png = textures.png(np.full((4, 4, 4), [128, 128, 255, 255], np.uint8))
    material = flat_paint(family + "_" + source.name, (BASE / textures.COLOUR_SCALE,) * 3,
                          rough=0.8, grime=0, role="paint")
    textures.ATLASES[material.name] = {
        "albedo": diffuse.packed_file.data,
        "normal": normal_png,
        "orm": textures.png(np.full((4, 4, 4), [255, 204, 0, 255], np.uint8)),
    }
    return material


def fitted_airframe(v, family, mats, skip_parts=()):
    source = skin_pins.verified(skin_pins.required(family, variant=v.variant["id"]))
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=source)
    # The burnt shell loses the small panel fittings and receives new char and
    # deformation vertices during wreck authoring.
    ratios = (0.12, 0.07, 0.035, 0.015) if v.wreck else (0.70, 0.28, 0.10, 0.035)
    for part in sorted(set(bpy.data.objects) - before, key=lambda o: o.name):
        grounded = part.pop("grounded", False)
        role = part["material_role"]
        if part.name in skip_parts:
            bpy.data.objects.remove(part, do_unlink=True)
            continue
        if family == "nh90" and part.name in {"airframe_dark", "airframe_gear"}:
            # The CC BY donor is posed with its cargo ramp lowered.  Its
            # landing gear and ramp are merged into this mesh, so remove only
            # the low aft ramp leaf before fitting the closed transport door.
            mesh = bmesh.new()
            mesh.from_mesh(part.data)
            removed = []
            for face in mesh.faces:
                centre = face.calc_center_median()
                if -3.25 < centre.x < 0.50 and centre.z < 1.82 and abs(centre.y) < 2.05:
                    removed.append(face)
            if removed:
                bmesh.ops.delete(mesh, geom=removed, context='FACES')
                mesh.to_mesh(part.data)
            mesh.free()
        # Donor panes are authored with the shared `glass` role, while roster
        # scripts tune the aircraft-specific `canopy` finish. Use that finish
        # for donor glazing so the tint, coverage and cockpit depth authored by
        # the roster actually reach the exported panes.
        if role == "atlas":
            material = _atlas(part.data.materials[0], family)
        elif role == "glass":
            material = mats["canopy"]
        else:
            material = mats[role]
        part.data.materials.clear()
        part.data.materials.append(material)
        del part["material_role"]
        for tier, ratio in zip(TIERS, ratios):
            obj = part.copy()
            obj.data = part.data.copy()
            obj.name = f"{part.name}_LOD{tier}"
            bpy.context.collection.objects.link(obj)
            obj.parent = v.hull
            bpy.context.view_layer.objects.active = obj
            simplify = obj.modifiers.new("detail tier", "DECIMATE")
            simplify.ratio = ratio
            bpy.ops.object.modifier_apply(modifier=simplify.name)
            if grounded:
                # Decimation can move the lowest tyre vertices, especially on
                # a distant tier. Register its contact patch to the same ground.
                height = min(vertex.co.z for vertex in obj.data.vertices)
                for vertex in obj.data.vertices:
                    vertex.co.z -= height
            triangles = obj.modifiers.new("normal map tangents", "TRIANGULATE")
            bpy.ops.object.modifier_apply(modifier=triangles.name)
        bpy.data.objects.remove(part, do_unlink=True)
