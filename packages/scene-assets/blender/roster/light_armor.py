"""Named US light armor: shared running gear, explicit family hulls and mount hardware."""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *

ROOT=os.path.abspath(os.path.join(os.path.dirname(__file__),'../../../..'))

def build(family):
    args=script_args(); out=next(a for a in args if not a.startswith('--'))
    ident=os.path.basename(out).removesuffix('.glb')
    manifest=json.load(open(os.path.join(ROOT,'specs/unit-roster/manifests',family+'.json')))
    variant=next(v for v in manifest['variants'] if v['id']==ident)
    frame=variant['physical_authoring']; L,W,H=frame['body_dimensions_m']
    reset()
    paint=textured('armor_olive','olive_paint',tint=1,chip=.25,dirt=.45,rise=1.1)
    dark=textured('dark_steel','olive_paint',colour=(.045,.055,.035),chip=.2,dirt=.5)
    rubber=textured('rubber','rubber',chip=0,dirt=.3)
    steel=textured('gun_steel','bare_steel',chip=.15,dirt=.2)
    glass=flat_paint('optics',(.018,.06,.07),rough=.15,grime=.1)
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
            mesh_part('track_band_'+str(side),belt,dark,track)
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
    elif family in ('stryker','lav'):
        islav=family=='lav'; roof=(1.94 if ident=='us_lav_lav_at' else 1.69) if islav else (1.69 if 'm1296' in ident else 2.545)
        profile=[(-L/2,.78),(L/2-.18,.65),(L/2,1.02),(L/2-1.10,roof),(-L/2+.08,roof)]
        prism(family+'_hull',profile,W-.48,mat=paint,parent=body,bevel=.035)
        r=.49 if islav else .56
        for i,x in enumerate(([2.19,1.02,-1.03,-2.19] if islav else [2.35,1.16,-1.15,-2.33])):
            row=['F0','F1','R0','R1'][i]
            for side in (-1,1):
                wheel(row,x,side,r,W/2-.20,.38)
                box('wheel_brow_'+row+str(side),(1.15,.28,.095),(x,side*(W/2-.15),r*1.90),paint,body,bevel=.025)
        box('rear_ramp',(.06,W-.65,roof-.81),(-L/2,0,(roof+.81)/2),dark,body)
        for side in (-1,1):
            box('upper_side_'+str(side),(L-1.55,.12,.25),(-.44,side*(W/2-.19),roof-.17),paint,body)
            for j in range(4):box('stowage_'+str(side)+'_'+str(j),(.66,.16,.25),(-2.25+j*.92,side*(W/2-.08),roof-.17),dark,body,lods=(0,1,2))
        if not islav:box('driver_periscope',(.40,.36,.13),(1.47,.57,roof+.03),glass,body)
    elif family=='humvee':
        # HMMWV silhouette: short hood, upright split windscreen, exposed
        # wheel arches and a compact rear bed under the ring mount.
        roof=2.18
        for i,x in enumerate((1.65,-1.55)):
            for side in (-1,1):
                wheel('F' if i==0 else 'R',x,side,.54,1.04,.40)
                box('humvee_fender_'+str(i)+str(side),(1.42,.38,.15),(x,side*1.01,1.13),paint,body,bevel=.05)
        prism('humvee_cab',[(-1.42,.91),(1.18,.91),(1.08,1.66),(.70,roof),(-1.25,roof)],2.04,mat=paint,parent=body,bevel=.045)
        prism('humvee_hood',[(1.00,1.00),(2.72,1.00),(2.62,1.38),(1.18,1.52)],1.90,mat=paint,parent=body,bevel=.045)
        box('humvee_rear_bed',(1.65,2.02,.42),(-1.98,0,1.18),paint,body,bevel=.035)
        box('humvee_bumper',(.18,2.30,.20),(2.75,0,.92),dark,body)
        box('humvee_grille',(.035,1.06,.28),(2.82,0,1.29),dark,body)
        for j in range(7): box('humvee_grille_bar_'+str(j),(.04,.035,.24),(2.84,-.43+j*.14,1.29),paint,body,lods=(0,1))
        for side in (-1,1):
            box('humvee_windscreen_'+str(side),(.035,.72,.48),(.88,side*.42,1.86),glass,body,rot=(0,-.57,0))
            box('humvee_mirror_'+str(side),(.10,.16,.20),(.73,side*1.20,1.98),dark,body,lods=(0,1,2))
            for j,x in enumerate((.25,-.72)):
                box('humvee_door_'+str(side)+str(j),(.85,.032,.91),(x,side*1.03,1.48),paint,body,bevel=.02)
                box('humvee_window_'+str(side)+str(j),(.68,.034,.39),(x,side*1.05,1.99),glass,body,bevel=.02)
            box('humvee_bed_rail_'+str(side),(1.58,.08,.38),(-1.96,side*.98,1.56),paint,body)
    else:
        roof=2.535
        for i,x in enumerate((2.13,-1.86)):
            for side in (-1,1):
                wheel('F' if i==0 else 'R',x,side,.60,1.03,.42)
                box('fender_'+str(i)+str(side),(1.46,.46,.16),(x,side*.98,1.22),paint,body,bevel=.06)
        prism('jltv_armored_cab',[(-1.16,.94),(1.35,.94),(1.24,1.90),(.70,roof),(-1.08,roof)],2.03,mat=paint,parent=body,bevel=.055)
        prism('jltv_short_hood',[(1.10,1.03),(3.01,1.03),(2.91,1.47),(1.25,1.66)],1.85,mat=paint,parent=body,bevel=.055)
        box('jltv_rear_bed',(1.80,2.02,.44),(-2.16,0,1.22),paint,body,bevel=.04)
        box('front_bumper',(.18,2.35,.22),(3.02,0,.94),dark,body)
        box('front_grille',(.03,1.18,.29),(3.08,0,1.36),dark,body)
        for j in range(7):box('jltv_grille_bar_'+str(j),(.035,.035,.27),(3.101,-.48+j*.16,1.36),paint,body,lods=(0,1))
        for side in (-1,1):
            cyl('jltv_headlamp_'+str(side),.12,.055,(3.035,side*.77,1.39),'X',glass,body,seg=16)
            box('jltv_step_'+str(side),(1.85,.22,.10),(-.12,side*1.08,.88),dark,body)
            box('jltv_mirror_'+str(side),(.10,.17,.24),(.82,side*1.21,2.16),dark,body,lods=(0,1,2))
        for side in (-1,1):
            box('windscreen_'+str(side),(.035,.75,.54),(1.01,side*.43,2.14),glass,body,rot=(0,-.65,0))
            for j,x in enumerate((.33,-.67)):
                box('door_'+str(side)+str(j),(.89,.032,1.02),(x,side*1.033,1.525),paint,body,bevel=.025)
                box('window_'+str(side)+str(j),(.74,.034,.47),(x,side*1.055,2.23),glass,body,bevel=.025)
            box('bed_rail_'+str(side),(1.73,.085,.43),(-2.14,side*.97,1.62),paint,body)

    # Hatches/periscopes, stored equipment and rear lamps belong to each hull.
    for j,x in enumerate((.20,-.70) if family=='jltv' else (.95,-1.48)):
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
            if family in ('bradley','lav','stryker'):
                box('armored_tow_carrier',(1.54,.77,.10),(muzzle.x-.77,0,-.21),paint,pitch)
                box('armored_tow_hood',(1.54,.77,.075),(muzzle.x-.77,0,.25 if family=='lav' else .21),paint,pitch)
                for side in (-1,1):box('armored_tow_side_'+str(side),(1.54,.06,.43),(muzzle.x-.77,side*.37,0),paint,pitch)
            box('tow_optics',(.32,.22,.25),(.24,-.42,.06),glass,pitch)
            if family=='lav':
                base=roof-pivot.z;top=-.16
                cyl('tow_pedestal',.23,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
                box('tow_pedestal_support',(.42,.42,top-base),(-.07,0,(base+top)/2),paint,yaw,bevel=.02)
            elif family=='stryker':
                base=roof-pivot.z;top=-.16
                cyl('tow_pedestal',.23,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            else:cyl('tow_pedestal',.23,.34,(0,0,-.28),'Z',paint,yaw,seg=16)
        elif gun:
            brad=family=='bradley'; drag='m1296' in ident
            width=1.48 if brad else (1.66 if drag else 1.35)
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
            if drag:box('dragoon_sensor_head',(.33,.32,.28),(-.32,-.49,.83),glass,yaw)
        else:
            base=roof-pivot.z if family in ('stryker','jltv') else -.29;top=.22 if family in ('stryker','jltv') else -.07
            cyl('remote_weapon_pedestal',.19,top-base,(0,0,(base+top)/2),'Z',paint,yaw,seg=16)
            box('hmg_receiver',(.63,.18,.20),(.43,0,0),dark,pitch)
            cyl('hmg_barrel',.039,muzzle.x-.70,((muzzle.x+.70)/2,0,0),'X',steel,pitch,seg=12)
            box('ammo_box',(.35,.30,.30),(.25,.30,-.08),paint,pitch)
            box('remote_sight',(.27,.25,.22),(.33,-.24,.14),glass,pitch)
            if family=='jltv':
                for side in (-1,1):box('gunner_shield_'+str(side),(.90,.08,.40),(.15,side*.51,.10),paint,yaw,bevel=.025)
            if 'm1127' in ident:
                cyl('recon_sensor_pedestal',.10,.43,(-.54,.49,.12),'Z',paint,yaw,seg=12)
                box('recon_sensor_head',(.31,.36,.23),(-.54,.49,.42),glass,yaw)
    # Track bottom is already within ground tolerance; preserve all frozen pivot heights.
    finish(ao_distance=.65,ao_rays=4)
    print('ROSTER_LIGHT_ARMOR',json.dumps({'family':family,'id':ident,'tris':triangles_by_tier()}))
    os.makedirs(os.path.dirname(os.path.abspath(out)),exist_ok=True);export(out,texture_px=256)
    preview=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if preview:
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
