import bpy, json, os, sys, math
from mathutils import Vector
ROOT=os.path.abspath(os.path.join(os.path.dirname(__file__),'../../../..'))
MANIFEST=os.path.join(ROOT,'fixtures/units/model-manifest.json')
OUT=os.path.join(ROOT,'assets/source/roster/disabled')

def mat(name,color):
    m=bpy.data.materials.new(name); m.diffuse_color=(*color,1); return m

def cube(name, loc, scale, material, bevel=0.04):
    bpy.ops.mesh.primitive_cube_add(location=loc); o=bpy.context.object; o.name=name; o.scale=(scale[0]/2,scale[1]/2,scale[2]/2); bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    if bevel:
        b=o.modifiers.new('edge_softening','BEVEL'); b.width=bevel; b.segments=2
    o.data.materials.append(material); return o

def cyl(name, loc, radius, depth, material, rot=(0,0,0)):
    bpy.ops.mesh.primitive_cylinder_add(vertices=16,radius=radius,depth=depth,location=loc,rotation=rot); o=bpy.context.object; o.name=name; o.data.materials.append(material); return o

def build(e):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    faction=e['id'].split('_')[0]
    palette={'us':(0.20,0.28,0.17),'europe':(0.28,0.32,0.24),'eastern':(0.34,0.20,0.12)}
    base=mat('faction_finish',palette.get(faction,(.25,.25,.25))); dark=mat('dark_steel',(.035,.045,.04)); glass=mat('sensor_glass',(.03,.12,.16))
    kind=e['source_kind']
    bpy.ops.object.empty_add(type='PLAIN_AXES', location=(0,0,0)); root=bpy.context.object; root.name='disabled_model_root'
    eid=e['id']
    if kind=='aircraft':
        cube('fuselage',(0,0,0.75),(4.4,.62,.52),base); cube('nose',(2.0,0,.75),(1.0,.5,.4),base); cube('wing',(.2,0,.72),(2.2,3.0,.10),base); cube('tail',(-1.65,0,1.2),(.45,.12,1.0),base); cyl('engine',(-1.4,.38,.72),.18,.9,dark,rot=(math.pi/2,0,0)); cyl('engine',(-1.4,-.38,.72),.18,.9,dark,rot=(math.pi/2,0,0))
        # Distinct silhouettes for drones and electronic/strike aircraft.
        if 'drone' in eid:
            cube('drone_body',(0,0,0.95),(1.45,.65,.28),base); cube('drone_wing',(-.15,0,1.0),(.7,2.1,.08),base)
        elif 'growler' in eid or 'ecr' in eid:
            cube('pod_left',(-.15,.7,.65),(1.1,.18,.22),dark); cube('pod_right',(-.15,-.7,.65),(1.1,.18,.22),dark)
    elif kind=='rotorcraft':
        cube('helicopter_body',(0,0,.9),(2.7,1.05,1.05),base); cube('tail',(-1.75,0,1.0),(2.0,.22,.25),base); cyl('main_rotor',(0,0,1.65),.08,4.2,dark,rot=(0,math.pi/2,0)); cube('rotor_blade',(0,0,1.7),(.2,4.4,.06),dark,0); cyl('tail_rotor',(-2.55,0,1.0),.06,.8,dark,rot=(math.pi/2,0,0)); cube('cockpit',(1.0,0,1.0),(.6,.85,.5),glass,.02)
        if 'transport' in eid or 'chinook' in eid or 'nh90' in eid or 'aw101' in eid:
            cube('cargo_body',(-.45,0,.92),(1.6,1.15,1.0),base); cube('ramp',(-1.35,0,.62),(.35,1.0,.55),dark)
        elif 'attack' in eid or 'tiger' in eid or 'apache' in eid or 'ka_52' in eid or 'mi_28' in eid:
            cube('weapon_stub',(.85,0,.72),(1.0,.12,.12),dark); cube('stub_left',(.1,.65,.75),(.8,.12,.12),dark)
    elif kind=='support':
        cube('support_hull',(0,0,.8),(3.2,1.7,1.0),base); cube('deck',(-.1,0,1.45),(1.7,1.4,.35),base); cyl('turret',(.45,0,1.8),.5,.35,base); cube('launcher',(.95,0,2.15),(1.5,.18,.18),dark,.02)
        if 'drone' in eid:
            cube('drone_boom',(1.1,0,2.2),(1.0,.12,.12),dark); cube('drone_wing',(.2,0,2.15),(.5,1.4,.06),base)
        elif 'air_defense' in eid or 'buk' in eid or 'nasams' in eid or 'pantsir' in eid:
            cube('radar_mast',(-.5,0,2.35),(.16,.16,1.3),dark); cyl('radar',(-.5,0,3.0),.35,.12,glass,rot=(0,math.pi/2,0))
        elif 'howitzer' in eid or 'artillery' in eid or 'msta' in eid or 'caesar' in eid or 'himars' in eid or 'mlrs' in eid:
            cube('long_gun',(1.15,0,2.05),(2.0,.16,.16),dark,.02)
    elif kind=='ground_vehicle':
        cube('scout_hull',(0,0,.7),(2.6,1.35,.9),base); cyl('wheel',(-.75,.73,.45),.36,.22,dark,rot=(math.pi/2,0,0)); cyl('wheel',(.75,.73,.45),.36,.22,dark,rot=(math.pi/2,0,0)); cube('sensor',(.15,0,1.35),(.55,.45,.3),glass); cube('mast',(-.55,0,1.65),(.08,.08,.8),dark)
    else:
        cube('infantry_rig',(0,0,1.0),(.35,.22,1.5),base); cube('weapon',(0,-.2,1.0),(.12,.9,.12),dark,.02)
    for obj in list(bpy.context.scene.objects):
        if obj != root:
            obj.parent = root
    os.makedirs(OUT,exist_ok=True); path=os.path.join(OUT,e['id']+'.glb'); bpy.ops.export_scene.gltf(filepath=path,export_format='GLB',export_apply=True)
    return path

m=json.load(open(MANIFEST))
for e in m['entries']:
    build(e)
print('EXPORTED',len(m['entries']))
