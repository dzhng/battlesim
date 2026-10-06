"""Named roster equipment on the unchanged Quaternius infantry rig.

Usage: infantry_equipment.py <unit-id> [a|b|c] [active|carried] [--preview=<dir>]
Add --preview-only to inspect the existing source without rebuilding it.
Writes that manifest unit's exclusive directory. Rifle-family grip, support and
sight anchors stay identical to the shared clips. Lengths are world metres;
geometry compensates for the shared hold and soldier scales, never the rig.
Tripod ATGMs use a root-weighted kit and the coordinator's frozen active pose.
Research-gated top-attack teams deliberately cannot dispatch.
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import bpy
import bmesh
from mathutils import Matrix, Vector
from common import REPO, box, cyl, empty, export_glb, script_args, texture_uvs, obj_from_bm
from infantry_kit import Kit, look_of, bone_of
from infantry_rig import Rig, SCALE
from mesh_lods import canonical, make_tiers, triangle_count
import weapons

MANIFEST = json.load(open(os.path.join(REPO, 'specs/unit-roster/manifests/infantry.json')))


def rifle_equipment(mats, model):
    """Distinct receiver/stock/optic layouts, with shared wrist anchors."""
    if model == 'carbine':
        return weapons.rifle(mats)
    root = empty(model, (0, 0, 0))
    black = mats['gun_black']
    finish = mats['gun_fde'] if model == 'm110' else black
    # Named references are complete unsuppressed weapon lengths. Fixed rear
    # anchor preserves shoulder fit; the barrel consumes the remaining length.
    length = MANIFEST['equipment_authoring'][model]['length_m']
    rear = .40
    front = rear - length / (SCALE * weapons.RIFLE_SCALE)
    bore = .045

    def b(name, size, loc, mat=black, rot=(0, 0, 0)):
        return box(model+'_'+name, size, loc, mat, root, rot=rot, bevel_=.002)

    def c(name, radius, depth, loc, mat=black):
        return cyl(model+'_'+name, radius, depth, loc, 'Y', mat, root, seg=12)

    b('pistol_grip', (.030, .038, .10), (0, .06, -.05), rot=(math.radians(22), 0, 0))
    b('trigger_guard', (.035, .06, .013), (0, .017, -.044))
    # The optic's eyepiece and the left wrist remain at the clip anchors.
    c('scope', .022 if model in ('m110', 'svd') else .029, .29, (0, -.075, .11))
    c('objective', .031 if model in ('m110', 'svd') else .038, .065, (0, -.23, .11))
    b('optic_mount', (.022, .12, .028), (0, -.05, .080))
    cyl(model+'_scope_turret', .015, .047, (0, -.08, .143), 'Z', black, root, seg=10)
    if model == 'm110':
        b('upper_receiver', (.039, .27, .06), (0, -.06, .043), finish)
        b('lower_receiver', (.036, .18, .05), (0, .0, -.005), finish)
        b('straight_magazine', (.032, .085, .12), (0, -.08, -.09))
        b('fixed_stock', (.045, .23, .09), (0, .285, .031), finish)
        c('buffer', .018, .12, (0, .16, .045), finish)
        b('handguard', (.053, .34, .056), (0, -.335, .045), finish)
        for side in (-1, 1):
            for j in range(5):
                b('vent_%s_%s'%(side,j), (.002, .033, .015), (side*.027, -.20-j*.058, .045))
        barrel_start = -.505
        brake = .045
    elif model == 'svd':
        b('slim_receiver', (.037, .26, .055), (0, -.055, .034))
        b('wood_handguard', (.047, .35, .052), (0, -.34, .033), mats['gun_fde'])
        # Open skeletal thumbhole stock, not a solid AR butt.
        b('stock_top', (.036, .26, .028), (0, .265, .042), mats['gun_fde'])
        b('stock_bottom', (.029, .24, .021), (0, .245, -.058), mats['gun_fde'], rot=(math.radians(-12), 0, 0))
        b('buttplate', (.040, .027, .135), (0, rear-.013, -.01))
        b('cheek_rest', (.043, .13, .030), (0, .29, .071), mats['gun_fde'])
        b('side_scope_bracket', (.022, .12, .070), (.033, -.025, .065))
        bm=bmesh.new()
        edge=[(-.108+.035*(j/6)**2,-.039-.145*j/6) for j in range(7)]
        profile=edge+[(y+.069,z) for y,z in reversed(edge)]
        rings=[[bm.verts.new((x,y,z)) for y,z in profile] for x in (-.016,.016)]
        bm.faces.new(rings[0][::-1]);bm.faces.new(rings[1])
        for j in range(len(profile)):
            k=(j+1)%len(profile);bm.faces.new((rings[0][j],rings[0][k],rings[1][k],rings[1][j]))
        bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
        magazine=obj_from_bm('svd_curved_magazine',bm,[black],root)
        for side in (-1,1):
            for j in range(4):b('handguard_vent_%s_%s'%(side,j),(.002,.035,.015),(side*.024,-.22-j*.074,.034))
        barrel_start = -.515
        brake = .085
        b('front_sight', (.021, .023, .068), (0, front+.13, .073))
    elif model == 'm107':
        b('large_rectangular_receiver', (.079, .61, .098), (0, -.045, .033))
        b('stock', (.078, .17, .105), (0, .30, .022))
        b('cheek_pad', (.064, .21, .025), (0, .235, .092))
        b('large_magazine', (.055, .105, .15), (0, -.20, -.080))
        b('carry_handle', (.016, .13, .024), (.063, -.17, .105))
        b('ventilated_shroud_top',(.087,.31,.013),(0,-.505,bore+.04))
        b('ventilated_shroud_bottom',(.087,.31,.013),(0,-.505,bore-.04))
        for side in (-1,1):
            for j in range(7):
                b('shroud_web_%s_%s'%(side,j),(.009,.015,.07),(side*.039,-.36-j*.049,bore))
        barrel_start = -.66
        brake = .15
        b('multiport_brake', (.099, brake, .062), (0, front+brake/2, bore))
        for side in (-1,1):
            for j in range(3):
                b('brake_port_%s_%s'%(side,j), (.002, .025, .031), (side*.050, front+.029+j*.046, bore))
    else:
        b('bullpup_receiver', (.066, .55, .079), (0, .10, .036))
        b('rear_magazine', (.046, .10, .145), (0, .26, -.078))
        b('rear_buttplate', (.080, .027, .145), (0, rear-.013, .01))
        b('cheek_rest', (.063, .18, .022), (0, .27, .09), mats['gun_fde'])
        b('forward_handguard', (.048, .24, .034), (0, -.29, .015))
        barrel_start = -.18
        brake = .18
        b('large_muzzle_brake', (.096, brake, .080), (0, front+brake/2, bore))
        for side in (-1,1):
            for j in range(2):
                b('brake_port_%s_%s'%(side,j), (.002, .053, .042), (side*.049, front+.049+j*.082, bore))
    barrel_end = front+brake
    c('barrel', .011 if model in ('m110','svd') else .018, barrel_start-barrel_end, (0, (barrel_start+barrel_end)/2, bore))
    if model in ('m110', 'svd'):
        c('flash_hider', .017, brake, (0, front+brake/2, bore))
    if model != 'svd':
        # Folded legs are carried, not an invented deployed bipod animation.
        for side in (-1,1):
            b('folded_bipod_'+str(side), (.014, .25, .020), (side*.035, -.43, -.003))
        c('bipod_hinge', .021, .025, (0, -.33, .015))
    empty('muzzle', (0, front, bore), root, .02)
    return root


def rpg_equipment(mats, model, socket=True):
    root = empty(model, (0, 0, 0))
    black, tube = mats['gun_black'], mats['launcher']
    frame = MANIFEST['equipment_authoring'][model]
    length = frame['length_m' if socket else 'packed_length_m'] / SCALE
    rear = .42 if model == 'rpg7' else .69
    front = rear-length
    cyl(model+'_tube', .025 if model=='rpg7' else .052, length, (0,(front+rear)/2,.10), 'Y', tube, root, seg=20)
    for y in (rear-.08, front+.10):
        cyl(model+'_band_'+str(y), .033 if model=='rpg7' else .061, .045, (0,y,.10), 'Y', black, root, seg=16)
    box(model+'_trigger_grip', (.032,.04,.15), (0,.06,.025), black, root, bevel_=.005)
    # The shared exported launcher clips do not fully reach their ideal left
    # IK target. Fit the support surface to their actual fingers, rather than
    # placing a detached grip at the unachieved target or editing the clips.
    if model=='rpg7':
        box('rpg7_support_grip',(.035,.045,.105),(.075,-.07,-.015),black,root,bevel_=.005)
        box('rpg7_support_bracket',(.105,.065,.07),(.04,-.07,.07),black,root,bevel_=.005)
    if model=='rpg29':
        box('rpg29_support_cradle',(.15,.14,.08),(.055,-.075,.025),black,root,bevel_=.005)
    # Shared launcher hold's actual left-side sight position.
    box(model+'_optic_bracket', (.11,.08,.036), (.07,.12,.11), black, root)
    cyl(model+'_optic', .027,.16,(.14,.13,.13),'Y',black,root,seg=16)
    if model=='rpg7':
        cyl('rpg7_wood_heat_shield', .036,.37,(0,.13,.10),'Y',mats['gun_fde'],root,seg=16)
        bpy.ops.mesh.primitive_cone_add(vertices=20,radius1=.027,radius2=.065,depth=.16,location=(0,rear-.04,.10),rotation=(-math.pi/2,0,0))
        flare=bpy.context.object;flare.name='rpg7_rear_flare';flare.data.materials.append(black);flare.parent=root
        # The grenade is loaded only in the active kit.
        if socket:
            cyl('pg7_motor',.021,.17,(0,front-.075,.10),'Y',black,root,seg=16)
            cyl('pg7_warhead',.055,.16,(0,front-.22,.10),'Y',tube,root,seg=20)
            bpy.ops.mesh.primitive_cone_add(vertices=20,radius1=.055,radius2=0,depth=.18,location=(0,front-.39,.10),rotation=(math.pi/2,0,0))
            cone=bpy.context.object;cone.name='pg7_pointed_nose';cone.data.materials.append(tube);cone.parent=root
        muzzle=front-.48
    else:
        cyl('rpg29_rear_venturi',.064,.14,(0,rear-.015,.10),'Y',black,root,seg=20)
        for side in (-1,1):
            box('rpg29_folded_bipod_'+str(side),(.018,.29,.021),(side*.065,front+.34,.043),black,root)
        if not socket:
            cyl('rpg29_detached_front_half',.052,.94/SCALE,(.16,(front+rear)/2,.10),'Y',tube,root,seg=20)
            for y in (front+.04,rear-.04):
                cyl('rpg29_packed_retention_'+str(y),.059,.04,(.16,y,.10),'Y',black,root,seg=16)
        muzzle=front
    if socket:
        empty('muzzle',(0,muzzle,.10),root,.02)
    return root


def tripod_equipment(kit, model, carried=False):
    """Actual tube, optics and tripod on root; packed tube/legs ride the back.

    Coordinates here are world metres in the source's -Y-forward frame, chosen
    against the reimported shared kneel_fire phase0. Root weighting keeps the
    three feet grounded when the operator's pelvis/arms move into that pose.
    """
    frame=MANIFEST['equipment_authoring'][model]
    root=empty(model+'_packed' if carried else model+'_ground_launcher',(0,0,0))
    dark,tube=kit.m['gun_black'],kit.m['launcher']
    length=frame['tube_length_m'];bore=frame['bore_m']

    def b(name,size,loc,mat=tube):
        return box(model+'_'+name,tuple(v/SCALE for v in size),Vector(loc)/SCALE,mat,root,bevel_=.004/SCALE)

    def c(name,radius,depth,loc,axis='Y',mat=tube):
        return cyl(model+'_'+name,radius/SCALE,depth/SCALE,Vector(loc)/SCALE,axis,mat,root,seg=16,bevel_=.002/SCALE)

    def beam(name,a,z,radius=.025,mat=tube):
        a,z=Vector(a),Vector(z);reach=z-a
        return cyl(model+'_'+name,radius/SCALE,reach.length/SCALE,(a+z)/(2*SCALE),'Z',mat,root,seg=10,rot=reach.to_track_quat('Z','Y').to_euler())

    if carried:
        c('packed_canister',.085 if model=='tow' else .078,length,(0,0,.10))
        for side in (-1,1):c('packed_tube_cap_'+str(side),.093 if model=='tow' else .086,.045,(0,side*length/2,.10),mat=dark)
        for j in range(3):c('folded_tripod_leg_'+str(j),.024,.79,(.12+j*.05,.06,.08),mat=dark)
        b('packed_optical_unit',(.23,.25,.24),(.22,.20,.19))
        back=max((o.matrix_world@Vector(p)).y for o in kit.parts if bone_of(o)=='spine_03' for p in o.bound_box)
        rotation=Matrix.Rotation(math.radians(24),4,'Y')@Matrix.Rotation(math.pi/2,4,'X')
        root.matrix_world=Matrix.Translation((0,back+.17,1.10))@rotation
        kit.rig.bone_parent(root,'spine_03')
        kit.parts.extend(o for o in root.children_recursive if o.type=='MESH')
        for j,along in enumerate((-.32,.32)):
            lug=root.matrix_world@Vector((-.045/SCALE,along/SCALE,.10/SCALE));anchor=Vector((j*.14-.07,back,1.10-along*.9));reach=lug-anchor
            strap=box(model+'_retention_'+str(j),(.035,.014,reach.length),(anchor+lug)/2,kit.m['webbing'],rot=reach.to_track_quat('Z','Y').to_euler())
            kit.rigid(strap,'spine_03')
        return
    # Full canister, not an enlarged missile or a generic shoulder CLU.
    c('launch_canister',.085 if model=='tow' else .078,length,(-.20,-.85,bore))
    for j,y in enumerate((-.85-length/2,-.85+length/2)):
        c('canister_end_ring_'+str(j),.094 if model=='tow' else .087,.045,(-.20,y,bore),mat=dark)
    for j,y in enumerate((-.59,-.98)):
        c('canister_clamp_'+str(j),.090 if model=='tow' else .083,.048,(-.20,y,bore),mat=dark)
    c('tripod_azimuth_head',.095,.15,(-.20,-.70,.80),'Z',dark)
    c('tripod_centre_adjuster',.035,.45,(-.20,-.70,.56),'Z',dark)
    b('elevation_cradle',(.28,.22,.13),(-.20,-.70,.94),dark)
    c('elevation_trunnion',.105,.32,(-.20,-.70,1.0),'X',dark)
    for j,foot in enumerate(((-.72,-1.16,.015),(.35,-1.14,.015),(-.48,.04,.015))):
        beam('tripod_leg_'+str(j),(-.20,-.70,.78),foot,.026)
        b('tripod_foot_'+str(j),(.095,.075,.03),foot,dark)
        beam('tripod_leg_brace_'+str(j),(-.20,-.70,.46),Vector(foot)*.55+Vector((-.09,-.315,.351)),.012,dark)
    # Optical rear eyepiece lies 8cm ahead of the actual kneeling eye.
    c('rear_eyepiece',.032,.10,(-.07,-.23,1.10),mat=dark)
    if model=='tow':
        b('optical_sight_body',(.22,.31,.28),(-.07,-.42,1.21))
        b('thermal_sight_top',(.27,.23,.095),(-.07,-.45,1.39),dark)
        c('forward_optical_lens',.062,.055,(-.07,-.595,1.22),mat=dark)
        b('control_box',(.19,.16,.17),(-.19,-.405,.96),dark)
        b('battery_control_case',(.22,.21,.21),(.035,-.11,.12),dark)
        beam('control_cable_upper',(-.06,-.46,1.07),(.02,-.39,.55),.010,dark)
        beam('control_cable_lower',(.02,-.39,.55),(.035,-.11,.23),.010,dark)
    else:
        b('below_tube_optical_unit',(.22,.30,.18),(-.07,-.42,1.07))
        c('forward_optical_lens',.047,.045,(-.07,-.59,1.075),mat=dark)
        b('tube_sight_support',(.22,.13,.065),(-.14,-.57,1.155),dark)
    for j,p in enumerate(((-.188,-.406,.945),(-.091,-.448,1.00))):
        b('operator_control_handle_'+str(j),(.038,.045,.11),p,dark)
    kit.rig.bone_parent(root,'root')
    kit.parts.extend(o for o in root.children_recursive if o.type=='MESH')
    # Kept directly under root, so kneeling arms never move the tripod bore.
    kit.rig.bone_parent(empty('muzzle',Vector((-.20,-.85-length/2,bore))/SCALE,size=.02),'root')


def build(unit, variant, mode):
    entry=next(v for v in MANIFEST['variants'] if v['id']==unit)
    if variant not in ('a','b','c') or mode not in ('active','carried'):
        raise SystemExit('Use look a/b/c and mode active/carried')
    if entry['equipment'][0] in ('Javelin','Akeron'):
        raise SystemExit('Unsupported launcher hold is gated: '+unit)
    if mode=='carried' and entry['rig_family']!='launcher':
        raise SystemExit('Only special launcher operators have a carried kit')
    model=entry['equipment_model']
    ground=model in ('tow','kornet')
    shoulder=model.startswith('rpg') and mode=='active'
    hold=weapons.LAUNCHER_HOLD if (shoulder or ground and mode=='active') else weapons.RIFLE_HOLD
    kind='recon' if entry['profile'] in ('scout','elite-scout','sniper') else 'rifle'
    rig=Rig()
    for name in ('Eyes','Eyebrows'):
        if name in bpy.data.objects: bpy.data.objects.remove(bpy.data.objects[name],do_unlink=True)
    kit=Kit(rig,look_of(kind,variant))
    kit.uniform();kit.boots();kit.plate_carrier();kit.molle()
    if kit.look['pack']!='none':kit.pack_straps()
    kit.belt();kit.chest_rig();kit.pack(kind);kit.hip_kit();kit.markings();kit.head_gear()
    if kit.look['scarf']:kit.scarf()
    kit.skin()
    if 'grenade' in entry['equipment']:
        for j,bearing in enumerate((-42,42)):
            at,out=kit.on_belt(bearing,.99);centre=at+out*.052
            grenade=cyl('fragmentation_grenade_'+str(j),.037,.070,centre,'Z',kit.m['launcher'],seg=12,bevel_=.014)
            kit.rigid(grenade,'pelvis')
            neck=cyl('grenade_fuze_'+str(j),.012,.022,centre+Vector((0,0,.042)),'Z',kit.m['gun_black'],seg=8)
            kit.rigid(neck,'pelvis')
            lever=box('grenade_safety_lever_'+str(j),(.011,.017,.070),centre+Vector((.033,0,.015)),kit.m['gun_black'],rot=(0,math.radians(-12),0))
            kit.rigid(lever,'pelvis')
    if model.startswith('rpg') and mode=='carried':
        root=rpg_equipment(kit.m,model,False)
        # A rifle in the hands and the actual launcher slung behind the pack.
        back=max((o.matrix_world@Vector(c)).y for o in kit.parts if bone_of(o)=='spine_03' for c in o.bound_box)
        rotation=Matrix.Rotation(math.radians(30),4,'Y')@Matrix.Rotation(math.pi/2,4,'X')
        root.matrix_world=Matrix.Translation((0,back+.13,1.1))@rotation
        rig.bone_parent(root,'spine_03')
        kit.parts.extend(c for c in root.children_recursive if c.type=='MESH')
        for j,along in enumerate((-.20,.30)):
            lug=root.matrix_world@Vector((-.045,along,.10))
            anchor=Vector((-.09 if j==0 else .09,back,1.1-along*.85))
            reach=lug-anchor
            strap=box('packed_rpg_retention_'+str(j),(.035,.014,reach.length),(anchor+lug)/2,kit.m['webbing'],rot=reach.to_track_quat('Z','Y').to_euler())
            kit.rigid(strap,'spine_03')
    if ground:tripod_equipment(kit,model,mode=='carried')
    if not ground or mode=='carried':
        constructor=(lambda m:rpg_equipment(m,model)) if shoulder else (lambda m:rifle_equipment(m,'carbine' if mode=='carried' else model))
        kit.weapon({'hold':hold,'build':constructor})
    kit.eye();kit.paint_all()
    soldier=kit.join();kit.bake_ao(soldier)
    tiers=make_tiers(soldier,'soldier')
    for t in tiers:
        canonical(t);t.parent=rig.arm;t.modifiers.new('armature','ARMATURE').object=rig.arm
    rig.arm.scale=(SCALE,)*3;bpy.context.view_layer.update();texture_uvs(tiers)
    for obj in list(bpy.data.objects):
        if obj.type=='EMPTY' and obj.name not in ('eye','muzzle'):bpy.data.objects.remove(obj,do_unlink=True)
    path=os.path.join(REPO,entry['export_directory'],mode+'_'+variant+'.glb')
    os.makedirs(os.path.dirname(path),exist_ok=True)
    export_glb(path,[rig.arm,*tiers,bpy.data.objects['eye'],bpy.data.objects['muzzle']])
    print('ROSTER_INFANTRY',json.dumps({'id':unit,'equipment':model,'variant':variant,'mode':mode,'skeleton':'quaternius-ubc-'+hold['family'],'tris':[triangle_count(t) for t in tiers],'path':path}))
    return path,hold


def preview(path,hold,directory):
    """Reimport source and shared clip GLB; poses are the actual exported clips."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    body_arm=next(o for o in bpy.data.objects if o.type=='ARMATURE')
    clip=os.path.join(REPO,'assets/source/infantry/clips_'+hold['family']+'.glb')
    if open(clip,'rb').read(4)!=b'glTF':
        raise SystemExit('Preview requires acquired shared clip source: '+clip)
    before=set(bpy.data.objects);bpy.ops.import_scene.gltf(filepath=clip)
    actions={a.name:a for a in bpy.data.actions}
    for o in set(bpy.data.objects)-before:bpy.data.objects.remove(o,do_unlink=True)
    for o in bpy.data.objects:
        if o.type=='MESH':o.hide_render='_LOD' in o.name and not o.name.endswith('_LOD0')
    scn=bpy.context.scene;scn.render.engine='BLENDER_EEVEE';scn.render.resolution_x=800;scn.render.resolution_y=800
    scn.world=bpy.data.worlds.new('review_world');scn.world.use_nodes=True;scn.world.node_tree.nodes['Background'].inputs['Color'].default_value=(.32,.32,.32,1);scn.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.8
    scn.view_settings.exposure=1.0
    bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.005))
    floor=bpy.context.object;floor.name='review_ground';floor_mat=bpy.data.materials.new('review_ground');floor_mat.diffuse_color=(.22,.24,.20,1);floor.data.materials.append(floor_mat)
    for loc in ((-3,-4,6),(4,3,4)):
        bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=450;light.data.size=4;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add();cam=bpy.context.object;scn.camera=cam;cam.data.type='ORTHO'
    os.makedirs(directory,exist_ok=True)
    for pose in ('idle','stand_aim','kneel_fire','prone_pinned'):
        body_arm.animation_data_create();action=next(a for n,a in actions.items() if n.split('.')[0]==pose)
        body_arm.animation_data.action=action
        if action.slots:body_arm.animation_data.action_slot=action.slots[0]
        scn.frame_set(0);bpy.context.view_layer.update()
        dg=bpy.context.evaluated_depsgraph_get()
        points=[o.matrix_world@Vector(p) for o in bpy.data.objects if o.type=='MESH' and o.name.endswith('_LOD0') for p in o.evaluated_get(dg).bound_box]
        lo=Vector(tuple(min(p[i] for p in points) for i in range(3)));hi=Vector(tuple(max(p[i] for p in points) for i in range(3)));target=(lo+hi)/2
        cam.data.ortho_scale=max(hi.y-lo.y,hi.z-lo.z)*1.25
        for label,offset in [('side',(-4,-.1,.6)),('front',(-2.7,-4,1.15))]:
            cam.location=target+Vector(offset);cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler();scn.render.filepath=os.path.join(directory,pose+'_'+label+'.png');bpy.ops.render.render(write_still=True)


if __name__=='__main__':
    args=script_args();unit=args[0];variant=args[1] if len(args)>1 and not args[1].startswith('--') else 'a';mode=args[2] if len(args)>2 and not args[2].startswith('--') else 'active'
    target=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    if '--preview-only' in args:
        entry=next(v for v in MANIFEST['variants'] if v['id']==unit)
        path=os.path.join(REPO,entry['export_directory'],mode+'_'+variant+'.glb')
        hold=weapons.LAUNCHER_HOLD if entry['rig_family']=='launcher' and mode=='active' else weapons.RIFLE_HOLD
    else:
        path,hold=build(unit,variant,mode)
    if target:preview(path,hold,os.path.abspath(target))
