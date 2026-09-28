import bpy, math, sys, os
from mathutils import Vector
tag = sys.argv[sys.argv.index('--')+1]; out = sys.argv[sys.argv.index('--')+2]
sc = bpy.context.scene
# unlit interiors: zero any lamp/light/brightness/emission modifier inputs
for o in bpy.data.objects:
  for m in o.modifiers:
    if m.type=='NODES' and m.node_group:
      for it in m.node_group.interface.items_tree:
        if getattr(it,'in_out','')!='INPUT': continue
        n = it.name.lower()
        try: has = it.identifier in m
        except TypeError: has = False
        if any(k in n for k in ('lamp','lit','light','bright','emiss','glow','neon')) and has:
          v = m[it.identifier]
          if isinstance(v,bool): m[it.identifier]=False
          elif isinstance(v,(int,float)): m[it.identifier]=type(v)(0)
          else: continue
          print("ZERO", o.name, it.name, v)
for mat in bpy.data.materials:
  if not mat.node_tree: continue
  for nd in mat.node_tree.nodes:
    if nd.type=='BSDF_PRINCIPLED' and not nd.inputs['Emission Strength'].is_linked: nd.inputs['Emission Strength'].default_value=0
    if nd.type=='EMISSION' and not nd.inputs['Strength'].is_linked: nd.inputs['Strength'].default_value=0
    if nd.type=='BSDF_PRINCIPLED' and nd.inputs['Emission Strength'].is_linked:
      mat.node_tree.links.remove(nd.inputs['Emission Strength'].links[0]); nd.inputs['Emission Strength'].default_value=0
dg = bpy.context.evaluated_depsgraph_get()
lo = Vector((1e9,)*3); hi = Vector((-1e9,)*3)
for inst in dg.object_instances:
  ob = inst.object
  if ob.type!='MESH': continue
  ws=[inst.matrix_world @ Vector(c) for c in ob.bound_box]
  if max(w.z for w in ws)-min(w.z for w in ws) < 1.5 or max(w.x for w in ws)-min(w.x for w in ws) > 60: continue
  for w in ws: lo = Vector(map(min, lo, w)); hi = Vector(map(max, hi, w))
lo.x=max(lo.x,-80); lo.y=max(lo.y,-80); hi.x=min(hi.x,80); hi.y=min(hi.y,80); hi.z=min(hi.z,80)
ctr = (lo+hi)/2; h = hi.z
print("BOUNDS", tag, [round(x,1) for x in lo], [round(x,1) for x in hi])
cam = bpy.data.objects.new("RTSCam", bpy.data.cameras.new("RTSCam")); sc.collection.objects.link(cam); sc.camera = cam
cam.data.sensor_fit='VERTICAL'; cam.data.angle_y=0.8; cam.data.clip_end=5000
sc.render.resolution_x=1280; sc.render.resolution_y=720; sc.render.resolution_percentage=100
sc.render.image_settings.file_format='JPEG'
def pitch_for(d):
  curve=[(25,0.22),(65,0.85),(2000,0.85)]
  if d<=25: return 0.22
  for (d0,p0),(d1,p1) in zip(curve,curve[1:]):
    if d<=d1: t=math.log(d/d0)/math.log(d1/d0); return p0+(p1-p0)*t
  return 0.85
yaw = math.radians(-135)
for d in (30, 80, 250):
  p = pitch_for(d); tgt = Vector((ctr.x, ctr.y, h*0.35 if d<60 else 0))
  dirv = Vector((math.cos(p)*math.cos(yaw), math.cos(p)*math.sin(yaw), math.sin(p)))
  cam.location = tgt + dirv*d
  cam.rotation_euler = (tgt-cam.location).to_track_quat('-Z','Y').to_euler()
  sc.render.filepath = os.path.join(out, f"{tag}_{d:03d}m.jpg")
  bpy.ops.render.render(write_still=True); print("WROTE", sc.render.filepath, "pitch", round(p,2))
