"""Freeze the shared infantry art into vehicle crew poses at source export.

Crew are part of the vehicle appearance, not extra simulation soldiers. The
driver follows the body; a gunner follows the mount's yaw, not barrel pitch.
The existing infantry source supplies clothing, equipment and all four tiers.
"""
import math
import os
import copy
import json
import struct

import bmesh
import bpy
from mathutils import Matrix, Vector

SOURCE = os.path.abspath(os.path.join(os.path.dirname(__file__),
                                    "../../../assets/source/infantry/rifle.glb"))


def crew(name, parent, hips, hands, feet):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=SOURCE)
    imported = set(bpy.data.objects) - before
    arm = next(o for o in imported if o.type == "ARMATURE")
    meshes = sorted((o for o in imported if o.type == "MESH"
                     and any(m.type == "ARMATURE" and m.object == arm for m in o.modifiers)),
                    key=lambda o: o.name)
    arm.animation_data_clear()
    # The shared source faces -Y. Vehicle art faces +X. Place its pelvis at
    # the seat / standing position, preserving the source's authored scale.
    arm.matrix_world = Matrix.Rotation(math.pi / 2, 4, "Z") @ arm.matrix_world
    bpy.context.view_layer.update()
    pelvis = arm.matrix_world @ arm.pose.bones["pelvis"].head
    arm.location += Vector(hips) - pelvis
    bpy.context.view_layer.update()

    targets = []
    for side, hand, foot in zip(("l", "r"), hands, feet):
        for bone, point, pole_point in (
            (f"lowerarm_{side}", hand, (hips[0] - 0.1, hips[1] + (0.55 if side == "l" else -0.55), hips[2] + 0.15)),
            (f"calf_{side}", foot, (hips[0] + 0.8, hips[1] + (0.15 if side == "l" else -0.15), hips[2] - 0.2)),
        ):
            target = bpy.data.objects.new(f"{name}_{bone}_target", None)
            pole = bpy.data.objects.new(f"{name}_{bone}_pole", None)
            bpy.context.collection.objects.link(target)
            bpy.context.collection.objects.link(pole)
            target.location = point
            pole.location = pole_point
            targets.extend((target, pole))
            constraint = arm.pose.bones[bone].constraints.new("IK")
            constraint.target = target
            constraint.pole_target = pole
            constraint.pole_angle = -math.pi / 2
            constraint.chain_count = 2
    bpy.context.view_layer.update()

    for mesh in meshes:
        # Remove the hand-bound rifle, retaining black helmet fittings,
        # belt buckles and radio aerials elsewhere on the shared soldier.
        bm = bmesh.new()
        bm.from_mesh(mesh.data)
        gun = {i for i, m in enumerate(mesh.data.materials) if m.name.split(".")[0] == "gun_black"}
        hand = mesh.vertex_groups["hand_r"].index
        weights = bm.verts.layers.deform.active
        rifle = [f for f in bm.faces if f.material_index in gun
                 and all(v[weights].get(hand, 0) > 0.99 for v in f.verts)]
        bmesh.ops.delete(bm, geom=rifle, context="FACES")
        bm.to_mesh(mesh.data)
        bm.free()
        tier = mesh.name.split("_LOD")[-1].split(".")[0]
        deps = bpy.context.evaluated_depsgraph_get()
        frozen = bpy.data.meshes.new_from_object(mesh.evaluated_get(deps), depsgraph=deps)
        world = mesh.matrix_world.copy()
        mesh.modifiers.clear()
        mesh.data = frozen
        mesh.name = f"{name}_LOD{tier}"
        # Freeze in the parent's frame, so the gunner rotates about the
        # pedestal with the HMG while his feet stay on the floor.
        frozen.transform(parent.matrix_world.inverted() @ world)
        mesh.parent = parent
        mesh.matrix_parent_inverse = Matrix.Identity(4)
        mesh.matrix_basis = Matrix.Identity(4)
    for o in (imported - set(meshes)) | set(targets):
        bpy.data.objects.remove(o, do_unlink=True)


def preserve_materials(path):
    """Carry the shared source's exact materials and embedded pixels through
    Blender. Its export strips images for the vehicle's recipe baker; routing
    imported ORM through Blender would also lose the side-tint alpha channel.
    Geometry, UVs and vertex colours are already on the frozen crew meshes.
    """
    def read(path):
        data = open(path, "rb").read()
        length = struct.unpack_from("<I", data, 12)[0]
        doc = json.loads(data[20:20 + length])
        start = 20 + length
        size = struct.unpack_from("<I", data, start)[0]
        return doc, bytearray(data[start + 8:start + 8 + size])

    source, pixels = read(SOURCE)
    doc, binary = read(path)
    images = {}
    slots = {}
    samplers = {}
    def texture(index):
        if index in slots:
            return slots[index]
        tex = copy.deepcopy(source["textures"][index])
        image = tex["source"]
        if image not in images:
            record = copy.deepcopy(source["images"][image])
            view = source["bufferViews"][record["bufferView"]]
            offset = view.get("byteOffset", 0)
            binary.extend(b"\0" * (-len(binary) % 4))
            record["bufferView"] = len(doc["bufferViews"])
            doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": view["byteLength"]})
            binary.extend(pixels[offset:offset + view["byteLength"]])
            images[image] = len(doc.setdefault("images", []))
            doc["images"].append(record)
        tex["source"] = images[image]
        if "sampler" in tex:
            sampler = tex["sampler"]
            if sampler not in samplers:
                samplers[sampler] = len(doc.setdefault("samplers", []))
                doc["samplers"].append(source["samplers"][sampler])
            tex["sampler"] = samplers[sampler]
        slots[index] = len(doc.setdefault("textures", []))
        doc["textures"].append(tex)
        return slots[index]

    materials = {m["name"]: m for m in source["materials"]}
    for i, current in enumerate(doc["materials"]):
        original = materials.get(current["name"].split(".")[0])
        if original is None:
            continue
        material = copy.deepcopy(original)
        material["name"] = current["name"]
        for container in (material, material.get("pbrMetallicRoughness", {})):
            for key, value in container.items():
                if key.endswith("Texture"):
                    value["index"] = texture(value["index"])
        doc["materials"][i] = material
    binary.extend(b"\0" * (-len(binary) % 4))
    doc["buffers"][0]["byteLength"] = len(binary)
    encoded = json.dumps(doc, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)
    with open(path, "wb") as output:
        output.write(struct.pack("<III", 0x46546C67, 2, 28 + len(encoded) + len(binary)))
        output.write(struct.pack("<II", len(encoded), 0x4E4F534A) + encoded)
        output.write(struct.pack("<II", len(binary), 0x004E4942) + binary)
