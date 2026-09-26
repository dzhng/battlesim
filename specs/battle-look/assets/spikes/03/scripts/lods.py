import bpy, os, json
blend = os.path.join(os.path.dirname(os.path.abspath(__file__)), "out", "rifleman.blend")
res = {}
for ratio in (1.0, 0.4, 0.15, 0.05):
    bpy.ops.wm.open_mainfile(filepath=blend)
    tot = 0
    for o in bpy.data.objects:
        if o.type != "MESH" or o.name in ("Eyes", "Eyebrows"):
            continue
        if ratio < 1.0:
            d = o.modifiers.new("lod", "DECIMATE")
            d.ratio = ratio
            # keep decimation before the armature modifier
            while o.modifiers.find("lod") > 0:
                bpy.context.view_layer.objects.active = o
                with bpy.context.temp_override(object=o):
                    bpy.ops.object.modifier_move_up(modifier="lod")
    dg = bpy.context.evaluated_depsgraph_get()
    for o in bpy.data.objects:
        if o.type == "MESH" and o.name not in ("Eyes", "Eyebrows"):
            me = o.evaluated_get(dg).to_mesh()
            me.calc_loop_triangles()
            tot += len(me.loop_triangles)
            o.evaluated_get(dg).to_mesh_clear()
    res[ratio] = tot
print("LODS", json.dumps(res))
