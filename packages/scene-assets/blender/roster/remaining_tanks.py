"""Type 15, T-14 and T-15 named-source authoring (legacy; rebuilt families leave it)."""
import bpy,bmesh,os,sys,json,math
from mathutils import Vector
sys.path.insert(0,os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
import parts as P
from catalog_frames import requested_variant
from wreckage import WRECK_ARG, burn, export_wreck

# Each family's paint scheme (`textures.SCHEMES`): its real nation's.
SCHEME={'type15':'chinese_digital','t14':'russian_green','t15':'russian_green'}


def build(family):
    args=script_args();v,out=requested_variant(family,args);ident=v['id']
    frame=v['frame'];L,W,H=frame['body_dimensions_m'];reset()
    paint=P.paint(SCHEME[family],'tank_paint',chip=.3,dirt=.4,rise=1.1);dark=textured('chassis_dark','olive_paint',colour=(.045,.055,.035),chip=.22,dirt=.5,role='paint');rubber=tyre();steel=bare_steel('barrel_steel',chip=.15,dirt=.2);glass=P.glass('optic_glass')
    root=empty(family);body=empty('body',parent=root);count=7 if family in ('t14','t15') else 6;r=.43 if family in ('t14','t15') else .40;half=L/2-.60;track_y=W/2-.35;deck={'t14':1.78,'t15':1.72,'type15':1.46}[family]
    for side in (-1,1):
        for i in range(count):
            x=L*.34-i*(L*.68/(count-1));wn=empty('wheel_'+('F' if i<count/2 else 'R')+str(i)+('L' if side>0 else 'R'),(x,side*track_y,r+.075),body,props={'radius_m':r});cyl('roadwheel_'+str(side)+str(i),r,.50,axis='Y',mat=rubber,parent=wn,seg=24);cyl('wheel_disc_'+str(side)+str(i),r*.74,.055,(0,side*.27,0),'Y',paint,wn,seg=20);cyl('wheel_hub_'+str(side)+str(i),r*.22,.08,(0,side*.29,0),'Y',dark,wn,seg=12)
            for j in range(6):
                a=j*math.tau/6;cyl('wheel_bolt_'+str(side)+str(i)+str(j),.022,.025,(r*.42*math.cos(a),side*.308,r*.42*math.sin(a)),'Y',steel,wn,seg=6,lods=(0,))
        R=r+.10;track=empty('track_'+('L' if side>0 else 'R'),parent=body,props={'track_length_m':4*half+math.tau*R,'link_pitch_m':.16})
        def belt(bm,lod,s=side):
            n=[20,12,8,5][lod];outer=[];inner=[]
            for x,a0 in [(half,-math.pi/2),(-half,math.pi/2)]:
                for j in range(n+1):
                    a=a0+j*math.pi/n;outer.append((x+R*math.cos(a),R+R*math.sin(a)));inner.append((x+(R-.12)*math.cos(a),R+(R-.12)*math.sin(a)))
            rings=[[bm.verts.new((x,s*track_y+w,z)) for x,z in p] for w in (-.30,.30) for p in (outer,inner)];cnt=len(outer)
            for j in range(cnt):
                k=(j+1)%cnt
                for a,b in [(0,2),(1,3),(0,1),(2,3)]:bm.faces.new((rings[a][j],rings[a][k],rings[b][k],rings[b][j]))
            bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
        mesh_part('track_band_'+str(side),belt,track_steel('track',links=False),track)
    # Explicit hull proportion and glacis differ, while running gear is shared.
    nose=L/2;rear=-L/2
    profile=[(rear,.72),(nose,.78),(nose,1.10),(nose-1.24,deck),(rear+.04,deck)]
    if family in ('t14','t15'):profile=[(rear,.78),(nose,.82),(nose,1.20),(nose-1.58,deck),(rear+.03,deck)]
    elif family=='type15':profile=[(rear,.66),(nose,.72),(nose,1.04),(nose-1.32,deck),(rear+.04,deck)]
    prism(family+'_hull',profile,W-.50,mat=paint,parent=body,bevel=.035)
    for side in (-1,1):
        skirt_h=.62;skirt_z=1.08
        box(family+'_side_skirt_'+str(side),(L-.20,.24,skirt_h),(0,side*(W/2-.12),skirt_z),paint,body,bevel=.025)
        for j in range(6):
            width=(L-.36)/6-.055;x=-L/2+.18+(j+.5)*(L-.36)/6
            box('side_armor_module_'+str(side)+str(j),(width,.10,skirt_h-.065),(x,side*(W/2+.002),skirt_z+.02),paint,body,bevel=.015,lods=(0,1,2))
            for k in (-1,1):cyl('armor_bolt_'+str(side)+str(j)+str(k),.023,.028,(x+k*width*.33,side*(W/2+.057),skirt_z+.24),'Y',steel,body,seg=6,lods=(0,))
        box('front_headlight_'+str(side),(.06,.23,.12),(L/2+.011,side*(W*.32),1.11),glass,body)
        cyl('tow_eye_'+str(side),.065,.07,(L/2+.025,side*.69,.83),'X',dark,body,seg=12,lods=(0,1,2))
    for j in range(9):box('engine_vent_'+str(j),(.06,W*.48,.025),(-L/2+.41+j*.13,0,deck+.016),dark,body,lods=(0,1))
    box('rear_engine_panel',(.055,W-.65,.57),(-L/2-.013,0,1.15),paint,body)
    cyl('driver_hatch',.25,.065,(L/2-1.48,-.26,deck+.025),'Z',paint,body,seg=16)
    for j in range(3):box('driver_vision_'+str(j),(.11,.13,.055),(L/2-1.17,-.46+j*.18,deck-.033),glass,body,lods=(0,1,2))
    rigs={};pivots={}
    for mount in frame['mounts']:
        gun=mount['role']=='gun';p=Vector(mount['pivot_m']);mu=Vector(mount['muzzle_m']);carrier=rigs.get(mount['on'],body);pp=pivots.get(mount['on'],Vector((0,0,0)));yaw=empty('turret' if gun else 'hmg',p-pp,carrier);rigs[mount['name']]=yaw;pivots[mount['name']]=p;pitch=empty('gun' if gun else 'hmg_gun',(0,0,mu.z),yaw);empty('muzzle' if gun else 'hmg_muzzle',(mu.x,mu.y,0),pitch)
        if gun:
            lower=[(-1.68,-1.40),(1.29,-1.40),(1.65,-.67),(1.65,.67),(1.29,1.40),(-1.68,1.40)];upper=[(-1.42,-1.04),(.96,-1.04),(1.30,-.54),(1.30,.54),(.96,1.04),(-1.42,1.04)];th=.67
            loft(family+'_named_turret',[(.03,lower),(th,upper)],paint,yaw,bevel=.035)
            box('ztz99a_commander_viewer',(.26,.26,.20),(-.40,.57,.86),glass,yaw)
            for side in (-1,1):
                prism('ztz99a_wedge_era_'+str(side),[(.20,.25),(1.67,.23),(1.62,.60),(.67,.77),(.20,.63)],.65,loc=(0,side*.90,0),mat=paint,parent=yaw,bevel=.018)
                for j in range(5):box('ztz99a_era_tile_'+str(side)+str(j),(.23,.025,.24),(.42+j*.24,side*1.232,.53),paint,yaw,lods=(0,1,2))
            cyl('commander_hatch',.25,.065,(-.42,-.44,th+.032),'Z',paint,yaw,seg=16)
            box('gunner_sight',(.29,.23,.19),(.58,-.47,th+.035),glass,yaw)
            for side in (-1,1):
                for j in range(4):cyl('smoke_launcher_'+str(side)+str(j),.052,.22,(-.39+j*.14,side*1.36,.36),'X',dark,yaw,seg=10,lods=(0,1,2))
            start=1.20;rad=.091
            box('main_mantlet',(.54,.57,.45),(1.24,0,0),paint,pitch,bevel=.035)
            cyl('main_barrel',rad,mu.x-start,((mu.x+start)/2,0,0),'X',paint,pitch,seg=20)
            cyl('bore_evacuator',rad*1.72,.66,(mu.x*.52,0,0),'X',paint,pitch,seg=20)
            cyl('muzzle_ring',rad*1.15,.11,(mu.x-.055,0,0),'X',dark,pitch,seg=16)
            for j in range(3):cyl('thermal_sleeve_band_'+str(j),rad*1.13,.035,(1.65+j*.73,0,0),'X',dark,pitch,seg=16,lods=(0,1,2))
        else:
            base=-.14;top=mu.z-.095;cyl('roof_weapon_pedestal',.15,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            box('roof_weapon_receiver',(.62,.19,.19),(.43,0,0),dark,pitch)
            cyl('roof_weapon_barrel',.028,mu.x-.70,((mu.x+.70)/2,0,0),'X',steel,pitch,seg=12)
            box('roof_weapon_ammo',(.32,.27,.26),(.24,.27,-.055),paint,pitch)
    wreck=WRECK_ARG in args
    if wreck:burn()
    finish(ao_distance=.7,ao_rays=4);print('ROSTER_REMAINING_TANK',json.dumps({'id':ident,'tris':triangles_by_tier()}));os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);(export_wreck if wreck else export)(out,texture_px=256)
    preview=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if preview and not wreck:
        bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=os.path.abspath(out));os.makedirs(preview,exist_ok=True)
        for o in bpy.data.objects:
            if o.type=='MESH':o.hide_render=('_LOD' in o.name and not o.name.endswith('_LOD0'))
        scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=960;scene.render.resolution_y=640;scene.render.resolution_percentage=100;scene.world=bpy.data.worlds.new('review_world');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7;bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.012))
        for loc,power in [((3,-6,12),1800),((-5,5,7),1100)]:
            bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.size=8;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
        bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.ortho_scale=max(L*1.4,frame['mounts'][0]['muzzle_m'][0]+L/2+.6)
        for label,loc in [('front',(L*1.5,-L*1.8,L*.85)),('side',(0,-L*2,L*.55)),('rear',(-L*1.6,L*1.7,L*.8))]:
            cam.location=loc;cam.rotation_euler=(Vector((.50,0,H*.45))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=os.path.join(preview,label+'.png');bpy.ops.render.render(write_still=True)
