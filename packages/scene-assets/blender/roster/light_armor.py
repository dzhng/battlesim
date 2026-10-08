"""Named US light armor (Bradley, LAV): shared running gear, explicit family hulls and mount hardware.
Stryker and HMMWV are rebuilt in their own scripts (roster/stryker.py, roster/humvee.py)."""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
import parts as P
from catalog_frames import requested_variant
from wreckage import WRECK_ARG, burn, export_wreck


# Each family's paint scheme (`textures.SCHEMES`): its real nation's.
SCHEME={'bradley':'us_desert_tan','lav':'us_desert_tan'}

def build(family):
    args=script_args(); variant,out=requested_variant(family,args)
    ident=variant['id']
    frame=variant['frame']; L,W,H=frame['body_dimensions_m']
    reset()
    paint=P.paint(SCHEME[family],'armor_paint',chip=.25,dirt=.45,rise=1.1)
    dark=textured('dark_steel','olive_paint',colour=(.045,.055,.035),chip=.2,dirt=.5,role='paint')
    rubber=tyre()
    steel=bare_steel('gun_steel',chip=.15,dirt=.2)
    glass=P.glass('optics')
    root=empty(family); body=empty('body',parent=root)

    def wheel(row,x,side,r,y,depth):
        node=empty('wheel_'+row+('L' if side>0 else 'R'),(x,side*y,r),body,props={'radius_m':r})
        cyl('tire_'+row+str(side),r,depth,axis='Y',mat=rubber,parent=node,seg=28)
        cyl('rim_'+row+str(side),r*.56,.06,(0,side*(depth/2+.005),0),'Y',paint,node,seg=20)
        cyl('hub_'+row+str(side),r*.21,.09,(0,side*(depth/2+.035),0),'Y',dark,node,seg=12)
        for j in range(16):
            a=j*math.tau/16
            box('tread_'+row+str(side)+'_'+str(j),(.15,depth*.88,.025),((r-.012)*math.sin(a),0,(r-.012)*math.cos(a)),rubber,node,rot=(0,a,.18*side),lods=(0,1))

    if family=='bradley':
        # Six exposed road wheels, continuous closed track belt and armored skirts.
        for side in (-1,1):
            for i,x in enumerate([2.18,1.31,.44,-.43,-1.30,-2.17]):
                wn=empty('wheel_'+('F' if i<3 else 'R')+str(i)+('L' if side>0 else 'R'),(x,side*1.43,.48),body,props={'radius_m':.39})
                cyl('roadwheel_'+str(side)+'_'+str(i),.39,.43,axis='Y',mat=rubber,parent=wn,seg=24)
                cyl('roadwheel_hub_'+str(side)+'_'+str(i),.25,.05,(x,side*1.66,.48),'Y',paint,body,seg=18)
            for x in (-2.65,2.65):cyl('endwheel_'+str(side)+'_'+str(x),.39,.42,(x,side*1.43,.67),'Y',dark,body,seg=24)
            track=empty('track_'+('L' if side>0 else 'R'),parent=body,props={'track_length_m':12.5,'link_pitch_m':.15})
            # Ring between nested rounded-ended profiles, with lower shoe at ground.
            def belt(bm,lod,s=side):
                n=[18,12,8,5][lod]; outer=[]; inner=[]
                for center,start in [(2.65,-math.pi/2),(-2.65,math.pi/2)]:
                    for j in range(n+1):
                        a=start+j*math.pi/n
                        outer.append((center+.46*math.cos(a),.46+.46*math.sin(a)))
                        inner.append((center+.35*math.cos(a),.46+.35*math.sin(a)))
                rings=[[bm.verts.new((x,s*1.43+y,z)) for x,z in p] for y in (-.28,.28) for p in (outer,inner)]
                count=len(outer)
                for j in range(count):
                    k=(j+1)%count
                    for a,b in [(0,2),(1,3),(0,1),(2,3)]:bm.faces.new((rings[a][j],rings[a][k],rings[b][k],rings[b][j]))
                bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
            mesh_part('track_band_'+str(side),belt,track_steel('track',links=False),track)
        prism('bradley_hull',[(-L/2,.66),(L/2,.78),(L/2,1.20),(2.10,1.98),(-2.80,2.07),(-L/2,1.84)],3.03,mat=paint,parent=body,bevel=.035)
        for side in (-1,1):
            box('armored_skirt_'+str(side),(5.9,.19,.70),(-.10,side*1.70,1.22),paint,body,bevel=.035)
            for j in range(6):box('skirt_segment_'+str(side)+'_'+str(j),(.84,.045,.58),(-2.45+j*.98,side*1.806,1.26),paint,body,lods=(0,1,2))
        box('rear_ramp',(.05,2.50,1.24),(-L/2-.005,0,1.34),dark,body)
        box('a4_glacis' if 'm2_' in ident else 'a3_glacis',(1.10,2.65,.11),(2.57,0,1.56),paint,body,rot=(0,.49,0))
        for side in (-1,1):
            for j in range(7):
                for k in range(2):
                    box('armor_tile_'+str(side)+'_'+str(j)+'_'+str(k),(.73,.08,.28),(-2.60+j*.82,side*1.83,1.18+k*.33),paint,body,bevel=.008,lods=(0,1,2))
        for j in range(8):box('engine_deck_grille_'+str(j),(.055,.81,.025),(1.15+j*.10,-.71,2.00),dark,body,lods=(0,1))
        roof=2.06
    else:
        roof=1.94 if ident=='us_lav_lav_at' else 1.69
        profile=[(-L/2,.78),(L/2-.18,.65),(L/2,1.02),(L/2-1.10,roof),(-L/2+.08,roof)]
        prism(family+'_hull',profile,W-.48,mat=paint,parent=body,bevel=.035)
        r=.49
        for i,x in enumerate([2.19,1.02,-1.03,-2.19]):
            row=['F0','F1','R0','R1'][i]
            for side in (-1,1):
                wheel(row,x,side,r,W/2-.20,.38)
                box('wheel_brow_'+row+str(side),(1.15,.28,.095),(x,side*(W/2-.15),r*1.90),paint,body,bevel=.025)
        box('rear_ramp',(.06,W-.65,roof-.81),(-L/2,0,(roof+.81)/2),dark,body)
        for side in (-1,1):
            box('upper_side_'+str(side),(L-1.55,.12,.25),(-.44,side*(W/2-.19),roof-.17),paint,body)
            for j in range(4):box('stowage_'+str(side)+'_'+str(j),(.66,.16,.25),(-2.25+j*.92,side*(W/2-.08),roof-.17),dark,body,lods=(0,1,2))
    # Hatches/periscopes, stored equipment and rear lamps belong to each hull.
    for j,x in enumerate((.95,-1.48)):
        cyl('roof_hatch_'+str(j),.28,.07,(x,-.48,roof+.03),'Z',paint,body,seg=16)
    for side in (-1,1):
        box('rear_lamp_'+str(side),(.055,.16,.08),(-L/2-.012,side*(W*.32),1.12),steel,body,lods=(0,1,2))
    rigs={}; pivots={}
    for m in frame['mounts']:
        gun=m['role']=='gun'; yawname='turret' if gun else 'hmg'; pitchname='gun' if gun else 'hmg_gun'; muzzlename='muzzle' if gun else 'hmg_muzzle'
        pivot=Vector(m['pivot_m']); carrier=rigs.get(m['on'],body); parentpivot=pivots.get(m['on'],Vector((0,0,0)))
        yaw=empty(yawname,pivot-parentpivot,carrier); rigs[m['name']]=yaw;pivots[m['name']]=pivot
        muzzle=Vector(m['muzzle_m']); pitch=empty(pitchname,(0,0,muzzle.z),yaw)
        empty(muzzlename,(muzzle.x,muzzle.y,0),pitch)
        launcher=m['name']=='launcher'
        if launcher:
            box('tow_elevation_cradle',(.47,.50,.27),(0,0,-.16),paint,pitch,bevel=.025)
            for side in (-1,1):
                cyl('tow_tube_'+str(side),.165,1.50,(muzzle.x-.75,side*.20,0),'X',paint,pitch,seg=16)
                cyl('tow_opening_'+str(side),.127,.035,(muzzle.x-.012,side*.20,0),'X',dark,pitch,seg=16)
            if family in ('bradley','lav'):
                box('armored_tow_carrier',(1.54,.77,.10),(muzzle.x-.77,0,-.21),paint,pitch)
                box('armored_tow_hood',(1.54,.77,.075),(muzzle.x-.77,0,.25 if family=='lav' else .21),paint,pitch)
                for side in (-1,1):box('armored_tow_side_'+str(side),(1.54,.06,.43),(muzzle.x-.77,side*.37,0),paint,pitch)
            box('tow_optics',(.32,.22,.25),(.24,-.42,.06),glass,pitch)
            if family=='lav':
                base=roof-pivot.z;top=-.16
                cyl('tow_pedestal',.23,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
                box('tow_pedestal_support',(.42,.42,top-base),(-.07,0,(base+top)/2),paint,yaw,bevel=.02)
            else:cyl('tow_pedestal',.23,.34,(0,0,-.28),'Z',paint,yaw,seg=16)
        elif gun:
            brad=family=='bradley'
            width=1.48 if brad else 1.35
            prism('named_turret',[(-.85,0),(.79,0),(.87,.28),(.32,.72),(-.71,.70)],width,mat=paint,parent=yaw,bevel=.035)
            box('gun_mantlet',(.39,.46,.39),(.71,0,0),paint,pitch,bevel=.04)
            start=.76; length=muzzle.x-start
            cyl('autocannon_barrel',.058 if brad else .070,length,(start+length/2,0,0),'X',steel,pitch,seg=16)
            cyl('gun_sleeve',.098,.74,(1.17,0,0),'X',dark,pitch,seg=16)
            cyl('muzzle_brake',.087,.16,(muzzle.x-.08,0,0),'X',dark,pitch,seg=12)
            box('gunner_sight',(.31,.24,.21),(.23,-width*.35,.68),glass,yaw)
            if family=='lav':box('lav_commander_periscope',(.21,.19,.18),(-.38,-.36,.83),glass,yaw)
            cyl('commander_hatch',.27,.07,(-.27,-.36,.75),'Z',paint,yaw,seg=16)
            if brad:
                cyl('commander_viewer_base',.18,.20,(-.48,-.45,.88),'Z',paint,yaw,seg=16)
                box('commander_viewer',(.29,.29,.24),(-.48,-.45,.99),glass,yaw)
                if 'm3_' in ident:box('cfv_stowage_basket',(.45,1.28,.28),(-1.02,0,.44),dark,yaw)
        else:
            base=-.29;top=-.07
            cyl('remote_weapon_pedestal',.19,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            box('hmg_receiver',(.63,.18,.20),(.43,0,0),dark,pitch)
            cyl('hmg_barrel',.039,muzzle.x-.70,((muzzle.x+.70)/2,0,0),'X',steel,pitch,seg=12)
            box('ammo_box',(.35,.30,.30),(.25,.30,-.08),paint,pitch)
            box('remote_sight',(.27,.25,.22),(.33,-.24,.14),glass,pitch)
    # Track bottom is already within ground tolerance; preserve all frozen pivot heights.
    wreck=WRECK_ARG in args
    if wreck:burn()
    finish(ao_distance=.65,ao_rays=4)
    print('ROSTER_LIGHT_ARMOR',json.dumps({'family':family,'id':ident,'tris':triangles_by_tier()}))
    os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);(export_wreck if wreck else export)(out,texture_px=256)
    preview=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if preview and not wreck:
        bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=os.path.abspath(out))
        os.makedirs(preview,exist_ok=True)
        for o in bpy.data.objects:
            if o.type=='MESH':o.hide_render=('_LOD' in o.name and not o.name.endswith('_LOD0'))
        scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=960;scene.render.resolution_y=640;scene.render.resolution_percentage=100
        scene.world=bpy.data.worlds.new('review_world');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7
        bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.012))
        for loc,power in [((3,-6,12),1800),((-5,5,7),1100)]:
            bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.size=8;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
        bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.ortho_scale=L*1.35
        for label,loc in [('front',(L*1.5,-L*1.8,L*.85)),('side',(0,-L*2,L*.55)),('rear',(-L*1.6,L*1.7,L*.8))]:
            cam.location=loc;cam.rotation_euler=(Vector((0,0,H*.45))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=os.path.join(preview,label+'.png');bpy.ops.render.render(write_still=True)
