import bpy, json, os, sys, math
from mathutils import Vector
ROOT=os.path.abspath(os.path.join(os.path.dirname(__file__),'../../../..'))
MANIFEST=os.path.join(ROOT,'fixtures/units/model-manifest.json')
OUT=os.path.join(ROOT,'assets/source/roster/disabled')

def mat(name,color):
    m=bpy.data.materials.new(name); m.diffuse_color=(*color,1); m.use_nodes=True
    bsdf=m.node_tree.nodes.get('Principled BSDF')
    if bsdf:
        bsdf.inputs['Base Color'].default_value=(*color,1)
        bsdf.inputs['Roughness'].default_value=.78
        bsdf.inputs['Metallic'].default_value=.12 if name != 'sensor_glass' else .35
    return m

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
        # A common aircraft root keeps cards cheap, while the silhouette branches
        # make fighters, strike aircraft, transports and drones readable at a
        # glance. These are disabled source models, so visual identity is the
        # contract; flight and weapon mechanics remain deferred.
        if 'drone' in eid or 'reaper' in eid:
            cube('drone_body',(0,0,0.95),(2.2,.48,.30),base); cube('drone_wing',(-.15,0,1.0),(.65,3.0,.08),base)
            cube('drone_tail',(-.92,0,1.22),(.36,.10,.62),base); cyl('drone_sensor',(0,0,0.72),.13,.26,glass)
        elif any(k in eid for k in ('f_22','f_35','j_20','su_57')):
            cube('stealth_fuselage',(0,0,.78),(4.5,.58,.48),base); cube('stealth_wing',(.1,0,.72),(2.0,3.7,.10),base)
            cube('stealth_tail',(-1.65,.48,1.2),(.48,.10,.82),base,0); cube('stealth_tail',(-1.65,-.48,1.2),(.48,.10,.82),base,0)
            cube('stealth_nose',(2.0,0,.80),(1.0,.42,.34),base)
        elif any(k in eid for k in ('su_34','su_25','a_10','f_15','f_a_18','rafale','mirage','tornado','eurofighter','gripen','j_10','mig_29','mig_31')):
            cube('fighter_fuselage',(0,0,.76),(4.4,.62,.52),base); cube('fighter_nose',(2.0,0,.76),(1.0,.5,.4),base)
            cube('fighter_wing',(.1,0,.72),(2.3,3.2,.10),base); cube('fighter_tail',(-1.65,0,1.2),(.45,.12,1.0),base)
            cyl('engine',(-1.4,.38,.72),.18,.9,dark,rot=(math.pi/2,0,0)); cyl('engine',(-1.4,-.38,.72),.18,.9,dark,rot=(math.pi/2,0,0))
            if 'su_34' in eid or 'a_10' in eid: cube('strike_store',(.15,0,.48),(1.4,1.3,.16),dark)
        else:
            cube('transport_fuselage',(0,0,.95),(4.8,1.05,.95),base); cube('transport_nose',(2.2,0,.95),(1.0,.9,.7),base)
            cube('transport_wing',(.1,0,1.0),(2.0,4.4,.12),base); cube('transport_tail',(-1.9,0,1.55),(.5,.16,1.2),base)
            cyl('engine',(-.8,.78,.9),.22,.8,dark,rot=(math.pi/2,0,0)); cyl('engine',(-.8,-.78,.9),.22,.8,dark,rot=(math.pi/2,0,0))
        if 'growler' in eid or 'ecr' in eid:
            cube('pod_left',(-.15,.7,.65),(1.1,.18,.22),dark); cube('pod_right',(-.15,-.7,.65),(1.1,.18,.22),dark)
    elif kind=='rotorcraft':
        cube('helicopter_body',(0,0,.9),(2.7,1.05,1.05),base); cube('tail',(-1.75,0,1.0),(2.0,.22,.25),base); cyl('main_rotor',(0,0,1.65),.08,4.2,dark,rot=(0,math.pi/2,0)); cube('rotor_blade',(0,0,1.7),(.2,4.4,.06),dark,0); cyl('tail_rotor',(-2.55,0,1.0),.06,.8,dark,rot=(math.pi/2,0,0)); cube('cockpit',(1.0,0,1.0),(.6,.85,.5),glass,.02)
        if 'chinook' in eid:
            cube('cargo_body',(-.45,0,.92),(1.9,1.25,1.1),base); cube('ramp',(-1.45,0,.62),(.35,1.0,.55),dark)
            cyl('rear_rotor',(-1.0,0,1.7),.07,3.3,dark,rot=(0,math.pi/2,0))
        elif any(k in eid for k in ('transport','nh90','aw101','mi_8','z_20','uh_1y','uh_60')):
            cube('cargo_body',(-.45,0,.92),(1.8,1.2,1.0),base); cube('ramp',(-1.35,0,.62),(.35,1.0,.55),dark); cube('side_door',(.1,.62,.9),(.65,.05,.65),dark,0)
        elif any(k in eid for k in ('attack','tiger','apache','ka_52','mi_24','mi_28','z_10','aw159','ah_1z')):
            cube('gunship_nose',(.85,0,.78),(1.1,.72,.65),base); cube('weapon_stub',(.85,0,.58),(1.0,.12,.12),dark)
            cube('stub_left',(.1,.72,.75),(.9,.14,.14),dark); cube('stub_right',(.1,-.72,.75),(.9,.14,.14),dark)
        elif 'ah_6' in eid or 'mh_6' in eid:
            cube('light_body',(.2,0,.78),(1.5,.62,.6),base); cube('skid_left',(-.1,.5,.38),(1.5,.08,.08),dark,0); cube('skid_right',(-.1,-.5,.38),(1.5,.08,.08),dark,0)
    elif kind=='support':
        cube('support_hull',(0,0,.8),(3.2,1.7,1.0),base); cube('deck',(-.1,0,1.45),(1.7,1.4,.35),base)
        if 'kamikaze_drone' in eid:
            cube('launch_box',(-.25,0,1.75),(1.25,1.1,.55),base); cube('drone_boom',(1.1,0,2.2),(1.0,.12,.12),dark); cube('drone_wing',(.2,0,2.15),(.5,1.4,.06),base)
        elif any(k in eid for k in ('air_defense','buk','nasams','pantsir','gepard','shorad')):
            cyl('turret',(.45,0,1.8),.5,.35,base); cube('launcher',(.95,0,2.15),(1.5,.18,.18),dark,.02)
            cube('radar_mast',(-.5,0,2.35),(.16,.16,1.3),dark); cyl('radar',(-.5,0,3.0),.35,.12,glass,rot=(0,math.pi/2,0))
        elif any(k in eid for k in ('howitzer','artillery','msta','caesar','pzh','himars','mlrs','tornado','mars','grad','plz')):
            cube('armored_cab',(-.9,0,1.95),(1.0,1.2,1.0),base); cube('long_gun',(1.15,0,2.05),(2.0,.16,.16),dark,.02)
            if any(k in eid for k in ('himars','mlrs','tornado','mars','grad')):
                cube('rocket_pod',(.45,0,2.0),(1.2,1.0,.45),dark)
        else:
            cyl('turret',(.45,0,1.8),.5,.35,base); cube('launcher',(.95,0,2.15),(1.5,.18,.18),dark,.02)
    elif kind=='ground_vehicle':
        if 'drone' in eid or any(k in eid for k in ('orlan','scout_drone')):
            cube('drone_body',(0,0,1.0),(1.5,.48,.28),base); cube('drone_wing',(-.1,0,1.03),(.55,2.2,.08),base); cube('drone_tail',(-.65,0,1.24),(.25,.08,.5),base); cyl('sensor',(0,0,.78),.12,.2,glass)
        elif any(k in eid for k in ('t_14','challenger_3','type_99','t_90','t_80','t_72','leopard','leclerc','kf51')):
            cube('tank_hull',(0,0,.8),(3.4,1.8,1.0),base); cube('tank_turret',(.25,0,1.55),(1.5,1.45,.55),base); cube('tank_gun',(1.65,0,1.62),(2.2,.14,.14),dark,0.02)
        elif 't_15' in eid:
            cube('ifv_hull',(-.2,0,.85),(3.5,1.9,1.2),base); cube('ifv_turret',(.45,0,1.7),(1.15,1.25,.5),base); cube('ifv_gun',(1.4,0,1.8),(1.3,.12,.12),dark,0.02); cube('rear_ramp',(-1.8,0,.8),(.12,1.2,.75),dark,0)
        elif 'type_15' in eid:
            cube('light_hull',(0,0,.7),(2.9,1.55,.9),base); cube('light_turret',(.2,0,1.35),(1.2,1.1,.45),base); cube('light_gun',(1.4,0,1.42),(1.7,.12,.12),dark,0.02)
        elif any(k in eid for k in ('brm','jaguar','reconnaissance')):
            cube('recon_hull',(0,0,.7),(2.9,1.5,.9),base); cube('recon_sensor',(.1,0,1.5),(.7,.7,.35),glass); cube('recon_mast',(-.55,0,1.75),(.08,.08,.75),dark)
        else:
            cube('scout_hull',(0,0,.7),(2.6,1.35,.9),base); cyl('wheel',(-.75,.73,.45),.36,.22,dark,rot=(math.pi/2,0,0)); cyl('wheel',(.75,.73,.45),.36,.22,dark,rot=(math.pi/2,0,0)); cube('sensor',(.15,0,1.35),(.55,.45,.3),glass); cube('mast',(-.55,0,1.65),(.08,.08,.8),dark)
    else:
        cube('infantry_rig',(0,0,1.0),(.35,.22,1.5),base)
        if any(k in eid for k in ('javelin','akeron')):
            cube('atgm_tube',(.1,-.22,1.15),(.18,1.15,.18),dark,.02); cube('launch_pack',(-.18,.18,1.05),(.35,.22,.55),base)
        elif any(k in eid for k in ('stinger','igla','fn_6')):
            cube('manpads_tube',(.08,-.2,1.35),(.16,1.0,.16),dark,.02); cube('grip',(.08,-.2,1.05),(.12,.18,.25),dark,.01)
        else:
            cube('weapon',(0,-.2,1.0),(.12,.9,.12),dark,.02)
    for obj in list(bpy.context.scene.objects):
        if obj != root:
            obj.parent = root
    os.makedirs(OUT,exist_ok=True); path=os.path.join(OUT,e['id']+'.glb'); bpy.ops.export_scene.gltf(filepath=path,export_format='GLB',export_apply=True)
    return path

m=json.load(open(MANIFEST))
for e in m['entries']:
    build(e)
print('EXPORTED',len(m['entries']))
