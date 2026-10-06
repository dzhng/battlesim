"""Named scouts and IFVs, exclusive to the remaining-ground model worker."""
import bpy,bmesh,os,sys,json,math
from mathutils import Vector
sys.path.insert(0,os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
ROOT=os.path.abspath(os.path.join(os.path.dirname(__file__),'../../../..'))

def build(family):
    args=script_args();out=next(a for a in args if not a.startswith('--'));ident=os.path.basename(out).removesuffix('.glb');m=json.load(open(os.path.join(ROOT,'specs/unit-roster/manifests',family+'.json')));v=next(v for v in m['variants'] if v['id']==ident);frame=v['physical_authoring'];L,W,H=frame['body_dimensions_m']
    reset();paint=textured('ground_olive','olive_paint',tint=1,chip=.25,dirt=.45,rise=.9);dark=textured('dark_steel','olive_paint',colour=(.045,.055,.035),chip=.18,dirt=.4);rubber=textured('rubber','rubber',chip=0,dirt=.3);steel=textured('gun_steel','bare_steel',chip=.12,dirt=.2);glass=flat_paint('armor_glass',(.018,.055,.063),rough=.15,grime=.1)
    root=empty(family);body=empty('body',parent=root)
    scout=family in ('fennek','vbl','tigr');deck={'fennek':1.86,'vbl':1.64,'tigr':1.94,'puma':2.56,'vbci':2.36}[family]
    if family=='puma':
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
            mesh_part('track_band_'+str(side),belt,dark,track)
        prism('puma_asymmetric_hull',[(-L/2,.76),(L/2,.83),(L/2,1.49),(L/2-1.47,deck),(-L/2+.05,deck)],3.34,mat=paint,parent=body,bevel=.035)
        for side in (-1,1):
            box('level_c_skirt_'+str(side),(L-.20,.36,.93),(0,side*1.80,1.59),paint,body,bevel=.03)
            for j in range(6):box('level_c_module_'+str(side)+str(j),(1.00,.13,.79),(-2.89+j*1.15,side*1.94,1.83),paint,body,bevel=.018,lods=(0,1,2))
        for j in range(8):box('engine_deck_grille_'+str(j),(.055,.91,.025),(1.15+j*.13,-.63,deck-.055),dark,body,lods=(0,1))
        box('rear_ramp',(.075,2.73,1.42),(-L/2-.004,0,1.63),paint,body)
    else:
        r={'fennek':.49,'vbl':.36,'tigr':.52,'vbci':.59}[family];depth=.36 if family=='vbl' else .43;axles=[L*.31,-L*.31] if scout else [L*.33,L*.112,-L*.115,-L*.33]
        for i,x in enumerate(axles):
            for side in (-1,1):
                row=('F' if i<len(axles)/2 else 'R')+str(i);wn=empty('wheel_'+row+('L' if side>0 else 'R'),(x,side*(W/2-depth/2-.025),r),body,props={'radius_m':r});cyl('tire_'+row+str(side),r,depth,axis='Y',mat=rubber,parent=wn,seg=28);cyl('rim_'+row+str(side),r*.61,.055,(0,side*(depth/2+.015),0),'Y',paint,wn,seg=20);cyl('hub_'+row+str(side),r*.20,.08,(0,side*(depth/2+.041),0),'Y',dark,wn,seg=12)
                for j in range(16):
                    a=j*math.tau/16;box('tread_'+row+str(side)+str(j),(.12,depth*.87,.024),((r-.012)*math.sin(a),0,(r-.012)*math.cos(a)),rubber,wn,rot=(0,a,.16*side),lods=(0,1))
        if family=='fennek':
            prism('fennek_low_wedge',[(-L/2,.76),(L/2,.73),(L/2,1.02),(1.48,1.24),(.92,deck),(-1.12,deck),(-1.53,1.57),(-L/2,1.57)],W-.23,mat=paint,parent=body,bevel=.035)
            for side in (-1,1):
                box('fennek_windscreen_'+str(side),(.03,.89,.47),(1.15,side*.49,1.60),glass,body,rot=(0,-.68,0))
                box('fennek_side_window_'+str(side),(.65,.033,.31),(.34,side*(W/2-.10),1.58),glass,body,bevel=.015)
                box('fennek_door_'+str(side),(.88,.028,.49),(.27,side*(W/2-.085),1.18),paint,body,bevel=.015)
            cyl('packed_sensor_base',.15,.26,(-.76,.57,1.91),'Z',paint,body,seg=16)
            box('packed_sensor_head',(.31,.28,.26),(-.76,.57,2.16),glass,body,bevel=.018)
            for j in range(6):box('rear_engine_vent_'+str(j),(.04,1.18,.02),(-2.41+j*.15,0,1.582),dark,body,lods=(0,1))
        elif family=='vbl':
            prism('vbl_short_hull',[(-L/2,.49),(L/2,.48),(L/2,.81),(.81,1.11),(.39,deck),(-L/2+.13,deck)],W-.26,mat=paint,parent=body,bevel=.025)
            for side in (-1,1):
                box('vbl_windscreen_'+str(side),(.024,.60,.30),(.622,side*.33,1.37),glass,body,rot=(0,-.67,0))
                box('vbl_door_'+str(side),(1.09,.025,.55),(-.43,side*(W/2-.12),.94),paint,body,bevel=.015)
                box('vbl_side_window_'+str(side),(.39,.027,.23),(-.29,side*(W/2-.10),1.39),glass,body,bevel=.013)
                for j in (-1,1):box('vbl_hood_louver_'+str(side)+str(j),(.38,.16,.027),(1.14,side*.43+j*.055,1.012),dark,body,lods=(0,1))
        elif family=='tigr':
            prism('tigr_m_armored_cab',[(-L/2+.08,.68),(1.05,.68),(1.01,1.62),(.76,deck),(-L/2+.12,deck)],W-.24,mat=paint,parent=body,bevel=.035)
            prism('tigr_squared_hood',[(.89,.84),(L/2-.03,.84),(L/2-.03,1.28),(.93,1.40)],W-.39,mat=paint,parent=body,bevel=.055)
            for side in (-1,1):
                box('tigr_windscreen_'+str(side),(.032,.76,.36),(.958,side*.42,1.73),glass,body,rot=(0,-.66,0))
                for j,x in enumerate((.12,-1.04)):
                    box('tigr_door_'+str(side)+str(j),(1.08,.03,.68),(x,side*(W/2-.10),1.08),paint,body,bevel=.02)
                    box('tigr_side_window_'+str(side)+str(j),(.65,.03,.29),(x,side*(W/2-.08),1.66),glass,body,bevel=.02)
                box('tigr_mirror_'+str(side),(.10,.15,.25),(.80,side*(W/2+.015),1.64),dark,body,lods=(0,1,2))
                box('tigr_step_'+str(side),(2.28,.18,.085),(-.49,side*(W/2-.025),.70),dark,body)
            # Tigr-M bonnet shoulders and front brush guard are distinct from the VBL.
            for side in (-1,1):
                box('tigr_front_fender_'+str(side),(1.40,.32,.10),(L*.31,side*(W/2-.09),1.20),paint,body,bevel=.03)
                cyl('brush_guard_upright_'+str(side),.047,.66,(L/2+.026,side*.79,1.04),'Z',dark,body,seg=12,lods=(0,1,2))
            cyl('brush_guard_crossbar',.047,1.65,(L/2+.026,0,1.35),'Y',dark,body,seg=12,lods=(0,1,2))
            spare=empty('rear_spare',(-L/2+.14,-.38,1.30),body);cyl('spare_tire',.48,.25,axis='X',mat=rubber,parent=spare,seg=24);cyl('spare_hub',.24,.28,axis='X',mat=paint,parent=spare,seg=16)
            box('tigr_grille',(.035,1.00,.30),(L/2,0,1.12),dark,body)
            for j in range(7):box('tigr_grille_bar_'+str(j),(.045,.035,.28),(L/2+.016,-.42+j*.14,1.12),paint,body,lods=(0,1))
        else:
            prism('vbci_tall_hull',[(-L/2,.84),(L/2-.13,.76),(L/2,1.19),(L/2-1.16,deck),(-L/2+.04,deck)],W-.38,mat=paint,parent=body,bevel=.035)
            for side in (-1,1):
                for j in range(5):box('vbci_side_armor_'+str(side)+str(j),(1.00,.065,.48),(-2.74+j*1.14,side*(W/2-.155),2.06),paint,body,bevel=.015,lods=(0,1,2))
            box('rear_ramp',(.07,W-.66,1.35),(-L/2-.004,0,1.60),paint,body)
        for side in (-1,1):
            box('front_bumper_'+str(side),(.13,.71,.12),(L/2-.026,side*(W*.27),.72),dark,body)
            cyl('headlamp_'+str(side),.09,.04,(L/2-.022,side*(W*.34),.95 if scout else 1.26),'X',glass,body,seg=16)
    cyl('roof_hatch',.25,.065,(-.55,-.38,deck+.025),'Z',paint,body,seg=16)
    rigs={};pivots={}
    for mount in frame['mounts']:
        gun=mount['role']=='gun';p=Vector(mount['pivot_m']);mu=Vector(mount['muzzle_m']);carrier=rigs.get(mount['on'],body);pp=pivots.get(mount['on'],Vector((0,0,0)));yaw=empty('turret' if gun else 'hmg',p-pp,carrier);rigs[mount['name']]=yaw;pivots[mount['name']]=p;pitch=empty('gun' if gun else 'hmg_gun',(0,0,mu.z),yaw);empty('muzzle' if gun else 'hmg_muzzle',(mu.x,mu.y,0),pitch)
        if mount['name']=='launcher':
            box('mells_launcher_box',(1.42,.69,.48),(mu.x-.71,0,0),paint,pitch,bevel=.035)
            for side in (-1,1):cyl('mells_launch_port_'+str(side),.11,.03,(mu.x-.014,side*.15,0),'X',dark,pitch,seg=16)
            box('mells_cover',(1.43,.70,.045),(mu.x-.71,0,.264),paint,pitch)
        elif gun:
            if family=='puma':
                low=2.58-p.z
                prism('puma_unmanned_turret',[(-1.03,low),(.94,low),(.78,low+.30),(.19,low+.50),(-.89,low+.50)],1.94,mat=paint,parent=yaw,bevel=.035)
                cyl('puma_muss_pedestal',.13,.24,(-.50,-.45,3.29-p.z),'Z',paint,yaw,seg=16)
                box('puma_muss_head',(.30,.32,.26),(-.50,-.45,3.48-p.z),glass,yaw,bevel=.015)
                box('puma_primary_sight',(.30,.26,.23),(.69,-.52,2.91-p.z),glass,yaw)
            else:
                low=deck-p.z
                cyl('vbci_one_man_turret',.61,.48,(0,0,low+.27),'Z',paint,yaw,seg=24,r2=.45,bevel=.025)
                box('vbci_sight',(.27,.22,.19),(.05,-.31,2.88-p.z),glass,yaw)
                cyl('vbci_commander_hatch',.235,.07,(-.18,-.12,2.87-p.z),'Z',paint,yaw,seg=16)
            box('gun_mantlet',(.40,.39,.32),(.69,0,0),paint,pitch,bevel=.035)
            start=.80;cyl('cannon_barrel',.060 if family=='puma' else .048,mu.x-start,((mu.x+start)/2,0,0),'X',steel,pitch,seg=16);cyl('cannon_muzzle',.085,.14,(mu.x-.07,0,0),'X',dark,pitch,seg=12)
            if family=='puma':
                box('puma_barrel_guard',(mu.x-.84,.17,.13),((mu.x+.84)/2,0,-.10),paint,pitch)
                for j in range(10):cyl('guard_vent_'+str(j),.034,.012,(.98+j*(mu.x-1.10)/10,-.094,-.10),'Y',dark,pitch,seg=8,lods=(0,1))
        else:
            base=deck-p.z;top=mu.z-.10;cyl('hmg_pedestal',.13,max(.10,top-base),(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            box('hmg_receiver',(.61,.18,.19),(.43,0,0),dark,pitch);cyl('hmg_barrel',.038,mu.x-.70,((mu.x+.70)/2,0,0),'X',steel,pitch,seg=12);box('hmg_ammo_box',(.31,.29,.28),(.22,.27,-.06),paint,pitch)
            if family=='tigr':box('kord_muzzle_brake',(.17,.071,.065),(mu.x-.085,0,0),dark,pitch)
            if family in ('vbl','tigr'):box('gunner_front_shield',(.065,.69,.31),(.77,0,.03),paint,yaw,bevel=.015)
    finish(ao_distance=.65,ao_rays=4);print('ROSTER_REMAINING_GROUND',json.dumps({'id':ident,'tris':triangles_by_tier()}));os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);export(out,texture_px=256)
    preview=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if preview:
        bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=os.path.abspath(out));os.makedirs(preview,exist_ok=True)
        for o in bpy.data.objects:
            if o.type=='MESH':o.hide_render=('_LOD' in o.name and not o.name.endswith('_LOD0'))
        scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=960;scene.render.resolution_y=640;scene.render.resolution_percentage=100;scene.world=bpy.data.worlds.new('review_world');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7;bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.012))
        for loc,power in [((3,-6,12),1800),((-5,5,7),1100)]:
            bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.size=8;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
        bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.ortho_scale=L*1.35
        for label,loc in [('front',(L*1.5,-L*1.8,L*.85)),('side',(0,-L*2,L*.55)),('rear',(-L*1.6,L*1.7,L*.8))]:
            cam.location=loc;cam.rotation_euler=(Vector((0,0,H*.45))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=os.path.join(preview,label+'.png');bpy.ops.render.render(write_still=True)
