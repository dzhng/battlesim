"""Worker-owned armor construction for Leopard 2 (the Abrams is rebuilt in abrams.py); scene-assets owns primitives,
materials, physical admission, LOD generation and export. Family outlines/gear live
in the two entrypoints, never inferred from an ID or a rescaled generic tank.
"""
import json, math, os, sys, hashlib
from pathlib import Path
import bpy, bmesh
from mathutils import Vector
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from parts import *
from catalog_frames import family_variants
from wreckage import WRECK_ARG, burn, export_wreck

REPO = Path(__file__).resolve().parents[4]
REFERENCE_PHOTOS = [{'url': 'https://thumb.wikimedia.org/wikipedia/commons/thumb/a/a7/Leopard_2_A7V_313_Bad_Frankenhausen_2024.JPG/960px-Leopard_2_A7V_313_Bad_Frankenhausen_2024.JPG', 'sha256': '0674f7147d27ecf1e16ab1a4f5c4e63da36726708b1f9388bb7df698a5959ea6'}, {'url': 'https://thumb.wikimedia.org/wikipedia/commons/thumb/8/87/German_Army_Leopard_2A6_tank_in_Oct._2012.jpg/960px-German_Army_Leopard_2A6_tank_in_Oct._2012.jpg', 'sha256': 'ca93b972d63f2d4b4707908c621e2156bfe0ab4b5cb5059aff925b93a50315e6'}, {'url': 'https://thumb.wikimedia.org/wikipedia/commons/thumb/6/6c/LEOPARD_2A9.jpg/960px-LEOPARD_2A9.jpg', 'sha256': '24a4b45810e13bbcf15a90b1085f5f914dbbf14c7f0d0170e1da76f1e68093c0', 'limitation': 'Leopard2 article labels this as2A8 demonstrator; file says2A9. Trophy layout evidence only, not verified production2A8.'}]


# Each family's paint scheme (`textures.SCHEMES`): its real nation's.
SCHEME = {'leopard': 'german_three_tone'}


def palette(scheme):
    return {
        'armor': paint(scheme, 'armor_paint', chip=.4, dirt=.55, rise=1.0),
        'dark': textured('running_gear', 'olive_paint', colour=(.046,.055,.035), dirt=.65, role='paint'),
        'rubber': tyre('track_rubber'),
        'steel': bare_steel('metal', chip=.3, dirt=.35),
        'track': track_steel('track_shoes'),
        'glass': glass('optics'),
        'black': flat_paint('recesses', (.016,.024,.018), rough=.8, grime=.2),
        'lamp': flat_paint('lamps', (.6,.55,.4), rough=.25, grime=.05),
        'canvas': textured('stowage_canvas', 'canvas', colour=(.13,.145,.075), dirt=.3, role='fabric'),
    }


def running_gear(parent, length, width, mats):
    radius=.35; center_z=.44; track_y=width/2-.38; track_width=.60
    a=length*.405; belt_r=.46; belt_z=.50; pitch=.17
    for side in (1,-1):
        sn='L' if side>0 else 'R'
        for k in range(7):
            x=-length*.32+k*(length*.64/6)
            wheel=empty(f'wheel_{sn}_{k+1}',(x,side*track_y,center_z),parent,props={'radius_m':radius})
            cyl(f'road_tyre_{sn}_{k}',radius,.28,(0,0,0),'Y',mats['rubber'],wheel,seg=28)
            cyl(f'road_disc_{sn}_{k}',radius*.82,.31,(0,0,0),'Y',mats['armor'],wheel,seg=24)
            cyl(f'road_hub_{sn}_{k}',radius*.26,.37,(0,0,0),'Y',mats['dark'],wheel,seg=16,lods=(0,1,2))
            for j in range(6):
                angle=j*math.tau/6
                cyl(f'wheel_bolt_{sn}_{k}_{j}',.022,.045,(radius*.47*math.cos(angle),side*.17,radius*.47*math.sin(angle)),'Y',mats['steel'],wheel,seg=6,lods=(0,))
            box(f'suspension_{sn}_{k}',(.45,.11,.10),(x+.19,side*(track_y-.22),center_z+.18),mats['dark'],parent,rot=(0,-.35,0),lods=(0,1))
        for label,x in [('sprocket',-a),('idler',a)]:
            wheel=empty(f'wheel_{sn}_{label}',(x,side*track_y,belt_z),parent,props={'radius_m':.37})
            cyl(f'{label}_{sn}',.37,.30,(0,0,0),'Y',mats['dark'],wheel,seg=24)
            cyl(f'{label}_hub_{sn}',.20,.35,(0,0,0),'Y',mats['armor'],wheel,seg=16,lods=(0,1,2))
            if label=='sprocket':
                for j in range(12):
                    angle=j*math.tau/12
                    box(f'sprocket_tooth_{sn}_{j}',(.09,.32,.06),(.385*math.sin(angle),0,.385*math.cos(angle)),mats['steel'],wheel,rot=(0,angle,0),lods=(0,1))
        for k in range(3):
            x=-a*.65+k*a*.65
            wheel=empty(f'wheel_{sn}_return_{k}',(x,side*track_y,.83),parent,props={'radius_m':.10})
            cyl(f'return_roller_{sn}_{k}',.10,.20,(0,0,0),'Y',mats['dark'],wheel,seg=16,lods=(0,1))
        loop_length=4*a+math.tau*belt_r
        track=empty('track_'+sn,parent=parent,props={'track_length_m':loop_length,'link_pitch_m':pitch})
        def belt(bm,lod):
            # Rounded end runs plus both straight runs, with real annular thickness.
            steps=(24,14,8,5)[lod];outer=[];inner=[];arc=[]
            for cx,start in [(a,-math.pi/2),(-a,math.pi/2)]:
                for j in range(steps+1):
                    theta=start+j*math.pi/steps
                    outer.append((cx+(belt_r+.04)*math.cos(theta),belt_z+(belt_r+.04)*math.sin(theta)))
                    inner.append((cx+(belt_r-.04)*math.cos(theta),belt_z+(belt_r-.04)*math.sin(theta)))
            rings=[]
            for profile in (outer,inner):
                rings.append([[bm.verts.new((x,side*track_y+y,z)) for x,z in profile] for y in (-track_width/2,track_width/2)])
            count=len(outer);uv=bm.loops.layers.uv.new('UVMap');s=0
            for j in range(count):
                nxt=(j+1)%count
                step=math.hypot(outer[nxt][0]-outer[j][0],outer[nxt][1]-outer[j][1])
                for ring,reverse in [(rings[0],False),(rings[1],True)]:
                    verts=[ring[0][j],ring[0][nxt],ring[1][nxt],ring[1][j]]
                    face=bm.faces.new(verts[::-1] if reverse else verts)
                    for loop in face.loops:
                        idx=verts.index(loop.vert);loop[uv].uv=((s+(step if idx in (1,2) else 0))/pitch,0 if idx<2 else 1)
                for edge in (0,1):
                    verts=[rings[0][edge][j],rings[1][edge][j],rings[1][edge][nxt],rings[0][edge][nxt]]
                    face=bm.faces.new(verts)
                    for loop in face.loops:
                        idx=verts.index(loop.vert)
                        loop[uv].uv=((s+(step if idx in (2,3) else 0))/pitch,0 if idx in (0,3) else .08/pitch)
                s+=step
            bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
        mesh_part('track_'+sn+'_band',belt,mats['track'],track)
        for j in range(int(2*a/.18)):
            x=-a+(j+.5)*2*a/int(2*a/.18)
            box(f'track_pad_{sn}_{j}',(.12,track_width*.78,.032),(x,side*track_y,.024),mats['rubber'],track,lods=(0,1))


def rig(frame,root):
    cannon=frame['mounts'][0];hp=frame['mounts'][1]
    pivot=Vector(cannon['pivot_m']);offset=Vector(cannon['muzzle_m'])
    turret=empty('turret',tuple(pivot),root)
    trunnion=1.60 if frame['body_dimensions_m'][2]<2.6 else 1.85
    gun=empty('gun',(trunnion,0,offset.z),turret)
    empty('muzzle',(offset.x-trunnion,0,0),gun)
    hmg=empty('hmg',tuple(Vector(hp['pivot_m'])-pivot),turret)
    hmg_gun=empty('hmg_gun',(0,0,hp['muzzle_m'][2]),hmg)
    empty('hmg_muzzle',(hp['muzzle_m'][0],0,0),hmg_gun)
    return turret,gun,hmg,hmg_gun,trunnion


def guns(frame,gun,hmg,hmg_gun,trunnion,mats):
    reach=frame['mounts'][0]['muzzle_m'][0]-trunnion
    # The long L55, fixed by the adopted muzzle.
    cyl('main_barrel',.086,reach-.12,((reach-.12)/2,0,0),'X',mats['armor'],gun,seg=28)
    cyl('muzzle_end',.092,.15,(reach-.075,0,0),'X',mats['steel'],gun,seg=24)
    cyl('muzzle_bore',.064,.012,(reach-.005,0,0),'X',mats['black'],gun,seg=20)
    evac=reach*.42
    cyl('bore_evacuator',.145,.65,(evac,0,0),'X',mats['armor'],gun,seg=28)
    for i,x in enumerate((.25,evac-.36,evac+.36,reach-.25)):
        cyl(f'thermal_sleeve_band_{i}',.102,.055,(x,0,0),'X',mats['dark'],gun,seg=20,lods=(0,1,2))
    box('mantlet',(.55,.70,.48),(.03,0,-.01),mats['armor'],gun,bevel=.045)
    cyl('roof_gun_ring',.27,.075,(0,0,.04),'Z',mats['armor'],hmg,seg=24)
    box('roof_gun_support',(.15,.14,.27),(0,0,.17),mats['dark'],hmg)
    box('roof_receiver',(.49,.13,.14),(.15,0,0),mats['dark'],hmg_gun,bevel=.018)
    cyl('roof_barrel',.025,1.01,(.925,0,0),'X',mats['dark'],hmg_gun,seg=16)
    cyl('roof_muzzle',.036,.06,(1.40,0,0),'X',mats['steel'],hmg_gun,seg=12)
    box('roof_ammo_box',(.24,.26,.26),(.12,.21,-.08),mats['armor'],hmg_gun,lods=(0,1,2))


def trophy(turret,z,width,mats):
    # Four flat-panel sensor faces on two side hardware stations, plus paired launchers.
    for side,sn in [(1,'L'),(-1,'R')]:
        radar=empty('trophy_radar_'+sn,(-.75,side*(width/2-.17),z),turret)
        launcher=empty('trophy_launcher_'+sn,(-1.30,side*(width/2-.17),z+.34),turret)
        box('trophy_radar_housing_'+sn,(.82,.16,.43),(0,0,0),mats['armor'],radar,bevel=.045)
        box('trophy_radar_panel_'+sn,(.65,.021,.31),(0,side*.092,0),mats['dark'],radar,bevel=.02)
        box('trophy_radar_forward_'+sn,(.025,.17,.31),(.42,0,0),mats['black'],radar,lods=(0,1,2))
        box('trophy_pedestal_'+sn,(.20,.19,.29),(0,0,-.14),mats['steel'],launcher)
        box('trophy_countermeasure_'+sn,(.38,.23,.22),(0,0,.06),mats['armor'],launcher,rot=(0,-.2,side*.18),bevel=.02)
        box('trophy_face_'+sn,(.25,.025,.14),(0,side*.13,.06),mats['dark'],launcher,lods=(0,1,2))


def hatches(turret,z,mats):
    for side in (-1,1):
        cyl(f'roof_hatch_{side}',.30,.055,(-.58,side*.68,z),'Z',mats['armor'],turret,seg=28,lods=(0,1,2))
        for k in range(4):
            box(f'hatch_periscope_{side}_{k}',(.15,.065,.07),(-.48+k*.12,side*.77,z+.055),mats['glass'],turret,lods=(0,1))
    for side in (-1,1):
        cyl(f'antenna_base_{side}',.07,.13,(-1.48,side*.90,z+.07),'Z',mats['dark'],turret,seg=12,lods=(0,1,2))
        cyl(f'antenna_{side}',.008,.20,(-1.48,side*.90,z+.20),'Z',mats['dark'],turret,seg=6,lods=(0,1))


def preview(path,outdir,length,height):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(path))
    for o in bpy.data.objects:
        if o.type=='MESH':o.hide_render='_LOD' in o.name and not o.name.endswith('_LOD0')
    scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE'
    scene.render.resolution_x=1100;scene.render.resolution_y=730;scene.render.resolution_percentage=100
    scene.world=bpy.data.worlds.new('review');scene.world.use_nodes=True
    scene.world.node_tree.nodes['Background'].inputs['Color'].default_value=(.27,.29,.27,1)
    scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7
    bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.015))
    ground=bpy.data.materials.new('review_ground');ground.diffuse_color=(.19,.21,.18,1);bpy.context.object.data.materials.append(ground)
    for position,power,size in [((5,-7,12),2600,8),((-7,4,8),1600,7)]:
        bpy.ops.object.light_add(type='AREA',location=position);o=bpy.context.object;o.data.energy=power;o.data.size=size;o.rotation_euler=(Vector((0,0,1.4))-o.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add();camera=bpy.context.object;scene.camera=camera;camera.data.type='ORTHO';camera.data.ortho_scale=length*1.6
    outdir.mkdir(parents=True,exist_ok=True)
    for label,position in [('front',(length*1.5,-length*1.8,length*.90)),('side',(0,-length*2,length*.30)),('rear',(-length*1.6,length*1.7,length*.8))]:
        camera.location=position;camera.rotation_euler=(Vector((1,0,height*.44))-camera.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=str(outdir/(label+'.png'));bpy.ops.render.render(write_still=True)


def export_family(family,build):
    args=script_args();selected=next((a.split('=',1)[1] for a in args if a.startswith('--variant=')),None)
    review=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    receipt=[];wreck=WRECK_ARG in args
    for variant in family_variants(family):
        if selected and variant['id']!=selected:continue
        reset();frame=variant['frame'];root=empty(variant['id']);root['unit_id']=variant['id']
        mats=palette(SCHEME[family]);hull=empty('hull',parent=root)
        running_gear(hull,*frame['body_dimensions_m'][:2],mats)
        turret,gun,hmg,hmg_gun,trunnion=rig(frame,root)
        build(variant,frame,hull,turret,mats)
        guns(frame,gun,hmg,hmg_gun,trunnion,mats)
        if wreck:burn()
        finish(ao_distance=1.0,ao_rays=8)
        counts=triangles_by_tier();assert all(counts[k]>counts[k+1]>0 for k in range(3)),counts
        out=REPO/variant['export']
        if wreck:
            print('VARIANT',variant['id'],counts,export_wreck(str(out)),flush=True);continue
        export(str(out))
        receipt.append({'id':variant['id'],'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'triangles_by_tier':counts,'mounts':frame['mounts'],'trophy_nodes':['trophy_radar_L','trophy_radar_R','trophy_launcher_L','trophy_launcher_R'] if variant['variant']=='2A8' else []})
        print('VARIANT',variant['id'],counts,flush=True)
        if review:preview(out,Path(review)/variant['id'],frame['body_dimensions_m'][0],frame['body_dimensions_m'][2])
    if wreck:return
    receipt_path=REPO/'assets/source/roster'/family/'source-receipt.json'
    references=REFERENCE_PHOTOS
    source_paths=[Path(__file__),Path(__file__).with_name(family+'.py'),Path(__file__).parent.parent/'parts.py',Path(__file__).parent.parent/'catalog_frames.py',Path(__file__).parent.parent/'common.py',Path(__file__).parent.parent/'textures.py']
    sources={str(p.relative_to(REPO)):hashlib.sha256(p.read_bytes()).hexdigest() for p in source_paths}
    receipt_path.write_text(json.dumps({'blender_version':bpy.app.version_string,'source_sha256':sources,'references':references,'reference_limits':['A8 hardware based on demonstrator photo with conflicting filename; production layout not verified'],'authoring':'Original procedural geometry; photos used as visual reference only, no source mesh/texture extraction','frames':'fixtures/catalog.json','variants':receipt},indent=2)+'\n')
