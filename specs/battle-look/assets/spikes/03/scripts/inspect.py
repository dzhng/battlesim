import bpy, sys, json

path = sys.argv[sys.argv.index("--") + 1]
bpy.ops.wm.read_factory_settings(use_empty=True)
if path.endswith(".fbx"):
    bpy.ops.import_scene.fbx(filepath=path)
else:
    bpy.ops.import_scene.gltf(filepath=path)
out = {"objects": [], "actions": []}
for o in bpy.data.objects:
    e = {"name": o.name, "type": o.type, "parent": o.parent.name if o.parent else None}
    if o.type == "MESH":
        me = o.data
        me.calc_loop_triangles()
        e["tris"] = len(me.loop_triangles)
        e["verts"] = len(me.vertices)
        e["groups"] = len(o.vertex_groups)
        e["materials"] = [m.name for m in me.materials if m]
        e["dims"] = list(o.dimensions)
    if o.type == "ARMATURE":
        e["bones"] = [(b.name, b.parent.name if b.parent else None, [round(v, 3) for v in b.head_local]) for b in o.data.bones]
        e["matrix_world"] = [list(r) for r in o.matrix_world]
    out["objects"].append(e)
for a in bpy.data.actions:
    out["actions"].append((a.name, [round(x, 1) for x in a.frame_range]))
json.dump(out, open(sys.argv[sys.argv.index("--") + 2], "w"), indent=1)
