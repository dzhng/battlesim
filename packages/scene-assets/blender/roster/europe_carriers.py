"""ACV, Ajax, Boxer and CV90: named hulls and hardware at their catalog frames."""
import bpy,bmesh,os,sys,json,math
from mathutils import Vector
sys.path.insert(0,os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
import parts as P
from catalog_frames import requested_variant

# Each family's paint scheme (`textures.SCHEMES`): its real nation's.
SCHEME={'acv':'us_desert_tan','ajax':'british_green','boxer':'german_three_tone','cv90':'swedish_splinter'}


def build(family):
    args=script_args();v,out=requested_variant(family,args);ident=v['id'];frame=v['frame'];L,W,H=frame['body_dimensions_m'];mkiv=ident.endswith('mk_iv');rct=ident.endswith('rct30')
    reset();paint=P.paint(SCHEME[family],'carrier_paint',chip=.25,dirt=.4,rise=1.2);dark=textured('dark_armor','olive_paint',colour=(.045,.055,.035),chip=.2,dirt=.4,role='paint');rubber=tyre();steel=bare_steel('gun_steel',chip=.12,dirt=.2);glass=P.glass('optics')
    root=empty(family);body=empty('body',parent=root)
    tracked=family in ('ajax','cv90');deck={'acv':2.82,'ajax':2.02,'boxer':2.31,'cv90':1.86}[family]
    if tracked:
        r=.43 if family=='ajax' else .37;y=W/2-.31;half=L/2-.62
        for side in (-1,1):
            for i in range(7):
                x=L*.34-i*(L*.68/6);wn=empty('wheel_'+('F' if i<3 else 'R')+str(i)+('L' if side>0 else 'R'),(x,side*y,r+.06),body,props={'radius_m':r});cyl('roadwheel_'+str(side)+str(i),r,.42,axis='Y',mat=rubber,parent=wn,seg=24);cyl('roadwheel_rim_'+str(side)+str(i),r*.69,.05,(0,side*.23,0),'Y',paint,wn,seg=18)
            radius=r+.08;track=empty('track_'+('L' if side>0 else 'R'),parent=body,props={'track_length_m':4*half+math.tau*radius,'link_pitch_m':.15})
            def belt(bm,lod,s=side,t=half,R=radius):
                n=[18,12,8,5][lod];outer=[];inner=[]
                for x,a0 in [(t,-math.pi/2),(-t,math.pi/2)]:
                    for j in range(n+1):
                        a=a0+j*math.pi/n;outer.append((x+R*math.cos(a),R+R*math.sin(a)));inner.append((x+(R-.10)*math.cos(a),R+(R-.10)*math.sin(a)))
                rings=[[bm.verts.new((x,s*y+w,z)) for x,z in p] for w in (-.28,.28) for p in (outer,inner)];count=len(outer)
                for j in range(count):
                    k=(j+1)%count
                    for a,b in [(0,2),(1,3),(0,1),(2,3)]:bm.faces.new((rings[a][j],rings[a][k],rings[b][k],rings[b][j]))
                bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
            mesh_part('track_band_'+str(side),belt,dark,track)
        front=L/2;rear=-L/2
        prism('ajax_tall_hull' if family=='ajax' else 'cv90_wedge_hull',[(rear,.72),(front,.82),(front,1.16),(front-1.30,deck),(rear+.06,deck)],W-.55,mat=paint,parent=body,bevel=.035)
        for side in (-1,1):
            box('modular_skirt_'+str(side),(L-.24,.22,.63),(0,side*(W/2-.10),1.26 if family=='ajax' else 1.12),paint,body,bevel=.025)
            count=7 if family=='ajax' else 6
            for j in range(count):box('armor_panel_'+str(side)+str(j),((L-.55)/count-.06,.055,.60),(-L/2+.42+j*(L-.55)/count,side*(W/2+.012),1.40 if family=='ajax' else 1.24),paint,body,bevel=.008,lods=(0,1,2))
        box('rear_exit',(.07,W-.72,1.02),(-L/2-.005,0,1.27),paint,body,bevel=.02)
        for j in range(8):box('engine_deck_grille_'+str(j),(.055,.73,.025),(1.28+j*.13,-.54,deck-.08),dark,body,lods=(0,1))
    else:
        r=.64 if family=='acv' else .59;xs=[L*.34,L*.115,-L*.12,-L*.34]
        for i,x in enumerate(xs):
            for side in (-1,1):
                row=('F' if i<2 else 'R')+str(i);wn=empty('wheel_'+row+('L' if side>0 else 'R'),(x,side*(W/2-.23),r),body,props={'radius_m':r});cyl('tire_'+row+str(side),r,.43,axis='Y',mat=rubber,parent=wn,seg=28);cyl('rim_'+row+str(side),r*.62,.055,(0,side*.222,0),'Y',paint,wn,seg=20);cyl('hub_'+row+str(side),r*.20,.08,(0,side*.25,0),'Y',dark,wn,seg=12)
                for j in range(16):
                    a=j*math.tau/16;box('tread_'+row+str(side)+str(j),(.15,.38,.024),((r-.012)*math.sin(a),0,(r-.012)*math.cos(a)),rubber,wn,rot=(0,a,.16*side),lods=(0,1))
        if family=='acv':
            # High seaworthy-shaped exterior, ground combat only.
            def acv_boat(bm,lod):
                profile=[(-L/2,.91),(L/2-.12,.98),(L/2,1.71),(L/2-1.20,deck),(-L/2+.03,deck)];widths=[W*.29,W*.29,W*.35,W*.43,W*.43]
                rings=[[bm.verts.new((x,side*w,z)) for (x,z),w in zip(profile,widths)] for side in (-1,1)]
                bm.faces.new(rings[0][::-1]);bm.faces.new(rings[1])
                for j in range(5):
                    k=(j+1)%5;bm.faces.new((rings[0][j],rings[0][k],rings[1][k],rings[1][j]))
                bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
            mesh_part('acv_high_boat_hull',acv_boat,paint,body)
            box('acv_bow_plate',(.075,W-.63,.79),(L/2-.52,0,2.13),paint,body,rot=(0,-.80,0),bevel=.02)
            for side in (-1,1):
                box('acv_upper_side_plate_'+str(side),(L-1.41,.12,.55),(-.59,side*(W/2-.11),2.33),paint,body,bevel=.02)
                for j in range(5):box('acv_panel_seam_'+str(side)+str(j),(.035,.03,.54),(-3.38+j*1.18,side*(W/2-.038),2.33),dark,body,lods=(0,1))
        else:
            prism('boxer_drive_module',[(-L/2,.82),(L/2-.13,.87),(L/2,1.17),(L/2-1.25,2.20),(-L/2+.04,1.62)],W-.40,mat=paint,parent=body,bevel=.035)
            # Distinct removable rear mission module and its seam.
            box('boxer_mission_module',(5.17,W-.36,.71),(-1.25,0,deck-.355),paint,body,bevel=.045)
            box('boxer_module_front_seam',(.055,W-.31,.64),(1.34,0,deck-.35),dark,body)
            box('driver_cab',(.88,.81,.37),(2.08,.58,2.03),paint,body,bevel=.025)
            for side in (-1,1):
                for j in range(4):box('boxer_side_panel_'+str(side)+str(j),(1.04,.055,.55),(-3.09+j*1.25,side*(W/2-.14),deck-.30),paint,body,bevel=.014,lods=(0,1,2))
        box('rear_ramp',(.07,W-.67,1.36),(-L/2-.006,0,1.63),paint,body,bevel=.02)
        for side in (-1,1):
            box('front_lamp_cluster_'+str(side),(.065,.25,.13),(L/2-.12,side*(W*.32),1.28),glass,body)
            box('mirror_'+str(side),(.12,.14,.30),(L/2-1.26,side*(W/2-.07),2.08),dark,body,lods=(0,1,2))
    # Driver vision, forward lights and access handles survive silhouette tiers.
    box('driver_hatch',(.50,.53,.055),(L/2-1.56,.55,deck+.015),paint,body,bevel=.018)
    for j in range(3):box('driver_vision_'+str(j),(.10,.14,.06),(L/2-1.24,.36+j*.19,deck-.025),glass,body,lods=(0,1,2))
    for side in (-1,1):
        if tracked:box('tracked_headlamp_'+str(side),(.08,.19,.13),(L/2-.03,side*(W*.32),1.17),glass,body)
        cyl('front_tow_eye_'+str(side),.063,.075,(L/2-.028,side*.66,1.01),'X',dark,body,seg=12,lods=(0,1,2))
        box('rear_lamp_'+str(side),(.045,.15,.085),(-L/2-.04,side*(W*.31),1.11),glass,body,lods=(0,1,2))
    for j,x in enumerate((.97,-1.48)):

        cyl('roof_hatch_'+str(j),.27,.07,(x,.45,deck+.03),'Z',paint,body,seg=16)
    for mount in frame['mounts']:
        gun=mount['role']=='gun';p=Vector(mount['pivot_m']);mu=Vector(mount['muzzle_m']);yaw=empty('turret' if gun else 'hmg',p,body);pitch=empty('gun' if gun else 'hmg_gun',(0,0,mu.z),yaw);empty('muzzle' if gun else 'hmg_muzzle',(mu.x,mu.y,0),pitch)
        if gun:
            if family=='ajax':
                profile=[(-1.10,.04),(.91,.04),(.94,.35),(.45,.77),(-.91,.77)];width=1.81
                prism('ajax_high_angular_turret',profile,width,mat=paint,parent=yaw,bevel=.035)
                box('ajax_panorama_head',(.32,.32,.19),(-.53,-.46,.95),glass,yaw)
                box('ajax_turret_stowage',(.42,1.54,.30),(-1.23,0,.38),dark,yaw)
            elif family=='cv90':
                profile=[(-1.05,.02),(.90,.02),(.83,.36),(.31,.70),(-.89,.69)] if mkiv else [(-.85,.02),(.87,.02),(.74,.35),(.29,.71),(-.74,.64)];width=1.75 if mkiv else 1.61
                prism('cv90_d_series_turret' if mkiv else 'cv9040c_bofors_turret',profile,width,mat=paint,parent=yaw,bevel=.035)
                if mkiv:
                    box('d_series_panorama_base',(.25,.25,.14),(-.47,-.51,.78),paint,yaw)
                    box('d_series_panorama_head',(.29,.27,.17),(-.47,-.51,.935),glass,yaw)
                    for side in (-1,1):box('d_series_cheek_'+str(side),(.72,.14,.38),(.17,side*.89,.31),paint,yaw,bevel=.025)
                else:box('bofors_sight',(.30,.25,.21),(-.38,-.38,.81),glass,yaw)
            else:
                prism('rct30_unmanned_turret',[(-1.01,.02),(.90,.02),(.78,.43),(.14,.74),(-.83,.74)],1.86,mat=paint,parent=yaw,bevel=.035)
                box('rct30_front_sight',(.32,.28,.24),(.63,-.50,.65),glass,yaw)
                box('rct30_side_equipment',(.76,.42,.55),(-.58,-1.02,.43),paint,yaw,bevel=.025)
                cyl('rct30_panorama_pedestal',.14,.33,(-.47,-.38,.83),'Z',paint,yaw,seg=16)
                box('rct30_panorama_head',(.32,.34,.29),(-.47,-.38,1.10),glass,yaw,bevel=.018)
                box('rct30_heat_shield',(mu.x-.92,.18,.14),((mu.x+.92)/2,0,mu.z-.11),paint,yaw)
                for j in range(9):box('rct30_shroud_vent_'+str(j),(.11,.012,.07),(1.05+j*(mu.x-1.17)/9,-.10,mu.z-.10),dark,yaw,lods=(0,1))
            for side in (-1,1):
                for j in range(3):cyl('smoke_tube_'+str(side)+str(j),.046,.20,(-.30+j*.15,side*.75,.41),'X',dark,yaw,seg=10,lods=(0,1,2))
            box('mantlet',(.42,.40,.37),(.78,0,0),paint,pitch,bevel=.035)
            start=.88;rad=.068 if family=='cv90' and not mkiv else .061
            cyl('cannon_barrel',rad,mu.x-start,((mu.x+start)/2,0,0),'X',steel,pitch,seg=16)
            cyl('cannon_sleeve',rad*1.55,.60,(1.18,0,0),'X',paint,pitch,seg=16)
            cyl('bofors_muzzle' if family=='cv90' and not mkiv else 'cannon_muzzle',rad*1.45,.18,(mu.x-.09,0,0),'X',dark,pitch,seg=12)
        else:
            base=deck-p.z;top=mu.z-.10
            cyl('remote_pedestal',.20,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            box('hmg_receiver',(.61,.20,.20),(.43,0,0),dark,pitch)
            cyl('hmg_barrel',.038,mu.x-.70,((mu.x+.70)/2,0,0),'X',steel,pitch,seg=12)
            box('ammo_box',(.33,.32,.29),(.26,.31,-.07),paint,pitch)
            box('remote_optics',(.26,.26,.22),(.30,-.26,.16),glass,pitch)
    finish(ao_distance=.7,ao_rays=4);print('ROSTER_CARRIER',json.dumps({'id':ident,'tris':triangles_by_tier()}));os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);export(out,texture_px=256)
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
