"""The disabled BRM and Jaguar, refused until slice 18 (the Puma is rebuilt in roster/puma.py). VBCI, Tigr-M, Fennek and VBL
are rebuilt in their own scripts (roster/vbci.py, tigr.py, fennek.py, vbl.py)."""
import bpy,bmesh,os,sys,json,math
from mathutils import Vector
sys.path.insert(0,os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
import parts as P
from catalog_frames import requested_variant
from wreckage import WRECK_ARG, burn, export_wreck

# Each family's paint scheme (`textures.SCHEMES`): its real nation's.
SCHEME={'jaguar':'french_three_tone','brm':'russian_green'}


def build(family):
    args=script_args();v,out=requested_variant(family,args);ident=v['id'];frame=v['frame'];L,W,H=frame['body_dimensions_m']
    reset();paint=P.paint(SCHEME[family],'ground_paint',chip=.25,dirt=.45,rise=.9);dark=textured('dark_steel','olive_paint',colour=(.045,.055,.035),chip=.18,dirt=.4,role='paint');rubber=tyre();steel=bare_steel('gun_steel',chip=.12,dirt=.2);glass=P.glass('armor_glass')
    root=empty(family);body=empty('body',parent=root)
    deck={'brm':1.85,'jaguar':2.36}[family]
    if family=='brm':
        r=.43;y=1.53;half=L/2-.63
        for side in (-1,1):
            for i in range(6):
                x=2.47-i*.99;wn=empty('wheel_'+('F' if i<3 else 'R')+str(i)+('L' if side>0 else 'R'),(x,side*y,r+.055),body,props={'radius_m':r});cyl('roadwheel_'+str(side)+str(i),r,.46,axis='Y',mat=rubber,parent=wn,seg=24);cyl('roadwheel_rim_'+str(side)+str(i),r*.69,.045,(0,side*.25,0),'Y',paint,wn,seg=18)
            R=.51;track=empty('track_'+('L' if side>0 else 'R'),parent=body,props={'track_length_m':4*half+math.tau*R,'link_pitch_m':.15})
            def belt(bm,lod,s=side):
                n=[18,12,8,5][lod];outer=[];inner=[]
                for x,a0 in [(half,-math.pi/2),(-half,math.pi/2)]:
                    for j in range(n+1):
                        a=a0+j*math.pi/n;outer.append((x+R*math.cos(a),R+R*math.sin(a)));inner.append((x+(R-.11)*math.cos(a),R+(R-.11)*math.sin(a)))
                rings=[[bm.verts.new((x,s*y+w,z)) for x,z in p] for w in (-.32,.32) for p in (outer,inner)];count=len(outer)
                for j in range(count):
                    k=(j+1)%count
                    for a,b in [(0,2),(1,3),(0,1),(2,3)]:bm.faces.new((rings[a][j],rings[a][k],rings[b][k],rings[b][j]))
                bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
            mesh_part('track_band_'+str(side),belt,track_steel('track',links=False),track)
        prism('puma_asymmetric_hull',[(-L/2,.76),(L/2,.83),(L/2,1.49),(L/2-1.47,deck),(-L/2+.05,deck)],3.34,mat=paint,parent=body,bevel=.035)
        for side in (-1,1):
            box('level_c_skirt_'+str(side),(L-.20,.36,.93),(0,side*1.80,1.59),paint,body,bevel=.03)
            for j in range(6):box('level_c_module_'+str(side)+str(j),(1.00,.13,.79),(-2.89+j*1.15,side*1.94,1.83),paint,body,bevel=.018,lods=(0,1,2))
        for j in range(8):box('engine_deck_grille_'+str(j),(.055,.91,.025),(1.15+j*.13,-.63,deck-.055),dark,body,lods=(0,1))
        box('rear_ramp',(.075,2.73,1.42),(-L/2-.004,0,1.63),paint,body)
    else:
        raise SystemExit(f"{family}: not drawn here (its own script, or slice 18)")
    cyl('roof_hatch',.25,.065,(-.55,-.38,deck+.025),'Z',paint,body,seg=16)
    rigs={};pivots={}
    for mount in frame['mounts']:
        gun=mount['role']=='gun';p=Vector(mount['pivot_m']);mu=Vector(mount['muzzle_m']);carrier=rigs.get(mount['on'],body);pp=pivots.get(mount['on'],Vector((0,0,0)));yaw=empty('turret' if gun else 'hmg',p-pp,carrier);rigs[mount['name']]=yaw;pivots[mount['name']]=p;pitch=empty('gun' if gun else 'hmg_gun',(0,0,mu.z),yaw);empty('muzzle' if gun else 'hmg_muzzle',(mu.x,mu.y,0),pitch)
        if mount['name']=='launcher':
            box('mells_launcher_box',(1.42,.69,.48),(mu.x-.71,0,0),paint,pitch,bevel=.035)
            for side in (-1,1):cyl('mells_launch_port_'+str(side),.11,.03,(mu.x-.014,side*.15,0),'X',dark,pitch,seg=16)
            box('mells_cover',(1.43,.70,.045),(mu.x-.71,0,.264),paint,pitch)
        elif gun:
            if family=='brm':
                low=2.58-p.z
                prism('puma_unmanned_turret',[(-1.03,low),(.94,low),(.78,low+.30),(.19,low+.50),(-.89,low+.50)],1.94,mat=paint,parent=yaw,bevel=.035)
                cyl('puma_muss_pedestal',.13,.24,(-.50,-.45,3.29-p.z),'Z',paint,yaw,seg=16)
                box('puma_muss_head',(.30,.32,.26),(-.50,-.45,3.48-p.z),glass,yaw,bevel=.015)
                box('puma_primary_sight',(.30,.26,.23),(.69,-.52,2.91-p.z),glass,yaw)
            box('gun_mantlet',(.40,.39,.32),(.69,0,0),paint,pitch,bevel=.035)
            start=.80;cyl('cannon_barrel',.048,mu.x-start,((mu.x+start)/2,0,0),'X',steel,pitch,seg=16);cyl('cannon_muzzle',.085,.14,(mu.x-.07,0,0),'X',dark,pitch,seg=12)
            if family=='brm':
                box('puma_barrel_guard',(mu.x-.84,.17,.13),((mu.x+.84)/2,0,-.10),paint,pitch)
                for j in range(10):cyl('guard_vent_'+str(j),.034,.012,(.98+j*(mu.x-1.10)/10,-.094,-.10),'Y',dark,pitch,seg=8,lods=(0,1))
        else:
            base=deck-p.z;top=mu.z-.10;cyl('hmg_pedestal',.13,max(.10,top-base),(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            box('hmg_receiver',(.61,.18,.19),(.43,0,0),dark,pitch);cyl('hmg_barrel',.038,mu.x-.70,((mu.x+.70)/2,0,0),'X',steel,pitch,seg=12);box('hmg_ammo_box',(.31,.29,.28),(.22,.27,-.06),paint,pitch)
    wreck=WRECK_ARG in args
    if wreck:burn()
    finish(ao_distance=.65,ao_rays=4);print('ROSTER_REMAINING_GROUND',json.dumps({'id':ident,'tris':triangles_by_tier()}));os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);(export_wreck if wreck else export)(out,texture_px=256)
    preview=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if preview and not wreck:
        bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=os.path.abspath(out));os.makedirs(preview,exist_ok=True)
        for o in bpy.data.objects:
            if o.type=='MESH':o.hide_render=('_LOD' in o.name and not o.name.endswith('_LOD0'))
        scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=960;scene.render.resolution_y=640;scene.render.resolution_percentage=100;scene.world=bpy.data.worlds.new('review_world');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7;bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.012))
        for loc,power in [((3,-6,12),1800),((-5,5,7),1100)]:
            bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.size=8;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
        bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.ortho_scale=L*1.35
        for label,loc in [('front',(L*1.5,-L*1.8,L*.85)),('side',(0,-L*2,L*.55)),('rear',(-L*1.6,L*1.7,L*.8))]:
            cam.location=loc;cam.rotation_euler=(Vector((0,0,H*.45))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=os.path.join(preview,label+'.png');bpy.ops.render.render(write_still=True)
