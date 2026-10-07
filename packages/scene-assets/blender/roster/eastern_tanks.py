"""Six-wheel Eastern tank lane; subject scripts own named silhouettes."""
import json, math, sys, hashlib
from pathlib import Path
import bpy, bmesh
from mathutils import Vector
sys.path.insert(0,str(Path(__file__).resolve().parent.parent))
from parts import *
from catalog_frames import family_variants
from armor import rig, preview, palette as western_palette
REPO=Path(__file__).resolve().parents[4]
REFERENCE_PHOTOS=[{'family': 't72', 'url': 'https://thumb.wikimedia.org/wikipedia/commons/thumb/5/56/Alabino05042017-40.jpg/960px-Alabino05042017-40.jpg', 'sha256': '8165f13648a6c2d3c000bc38b0ba3bbff97c65c91d3e65411a4870c1c1ac3547'}, {'family': 't80', 'url': 'https://thumb.wikimedia.org/wikipedia/commons/thumb/f/ff/T-80BVM.jpg/960px-T-80BVM.jpg', 'sha256': 'ba29c5dd0d9f53f747d4b253667a25c06d9eb1a3103966ffdbe8a278977df88f'}, {'family': 't90', 'url': 'https://thumb.wikimedia.org/wikipedia/commons/thumb/d/d9/T-90M.jpg/960px-T-90M.jpg', 'sha256': 'a6a58e43ed972944db823af7ea606e0ca7ba94fa326541019c75015e70fe2b5f'}]
REFERENCE_LIMITS={
    't72':'Base-family range; selected maximum hull/width gives conservative art envelope. B3 2016 equipment must match reference.',
    't80':'Family infobox mixes B/U values; chosen envelope is delegated approximation, not a BVM measurement.',
    't90':'Base family hull dimensions; T-90M bustle changes silhouette and cannot be generic rounded T-72 turret.',
}

def running_gear(parent, length, width, mats, turbine=False):
    radius=.325 if turbine else .36; center_z=.44; track_y=width/2-.38; track_width=.60
    a=length*.405; belt_r=.46; belt_z=.50; pitch=.17
    for side in (1,-1):
        sn='L' if side>0 else 'R'
        for k in range(6):
            x=-length*.32+k*(length*.64/5)
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


def palette():
    m=western_palette()
    m['armor']=textured('eastern_olive','olive_paint',colour=(.16,.205,.115),tint=.3,chip=.4,dirt=.5,rise=1)
    return m


def chassis(frame,hull,m,turbine=False,modern=False):
    L,W,H=frame['body_dimensions_m'];rear=-L/2;front=L/2;deck=1.21 if turbine else 1.23
    prism('upper_hull',[(rear+.02,.82),(front-.20,.80),(front,1.04),(front-1.25,deck),(rear+.05,deck)],W-.12,mat=m['armor'],parent=hull,bevel=.025)
    prism('lower_hull',[(rear+.25,.35),(front-.45,.35),(front-.10,.80),(rear,.91)],2.14,mat=m['dark'],parent=hull,bevel=.02)
    for side in (-1,1):
        for k in range(5):
            x=front-.53-k*.98
            box(f'side_era_{side}_{k}',(.94,.075,.46),(x,side*(W/2-.06),1.02),m['armor'],hull,bevel=.012)
            prism(f'lower_skirt_{side}_{k}',[(x-.45,.81),(x+.45,.81),(x+.45,.58),(x+.18,.48),(x-.45,.57)],.04,loc=(0,side*(W/2-.075),0),mat=m['rubber'],parent=hull,lods=(0,1,2))
            for j in range(2):box(f'skirt_fastener_{side}_{k}_{j}',(.035,.035,.05),(x-.35+j*.70,side*(W/2-.009),1.08),m['steel'],hull,lods=(0,))
        prism(f'front_fender_{side}',[(front-.63,1.11),(front-.04,.89),(front+.02,1.07),(front-.45,1.32)],.59,loc=(0,side*(W/2-.35),0),mat=m['armor'],parent=hull,bevel=.018)
        box(f'light_guard_{side}',(.16,.25,.20),(front-.30,side*1.09,1.22),m['armor'],hull,bevel=.02,lods=(0,1,2))
        cyl(f'headlamp_{side}',.075,.035,(front-.205,side*1.09,1.235),'X',m['lamp'],hull,seg=16,lods=(0,1))
    for row in range(2):
        for k in range(7):box(f'glacis_era_{row}_{k}',(.41,.40,.12),(front-.57-row*.46,-1.30+k*.43,1.065+row*.087),m['armor'],hull,rot=(0,-.19,0),bevel=.009,lods=(0,1,2))
    cyl('driver_hatch',.29,.045,(front-1.06,0,1.30),'Z',m['armor'],hull,seg=24,lods=(0,1,2))
    box('driver_periscope',(.14,.21,.045),(front-.80,0,1.265),m['glass'],hull,lods=(0,1))
    box('engine_recess',(1.55,2.26,.025),(rear+.98,0,deck+.017),m['black'],hull,lods=(0,1,2))
    for k in range(13):box(f'engine_louvre_{k}',(.05,2.20,.026),(rear+.27+k*.11,0,deck+.037),m['dark'],hull,lods=(0,1))
    if turbine:
        box('turbine_rear_exhaust',(.035,1.54,.29),(rear-.025,0,1.01),m['black'],hull,lods=(0,1,2))
        for k in range(6):box(f'turbine_exhaust_bar_{k}',(.025,1.48,.025),(rear-.047,0,.90+k*.044),m['dark'],hull,lods=(0,1))
    else:box('diesel_left_exhaust',(.57,.04,.18),(rear+.50,W/2-.15,1.09),m['black'],hull,lods=(0,1,2))


def round_turret(turret,m,rx=1.68,ry=1.40):
    rings=[(z,[(-.27+rx*s*math.cos(i*math.tau/20),ry*s*math.sin(i*math.tau/20)) for i in range(20)]) for z,s in [(.015,.83),(.12,1),(.37,.88),(.54,.61),(.60,.43)]]
    loft('cast_turret_dome',rings,mat=m['armor'],parent=turret,bevel=.015)


def era_cheeks(turret,m,relikt=False):
    for side in (-1,1):
        for k in range(5):
            theta=side*(.36+k*.27)
            box(f'{"relikt" if relikt else "kontakt5"}_cheek_{side}_{k}',(.66,.35,.26),(-.06+1.55*math.cos(theta),1.48*math.sin(theta),.36),m['armor'],turret,rot=(0,.29,theta),bevel=.012)
        for k in range(3):box(f'turret_side_era_{side}_{k}',(.39,.19,.36),(-.71-k*.35,side*1.31,.28),m['armor'],turret,bevel=.015,lods=(0,1,2))


def roof(turret,m,modern=False):
    for side in (-1,1):
        cyl(f'roof_hatch_{side}',.28,.055,(-.38,side*.57,.62),'Z',m['armor'],turret,seg=24,lods=(0,1,2))
        box(f'hatch_scope_{side}',(.16,.15,.07),(-.13,side*.65,.67),m['glass'],turret,lods=(0,1))
    box('sosna_sight',(.40,.30,.25),(.59,.65,.62),m['armor'],turret,bevel=.02,lods=(0,1,2))
    box('sosna_lens',(.025,.19,.15),(.805,.65,.63),m['glass'],turret,lods=(0,1))
    cyl('antenna_base',.045,.07,(-1.02,-.40,.62),'Z',m['dark'],turret,seg=10,lods=(0,1,2))
    cyl('antenna',.006,.17,(-1.02,-.40,.74),'Z',m['dark'],turret,seg=6,lods=(0,1))
    if modern:
        cyl('panorama_base',.15,.16,(-.53,.51,.70),'Z',m['armor'],turret,seg=20,lods=(0,1,2))
        box('panorama_head',(.29,.25,.16),(-.53,.51,.82),m['armor'],turret,bevel=.018,lods=(0,1,2))
        box('panorama_lens',(.022,.16,.085),(-.373,.51,.83),m['glass'],turret,lods=(0,1))
    for k in range(6):cyl(f'smoke_tube_{k}',.05,.26,(-.69+(k%3)*.17,.99,.48+(k//3)*.16),'X',m['dark'],turret,seg=12,rot=(0,-.35,.45),lods=(0,1,2))


def slats(hull,frame,m):
    L,W,H=frame['body_dimensions_m'];rear=-L/2
    for side in (-1,1):
        for k in range(9):box(f'rear_side_slat_{side}_{k}',(1.08,.025,.025),(rear+.53,side*(W/2-.025),.66+k*.065),m['steel'],hull,lods=(0,1))
        for k in range(4):box(f'rear_side_frame_{side}_{k}',(.024,.035,.64),(rear+.02+k*.34,side*(W/2-.025),.91),m['steel'],hull,lods=(0,1,2))
    for k in range(9):box(f'rear_slat_{k}',(.025,W-.18,.025),(rear-.045,0,.66+k*.065),m['steel'],hull,lods=(0,1))
    for side in (-1,1):box(f'rear_frame_{side}',(.035,.03,.64),(rear-.04,side*(W/2-.10),.91),m['steel'],hull,lods=(0,1,2))


def weapons(frame,gun,hmg,hmg_gun,trunnion,m,remote=False):
    reach=frame['mounts'][0]['muzzle_m'][0]-trunnion
    cyl('125mm_barrel',.077,reach-.11,((reach-.11)/2,0,0),'X',m['armor'],gun,seg=28)
    cyl('muzzle_end',.084,.12,(reach-.06,0,0),'X',m['steel'],gun,seg=24)
    cyl('muzzle_bore',.060,.012,(reach-.004,0,0),'X',m['black'],gun,seg=20)
    cyl('bore_evacuator',.128,.51,(reach*.44,0,0),'X',m['armor'],gun,seg=24)
    for i,x in enumerate((.26,reach*.44-.30,reach*.44+.30,reach-.22)):cyl(f'barrel_band_{i}',.091,.045,(x,0,0),'X',m['dark'],gun,seg=16,lods=(0,1,2))
    box('mantlet_cover',(.54,.63,.36),(-.06,0,0),m['canvas'],gun,bevel=.045)
    cyl('commander_gun_ring',.27,.075,(0,0,.04),'Z',m['armor'],hmg,seg=24)
    box('commander_gun_support',(.18,.13,.27),(0,0,.18),m['dark'],hmg)
    if remote:
        box('remote_weapon_pedestal',(.33,.35,.23),(0,0,.16),m['armor'],hmg,bevel=.025)
        box('remote_weapon_optic',(.18,.17,.18),(.17,-.19,.26),m['black'],hmg,lods=(0,1,2))
    else:box('commander_hatch_shield',(.13,.48,.40),(-.30,0,.20),m['armor'],hmg,bevel=.04,lods=(0,1,2))
    box('hmg_receiver',(.44,.12,.13),(.15,0,0),m['dark'],hmg_gun,bevel=.015)
    cyl('hmg_barrel',.025,1.01,(.925,0,0),'X',m['dark'],hmg_gun,seg=16)
    cyl('hmg_muzzle',.033,.06,(1.40,0,0),'X',m['steel'],hmg_gun,seg=12)
    box('hmg_ammo_box',(.24,.25,.22),(.13,.18,-.04),m['armor'],hmg_gun,lods=(0,1,2))


def export_family(family,build):
    args=script_args();review=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    rows=[]
    for v in family_variants(family):
        frame=v['frame'];reset();root=empty(v['id']);root['unit_id']=v['id'];hull=empty('hull',parent=root);m=palette()
        running_gear(hull,*frame['body_dimensions_m'][:2],m,family=='t80');turret,gun,hmg,hmg_gun,trunnion=rig(frame,root)
        build(v,frame,hull,turret,m);weapons(frame,gun,hmg,hmg_gun,trunnion,m,family=='t90')
        finish(ao_distance=1.0,ao_rays=8);counts=triangles_by_tier();assert all(counts[k]>counts[k+1]>0 for k in range(3))
        out=REPO/v['export'];export(str(out));rows.append({'id':v['id'],'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'triangles_by_tier':counts,'mounts':frame['mounts']})
        print('VARIANT',v['id'],counts,flush=True)
        if review:preview(out,Path(review)/v['id'],frame['body_dimensions_m'][0],frame['body_dimensions_m'][2])
    paths=[Path(__file__),Path(__file__).with_name(family+'.py'),Path(__file__).with_name('armor.py')]+[Path(__file__).parent.parent/n for n in ('parts.py','catalog_frames.py','common.py','textures.py')]
    data={'authoring':'Original procedural geometry; reference photos only, no third-party mesh/texture extraction','blender_version':bpy.app.version_string,'source_sha256':{str(p.relative_to(REPO)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},'frames':'fixtures/catalog.json','reference_photos':[r for r in REFERENCE_PHOTOS if r['family']==family],'reference_limits':REFERENCE_LIMITS[family],'variants':rows}
    (REPO/'assets/source/roster'/family/'source-receipt.json').write_text(json.dumps(data,indent=2)+'\n')
