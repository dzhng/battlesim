"""Named BMP authoring; the resolved catalog owns every mount frame. BTR and ZBL-08 are
rebuilt in their own scripts (roster/btr.py, roster/zbl08.py)."""
import bpy,bmesh,os,sys,json,math
from mathutils import Vector
sys.path.insert(0,os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
import parts as P
from catalog_frames import requested_variant
from wreckage import WRECK_ARG, burn, export_wreck

# Each family's paint scheme (`textures.SCHEMES`): its real nation's.
SCHEME={'bmp':'russian_green'}


def build(family):
    args=script_args();v,out=requested_variant(family,args);ident=v['id']
    frame=v['frame'];L,W,H=frame['body_dimensions_m'];bmp3=ident.endswith('_bmp_3')
    reset();paint=P.paint(SCHEME[family],'named_paint',chip=.28,dirt=.4,rise=.9);dark=textured('dark_steel','olive_paint',colour=(.045,.055,.035),chip=.2,dirt=.4,role='paint');rubber=tyre();steel=bare_steel('barrel_steel',chip=.12,dirt=.25);glass=P.glass('optics')
    root=empty(family);body=empty('body',parent=root)
    if family=='bmp':
        deck=1.60 if bmp3 else 1.58
        # BMP-2 has a pointed flat wedge; BMP-3 has a broader vertical boat stern.
        profile=[(-L/2,.58),(L/2,.62),(L/2,1.03),(L/2-1.48,deck),(-L/2+.06,deck)]
        prism('bmp3_boat_hull' if bmp3 else 'bmp2_wedge_hull',profile,W-.55,mat=paint,parent=body,bevel=.025)
        for side in (-1,1):
            y=side*(W/2-.27)
            for i in range(6):
                x=L*.33-i*(L*.66/5);r=.35 if bmp3 else .34
                wn=empty('wheel_'+('F' if i<3 else 'R')+str(i)+('L' if side>0 else 'R'),(x,y,.43),body,props={'radius_m':r})
                cyl('roadwheel_'+str(side)+str(i),r,.36,axis='Y',mat=rubber,parent=wn,seg=24)
                cyl('roadwheel_rim_'+str(side)+str(i),r*.72,.045,(0,side*.19,0),'Y',paint,wn,seg=18)
            tx=L/2-.62;track=empty('track_'+('L' if side>0 else 'R'),parent=body,props={'track_length_m':4*tx+math.tau*.42,'link_pitch_m':.14})
            def belt(bm,lod,s=side,t=tx):
                n=[18,12,8,5][lod];outer=[];inner=[]
                for x,a0 in [(t,-math.pi/2),(-t,math.pi/2)]:
                    for j in range(n+1):
                        a=a0+j*math.pi/n;outer.append((x+.42*math.cos(a),.42+.42*math.sin(a)));inner.append((x+.32*math.cos(a),.42+.32*math.sin(a)))
                rings=[[bm.verts.new((x,s*(W/2-.27)+y,z)) for x,z in p] for y in (-.24,.24) for p in (outer,inner)];count=len(outer)
                for j in range(count):
                    k=(j+1)%count
                    for a,b in [(0,2),(1,3),(0,1),(2,3)]:bm.faces.new((rings[a][j],rings[a][k],rings[b][k],rings[b][j]))
                bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
            mesh_part('track_band_'+str(side),belt,track_steel('track',links=False),track)
            box('track_fender_'+str(side),(L-.22,.32,.11),(0,side*(W/2-.14),1.08),paint,body,bevel=.025)
            for j in range(5):box('fender_rib_'+str(side)+str(j),(.04,.36,.24),(-2.40+j*1.15,side*(W/2-.16),1.20),paint,body,lods=(0,1,2))
        if bmp3:
            # Rear engine deck, twin stern hatches and rear ventilation louvers.
            for side in (-1,1):
                box('stern_door_'+str(side),(.06,.86,.86),(-L/2-.003,side*.49,1.09),paint,body,bevel=.012)
                for j in range(5):box('rear_vent_'+str(side)+str(j),(.025,.57,.035),(-L/2-.036,side*.91,.78+j*.075),dark,body,lods=(0,1))
            for j in range(8):box('engine_grille_'+str(j),(.05,1.53,.025),(-2.87+j*.15,0,deck+.018),dark,body,lods=(0,1))
        else:
            for side in (-1,1):
                box('rear_exit_door_'+str(side),(.085,1.13,.87),(-L/2+.015,side*.60,1.04),paint,body,bevel=.045)
                cyl('door_port_'+str(side),.12,.025,(-L/2-.04,side*.60,1.27),'X',dark,body,seg=12)
    for j,x in enumerate((1.11,-1.43)):
        cyl('deck_hatch_'+str(j),.27,.065,(x,.45,deck+.022),'Z',paint,body,seg=16)
    rigs={};pivots={}
    for m in frame['mounts']:
        main=m['role']=='gun';pivot=Vector(m['pivot_m']);carrier=rigs.get(m['on'],body);parentpivot=pivots.get(m['on'],Vector((0,0,0)))
        yaw=empty('turret' if main else 'hmg',pivot-parentpivot,carrier);rigs[m['name']]=yaw;pivots[m['name']]=pivot;mu=Vector(m['muzzle_m'])
        pitch=empty('gun' if main else 'hmg_gun',(0,0,mu.z),yaw);empty('muzzle' if main else 'hmg_muzzle',(mu.x,mu.y,0),pitch)
        if m['name']=='launcher':
            # Berezhok has two Kornet tubes on each turret flank; one physical pack.
            for side in (-1,1):
                y=side*.851-pivot.y
                box('kornet_cradle_'+str(side),(1.57,.46,.095),(.29,y,-.19),paint,pitch)
                for tube in (-1,1):
                    cyl('kornet_tube_'+str(side)+str(tube),.12,1.52,(mu.x-.76,y+tube*.135,0),'X',paint,pitch,seg=16)
                    cyl('kornet_opening_'+str(side)+str(tube),.09,.025,(mu.x-.012,y+tube*.135,0),'X',dark,pitch,seg=12)
        else:
            if main:
                if family=='bmp':
                    cyl('bmp_turret_ring',.83,.15,(0,0,.04),'Z',paint,yaw,seg=32)
                    cyl('bmp_turret_body',.78,.47,(0,0,.34),'Z',paint,yaw,seg=32,r2=.64,bevel=.045)
                    cyl('bmp_turret_roof',.69,.075,(0,0,.615),'Z',paint,yaw,seg=24)
                    cyl('commander_hatch',.235,.06,(-.29,-.32,.68),'Z',paint,yaw,seg=16)
                    box('commander_viewer',(.22,.22,.17),(-.29,-.32,.77),glass,yaw)
                for side in (-1,1):
                    for j in range(3):cyl('smoke_tube_'+str(side)+str(j),.047,.20,(-.41+j*.14,side*.69,.48),'X',dark,yaw,seg=10,lods=(0,1,2))
            large=bmp3 and main;start=.68;rad=.095 if large else .047
            box('gun_mantlet' if main else 'autocannon_attachment',(.40,.35 if large else .21,.29),(.58,0,0),paint,pitch,bevel=.02)
            cyl('gun_barrel' if main else 'distinct_30mm_barrel',rad,mu.x-start,((mu.x+start)/2,0,0),'X',steel,pitch,seg=16)
            cyl('barrel_sleeve' if main else '30mm_sleeve',rad*1.48,.62,(1.01,0,0),'X',paint,pitch,seg=16)
            cyl('muzzle_brake' if main else '30mm_muzzle_brake',rad*1.35,.13,(mu.x-.065,0,0),'X',dark,pitch,seg=12)
    wreck=WRECK_ARG in args
    if wreck:burn()
    finish(ao_distance=.65,ao_rays=4);print('ROSTER_EAST_ARMOR',json.dumps({'id':ident,'tris':triangles_by_tier()}));os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);(export_wreck if wreck else export)(out,texture_px=256)
    preview=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if preview and not wreck:
        bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=os.path.abspath(out));os.makedirs(preview,exist_ok=True)
        for o in bpy.data.objects:
            if o.type=='MESH':o.hide_render=('_LOD' in o.name and not o.name.endswith('_LOD0'))
        scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=960;scene.render.resolution_y=640;scene.render.resolution_percentage=100;scene.world=bpy.data.worlds.new('review_world');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7
        bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.012))
        for loc,power in [((3,-6,12),1800),((-5,5,7),1100)]:
            bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.size=8;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
        bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.ortho_scale=L*1.35
        for label,loc in [('front',(L*1.5,-L*1.8,L*.85)),('side',(0,-L*2,L*.55)),('rear',(-L*1.6,L*1.7,L*.8))]:
            cam.location=loc;cam.rotation_euler=(Vector((0,0,H*.45))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=os.path.join(preview,label+'.png');bpy.ops.render.render(write_still=True)
