"""Named roster equipment on the unchanged Quaternius infantry rig.

Usage: infantry_equipment.py <kit> [a|b|c] [active|carried] [--army=<army>] [--preview=<dir>]
       infantry_equipment.py <kit> --receipt
Add --preview-only to inspect the existing source without rebuilding it;
--receipt writes the kit's receipt once every look is exported.
Writes into the kit's own directory, `assets/source/roster/infantry/<kit>/`.
A kit is built in one of its armies' looks (`KITS`, its first by default):
`<mode>_<look>.glb` for the first, `<army>_<mode>_<look>.glb` for the
others. Rifle-family grip, support and sight anchors stay identical to the
shared clips. Lengths are world metres; geometry compensates for the shared
hold and soldier scales, never the rig. Tripod ATGMs use a root-weighted kit
and the coordinator's frozen active pose.
"""
import hashlib
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import bpy
from mathutils import Matrix, Vector
from common import REPO, box, cyl, empty, fbm, script_args
from infantry_kit import SLUNG_MIDDLE_M, dressed, export_soldier, scale3
from infantry_rig import SCALE, back_y
from mesh_lods import triangle_count
import weapons
import skin_pins
from infantry_roster import KITS, kind_of, source_path
from infantry_preview import preview
from support_weapons import lmg_equipment, rifle_equipment

# Team launchers' lengths, world metres (the roster's equipment authoring):
# launcher tubes loaded and packed; tripod ATGMs' canister and bore height
# against the shared kneel_fire pose. Team rifles' and LMGs' are support_weapons'.
LENGTHS = {
    "rpg7": dict(length_m=0.95, packed_length_m=0.95), "rpg29": dict(length_m=1.85, packed_length_m=1.0),
    "tow": dict(tube_length_m=1.28, bore_m=1.066), "kornet": dict(tube_length_m=1.32, bore_m=1.18),
    # Shoulder-fired launch tubes (references: assets/references/<kit>/).
    "javelin": dict(length_m=1.20, radius_m=0.071), "stinger": dict(length_m=1.52, radius_m=0.035),
    "igla": dict(length_m=1.70, radius_m=0.036), "fn6": dict(length_m=1.49, radius_m=0.036),
    "akeron": dict(length_m=1.25, radius_m=0.075),
}
# Launchers fired from the shoulder (the launcher hold), and those on a tripod.
SHOULDER = ("rpg7", "rpg29", "javelin", "stinger", "igla", "fn6", "akeron")
# The slim air-defence tubes among them.
MANPADS = ("stinger", "igla", "fn6")
# A launch tube painted other than the kit's launcher olive: the Akeron's is
# a light olive between its black caps (references: assets/references/<kit>/).
TUBE_PAINT = {"akeron": (0.24, 0.25, 0.16)}
LAUNCHERS = ("tow", "kornet", *SHOULDER)


def rpg_equipment(mats, model, socket=True):
    root = empty(model, (0, 0, 0))
    black, tube = mats['gun_black'], mats['launcher']
    frame = LENGTHS[model]
    length = frame['length_m' if socket else 'packed_length_m'] / SCALE
    rear = .42 if model == 'rpg7' else .69
    front = rear-length
    # The RPG-7 is a blued steel tube; the RPG-29's is olive.
    cyl(model+'_tube', .025 if model=='rpg7' else .052, length, (0,(front+rear)/2,.10), 'Y', black if model=='rpg7' else tube, root, seg=20)
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


def shoulder_tube_equipment(mats, model, socket=True):
    """A shoulder-fired missile on the launcher hold's grip and support
    points: its launch tube, end caps, the grip under it and its sight on the
    left (the hold's sight side) -- the Javelin's command launch unit, the
    Akeron's sight block, a MANPADS' gripstock, folding sight frame and IFF
    antenna. Each is drawn after its references' silhouette. Packed (no
    socket) it is the same launcher, slung."""
    root = empty(model, (0, 0, 0))
    black, tube = mats['gun_black'], mats['launcher']
    frame = LENGTHS[model]
    length = frame['length_m'] / SCALE
    r = frame['radius_m']
    heavy = model in ('javelin', 'akeron')
    # A heavy tube (Javelin, Akeron) rides centred on the shoulder, half of
    # it behind; a MANPADS mostly ahead.
    rear = .80 if heavy else .45
    front = rear-length
    axis = .10  # the bore on the hold's aim line, as the RPG's is
    cyl(model+'_tube', r, length, (0,(front+rear)/2,axis), 'Y', tube, root, seg=20)
    # End caps: the Akeron's oversized black shock absorbers, the Javelin's
    # bulbous caps, a MANPADS' slim ones.
    cap_r, cap_l = {'akeron': (1.6, .16), 'javelin': (1.25, .10)}.get(model, (1.15, .06))
    inset = max(.05, cap_l/2)
    for k,y in enumerate((rear-inset, front+inset)):
        cyl(model+'_end_cap_'+str(k), r*cap_r, cap_l, (0,y,axis), 'Y', black, root, seg=20)
    box(model+'_trigger_grip', (.032,.04,.15), (0,.06,.025), black, root, bevel_=.005)
    box(model+'_support_grip', (.035,.045,.11), (.0,-.25,.0), black, root, bevel_=.005)
    # Every tube: its carrying sling's two swivels, and the ribbed rings
    # either side of the grip where the tube's halves join.
    for k,y in enumerate((rear-.20, front+.20)):
        box(model+'_sling_swivel_'+str(k), (.012,.03,.03), (-r-.01,y,axis), black, root)
    for k,y in enumerate((.18, -.38)):
        cyl(model+'_band_'+str(k), r*1.06, .025, (0,y,axis), 'Y', black, root, seg=20)
    if model == 'javelin':
        # The command launch unit clamped to the tube's left, its eyepiece at
        # the gunner's face: its dark body (it must not melt into a tan
        # uniform), two handles, the eyepiece's rubber eyecup, the day
        # sight's and the thermal sight's windows on its front, the display
        # controls; the battery coolant unit under the tube's rear.
        box('javelin_clu', (.20,.26,.20), (.14,.0,axis+.02), black, root, bevel_=.02)
        for k,y in enumerate((.09,-.09)):
            box('javelin_clu_handle_'+str(k), (.03,.03,.14), (.25,y,axis+.0), black, root, bevel_=.006)
        box('javelin_clu_eyepiece', (.07,.06,.06), (.14,.15,axis+.06), black, root, bevel_=.01)
        cyl('javelin_clu_eyecup', .032, .03, (.14,.19,axis+.06), 'Y', black, root, seg=14)
        cyl('javelin_clu_lens', .045, .02, (.14,-.14,axis+.03), 'Y', mats['lens'], root, seg=16)
        box('javelin_clu_day_window', (.05,.012,.04), (.20,-.135,axis+.08), mats['lens'], root)
        for k in range(3):
            box('javelin_clu_switch_'+str(k), (.02,.02,.012), (.10+k*.03,.06,axis+.125), black, root)
        cyl('javelin_bcu', .045, .16, (0,rear-.20,axis-r-.04), 'Y', black, root, seg=14)
        box('javelin_bcu_clamp', (.06,.04,.05), (0,rear-.20,axis-r-.01), black, root)
        muzzle = front
    elif model == 'akeron':
        # The firing post's sight block on the left, its hood and the
        # thermal channel's window, the shoulder rest under the shoulder.
        box('akeron_sight', (.16,.22,.16), (.13,-.05,axis+.02), black, root, bevel_=.015)
        box('akeron_sight_hood', (.17,.06,.03), (.13,-.15,axis+.11), black, root)
        cyl('akeron_sight_lens', .04, .02, (.13,-.17,axis+.04), 'Y', mats['lens'], root, seg=14)
        box('akeron_thermal_window', (.05,.012,.05), (.08,-.165,axis-.02), mats['lens'], root)
        cyl('akeron_eyecup', .03, .04, (.13,.07,axis+.06), 'Y', black, root, seg=14)
        box('akeron_shoulder_rest', (.05,.16,.09), (0,.30,axis-r-.03), black, root, bevel_=.01)
        muzzle = front
    else:
        # MANPADS: the gripstock under the tube with its battery/coolant
        # unit, the sight's front and rear frames on its left, the seeker's
        # nose cap; the Stinger's IFF antenna grid folded over the front,
        # the Igla's and FN-6's interrogator box on the gripstock.
        box(model+'_gripstock', (.05,.30,.07), (0,-.05,axis-.06), black, root, bevel_=.006)
        box(model+'_battery', (.05,.05,.10), (0,-.20,axis-.12), black, root, bevel_=.006)
        cyl(model+'_bcu', .025, .12, (0,-.26,axis-.10), 'Z', black, root, seg=12)
        box(model+'_sight_frame', (.012,.10,.08), (.07,-.10,axis+.03), black, root)
        box(model+'_rear_sight', (.012,.03,.05), (.07,.10,axis+.04), black, root)
        box(model+'_sight_arm', (.06,.02,.02), (.04,-.10,axis), black, root)
        if model == 'stinger':
            box('stinger_iff_antenna', (.006,.16,.12), (0,front+.40,axis+.10), black, root)
            for k in range(3):
                box('stinger_iff_rib_'+str(k), (.008,.006,.12), (0,front+.34+k*.06,axis+.10), black, root)
        else:
            box(model+'_interrogator', (.07,.10,.08), (.06,.0,axis-.06), black, root, bevel_=.008)
        cap = .07 if model == 'igla' else .05
        cyl(model+'_nose_cap', r*1.2, cap, (0,front-cap/2,axis), 'Y', black, root, seg=16)
        muzzle = front-cap
    if socket:
        empty('muzzle',(0,muzzle,axis),root,.02)
    return root


def tripod_equipment(kit, model, carried=False):
    """Actual tube, optics and tripod on root; packed tube/legs ride the back.

    Coordinates here are world metres in the source's -Y-forward frame, chosen
    against the reimported shared kneel_fire phase0. Root weighting keeps the
    three feet grounded when the operator's pelvis/arms move into that pose.
    """
    frame=LENGTHS[model]
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
        back=back_y(kit.parts)
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


def hold_of(unit, mode):
    """The clip family a kit's soldier is carried with: a launcher in the
    hands (the shoulder RPG, the tripod ATGM's operator) or a rifle."""
    return weapons.LAUNCHER_HOLD if KITS[unit]['weapon'] in LAUNCHERS and mode=='active' else weapons.RIFLE_HOLD


def build(unit, variant, mode, army=None):
    if unit not in KITS:
        raise SystemExit(f'{unit} is no roster kit; kits: {", ".join(KITS)}')
    spec=KITS[unit];armies=spec['armies']
    army=army or armies[0]
    if army not in armies:
        raise SystemExit(f'{unit} has no {army} look; its armies: {", ".join(armies)}')
    if variant not in ('a','b','c') or mode not in ('active','carried'):
        raise SystemExit('Use look a/b/c and mode active/carried')
    model=spec['weapon']
    if mode=='carried' and model not in LAUNCHERS:
        raise SystemExit('Only launcher operators have a carried kit')
    ground=model in ('tow','kornet')
    shoulder=model in SHOULDER and mode=='active'
    hold=hold_of(unit,mode)
    kit=dressed(army,variant,kind_of(unit))
    if model in SHOULDER and mode=='carried':
        rpg=model.startswith('rpg')
        root=rpg_equipment(kit.m,model,False) if rpg else shoulder_tube_equipment(kit.m,model,False)
        # A rifle in the hands and the actual launcher slung behind the pack:
        # an RPG muzzle down; a missile tube muzzle up, with its sight (the
        # Javelin's CLU) facing out rather than into the pack.
        # A tube's middle at SLUNG_MIDDLE_M, whatever length rides behind its
        # grip, lowered for a tube longer than the Stinger's (the Igla): its
        # upper cap stays under the soldier's height at the sling's 30° lean.
        overlong=max(0,LENGTHS[model].get('length_m',0)-LENGTHS['stinger']['length_m'])
        lift=1.1 if rpg else SLUNG_MIDDLE_M+bpy.data.objects[model+'_tube'].location.y-overlong*math.cos(math.radians(30))/2
        # A slim MANPADS tube lies on the pack by its own radius, its foot
        # tipped in to follow the soldier's forward lean; the Javelin's and
        # Akeron's sight blocks fill that gap themselves.
        slim=model in MANPADS
        standoff=.13 if rpg else LENGTHS[model]['radius_m']+.01-.10 if slim else -.02
        kit.sling(root,lift,standoff,'packed_'+('rpg' if rpg else model),muzzle_up=not rpg,tip_deg=10 if slim else 0)
    if ground:tripod_equipment(kit,model,mode=='carried')
    if not ground or mode=='carried':
        # A launcher team's carrier holds his army's rifle, as a rifleman does.
        constructor=((lambda m:rpg_equipment(m,model)) if shoulder and model.startswith('rpg')
                     else (lambda m:shoulder_tube_equipment(m,model)) if shoulder
                     else (lambda m:weapons.RIFLES[kit.army['rifle']](m)) if model=='rifle' or mode=='carried'
                     else (lambda m:lmg_equipment(m,kit.army['lmg'])) if model=='lmg'
                     else (lambda m:rifle_equipment(m,model)))
        kit.weapon({'hold':hold,'build':constructor})
    if model in TUBE_PAINT:
        rgb=TUBE_PAINT[model]
        kit.painters[model+'_tube']=lambda p,n:(scale3(rgb,.9+.2*fbm(p,40,97)),.3+.2*fbm(p,12,98))
    path=source_path(unit,variant,mode,army)
    tiers=export_soldier(kit,path)
    print('ROSTER_INFANTRY',json.dumps({'id':unit,'army':army,'equipment':model,'variant':variant,'mode':mode,'skeleton':'quaternius-ubc-'+hold['family'],'tris':[triangle_count(t) for t in tiers],'path':path}))
    return path,hold


def receipt(unit):
    """Write a kit's receipt beside its sources (`source-receipt.json`): the
    hash of every script of this directory the export loads, its reference
    library, and each of its exported looks' bytes. Every look must already be
    exported (each export is its own Blender run)."""
    blender=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    loaded={os.path.abspath(__file__)}|{os.path.abspath(m.__file__) for m in list(sys.modules.values())
                                         if getattr(m,'__file__',None) and os.path.abspath(m.__file__).startswith(blender+os.sep)}
    spec=KITS[unit]
    modes=('active','carried') if spec['weapon'] in LAUNCHERS else ('active',)
    variants=[]
    for army in spec['armies']:
        for mode in modes:
            for look in 'abc':
                path=source_path(unit,look,mode,army)
                data=open(path,'rb').read() if os.path.exists(path) else b''
                if data[:4]!=b'glTF':
                    raise SystemExit('Export every look before its receipt; missing or not fetched: '+path)
                variants.append({'army':army,'mode':mode,'look':look,'source':os.path.relpath(path,REPO),
                                 'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
    sha=lambda p:hashlib.sha256(open(p,'rb').read()).hexdigest()
    loaded.add(os.path.join(os.path.dirname(os.path.abspath(__file__)),'armies.json'))
    # The generated bodies its looks wore (`skin_pins`), their pinned hashes.
    pins=[skin_pins.required('infantry',army=army,look=look,kind=kind_of(unit)) for army in spec['armies'] for look in 'abc']
    worn={p['glb']:p['sha256'] for p in pins}
    out=os.path.join(REPO,'assets/source/roster/infantry',unit,'source-receipt.json')
    with open(out,'w') as f:
        f.write(json.dumps({
            'blender_version':bpy.app.version_string,
            'source_sha256':{os.path.relpath(p,REPO):sha(p) for p in sorted(loaded)},
            'references':f'assets/references/{unit}/references.json',
            'authoring':('Generated bodies (`skins`, pinned in blender/skins/) on the shared Quaternius rig, with the '
                         'procedural weapon and fittings built from the committed references; photos are visual '
                         'reference only.'),
            'skins':worn,
            'variants':variants,
        },indent=2)+'\n')
    print('ROSTER_INFANTRY_RECEIPT',out)


if __name__=='__main__':
    args=script_args();unit=args[0];variant=args[1] if len(args)>1 and not args[1].startswith('--') else 'a';mode=args[2] if len(args)>2 and not args[2].startswith('--') else 'active'
    target=next((a.split('=',1)[1] for a in args if a.startswith('--preview=')),None)
    army=next((a.split('=',1)[1] for a in args if a.startswith('--army=')),None)
    if '--receipt' in args:
        receipt(unit);sys.exit(0)
    if '--preview-only' in args:
        path=source_path(unit,variant,mode,army)
        hold=hold_of(unit,mode)
    else:
        path,hold=build(unit,variant,mode,army)
    if target:preview(path,hold,os.path.abspath(target))
