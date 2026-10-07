"""Freeze a soldier's art into vehicle crew poses at source export.

Crew are part of the vehicle appearance, not extra simulation soldiers. The
driver follows the body; a gunner follows the mount's yaw, not barrel pitch.

- A vehicle's crew wear its faction's soldier (`soldier_source`).
- Crew are drawn from the soldier's tier 1 down: vehicle tier t carries the
  soldier's tier t + 1, and the far tier 3 carries no crew. They are the
  vehicle's own triangles, so its budget counts them.
- Crew carry no weapon: every piece of the soldier skinned wholly to the
  right hand's joint (the node his weapon and its muzzle socket ride) is cut.
- Crew materials are named `crew_<soldier material>`, and only those take the
  soldier's exact materials back after export (`preserve_materials`), so a
  vehicle material is never overwritten.
"""
import math
import os
import copy
import json
import struct
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector

REPO = Path(__file__).resolve().parents[3]
# Each faction's crew soldier, an appearance in assets/catalog.json: its army's
# rifleman (the rifle squad's faction look). Europe has none of its own yet
# (slice 17) and wears the squad's default, US Army OCP.
CREW_SOLDIER = {
    "us": "rifle_squad_active_a",
    "europe": "rifle_squad_active_a",
    "eastern": "rifle_squad_eastern_active_a",
}
CREW_TIERS = 3
WEAPON_JOINT = "hand_r"
PREFIX = "crew_"


def soldier_source(faction):
    """The source GLB of the faction's crew soldier."""
    appearances = json.loads((REPO / "assets/catalog.json").read_text())["appearances"]
    return str(REPO / appearances[CREW_SOLDIER[faction]]["source"])


def _without_weapon(mesh):
    """Cut every connected piece skinned wholly to the weapon joint."""
    bm = bmesh.new()
    bm.from_mesh(mesh.data)
    weights = bm.verts.layers.deform.active
    joint = mesh.vertex_groups[WEAPON_JOINT].index
    seen = set()
    cut = []
    for start in bm.verts:
        if start in seen:
            continue
        seen.add(start)
        piece = [start]
        stack = [start]
        while stack:
            vert = stack.pop()
            for edge in vert.link_edges:
                other = edge.other_vert(vert)
                if other not in seen:
                    seen.add(other)
                    piece.append(other)
                    stack.append(other)
        if all(v[weights].get(joint, 0) > 0.99 for v in piece):
            cut.extend(piece)
    bmesh.ops.delete(bm, geom=cut, context="VERTS")
    bm.to_mesh(mesh.data)
    bm.free()


def _crew_materials(mesh):
    """Rename the soldier's materials into the crew's namespace, sharing one
    material per name between every crewman."""
    for slot in mesh.material_slots:
        material = slot.material
        if material is None or material.name.startswith(PREFIX):
            continue
        name = PREFIX + material.name.split(".")[0]
        existing = bpy.data.materials.get(name)
        if existing is None:
            material.name = name
        else:
            slot.material = existing


def crew(name, parent, hips, hands, feet, source):
    """Pose one crewman from the soldier GLB `source` with his pelvis at
    `hips`, hands at `hands` and feet at `feet` (left, right), frozen under
    `parent` as `<name>_LOD0..2`."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=source)
    imported = set(bpy.data.objects) - before
    arm = next(o for o in imported if o.type == "ARMATURE")
    skinned = [o for o in imported if o.type == "MESH"
               and any(m.type == "ARMATURE" and m.object == arm for m in o.modifiers)]
    meshes = []
    for mesh in skinned:
        tier = int(mesh.name.split("_LOD")[-1].split(".")[0]) - 1
        if 0 <= tier < CREW_TIERS:
            meshes.append((tier, mesh))
    meshes.sort(key=lambda item: item[0])
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

    for tier, mesh in meshes:
        _without_weapon(mesh)
        _crew_materials(mesh)
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
    for o in (imported - {mesh for _, mesh in meshes}) | set(targets):
        bpy.data.objects.remove(o, do_unlink=True)


def preserve_materials(path, source):
    """Carry the crew soldier's exact materials and embedded pixels through
    Blender into the vehicle GLB at `path`. Its export strips images for the
    vehicle's recipe baker; routing imported ORM through Blender would also
    lose the side-tint alpha channel. Geometry, UVs and vertex colours are
    already on the frozen crew meshes. Only `crew_*` materials are replaced.
    """
    def read(path):
        data = open(path, "rb").read()
        length = struct.unpack_from("<I", data, 12)[0]
        doc = json.loads(data[20:20 + length])
        start = 20 + length
        size = struct.unpack_from("<I", data, start)[0]
        return doc, bytearray(data[start + 8:start + 8 + size])

    soldier, pixels = read(source)
    doc, binary = read(path)
    images = {}
    slots = {}
    samplers = {}
    def texture(index):
        if index in slots:
            return slots[index]
        tex = copy.deepcopy(soldier["textures"][index])
        image = tex["source"]
        if image not in images:
            record = copy.deepcopy(soldier["images"][image])
            view = soldier["bufferViews"][record["bufferView"]]
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
                doc["samplers"].append(soldier["samplers"][sampler])
            tex["sampler"] = samplers[sampler]
        slots[index] = len(doc.setdefault("textures", []))
        doc["textures"].append(tex)
        return slots[index]

    materials = {m["name"]: m for m in soldier["materials"]}
    for i, current in enumerate(doc["materials"]):
        if not current["name"].startswith(PREFIX):
            continue
        original = materials.get(current["name"][len(PREFIX):].split(".")[0])
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
