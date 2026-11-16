"""The team rifles and light machine guns the roster's kits carry (marksman and sniper
rifles; each army's light machine gun), built to their real lengths. The service
rifles are `weapons.RIFLES`."""
import math

import bmesh
from common import box, cyl, empty, obj_from_bm
from infantry_rig import SCALE
import weapons

# Lengths, world metres: complete unsuppressed rifles; light machine guns with the
# stock extended.
LENGTHS = {
    "m110": dict(length_m=1.029), "g28": dict(length_m=1.08), "svd": dict(length_m=1.225),
    "m107": dict(length_m=1.448), "asvk": dict(length_m=1.42),
    "m249": dict(length_m=1.04), "mg4": dict(length_m=1.03), "pkp": dict(length_m=1.155),
}


def rifle_equipment(mats, model):
    """A marksman's or sniper's rifle: distinct receiver, stock and optic
    layouts on the shared wrist anchors. The M110 is flat dark earth, the
    G28 its sand twin (references: both AR-10 pattern) with a sand scope."""
    root = empty(model, (0, 0, 0))
    black = mats['gun_black']
    finish = mats['gun_fde'] if model in ('m110', 'g28') else black
    optic = finish if model == 'g28' else black
    # Named references are complete unsuppressed weapon lengths. Fixed rear
    # anchor preserves shoulder fit; the barrel consumes the remaining length.
    length = LENGTHS[model]['length_m']
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
    slim = model in ('m110', 'g28', 'svd')
    c('scope', .022 if slim else .029, .29, (0, -.075, .11), optic)
    c('objective', .031 if slim else .038, .065, (0, -.23, .11), optic)
    b('optic_mount', (.022, .12, .028), (0, -.05, .080))
    cyl(model+'_scope_turret', .015, .047, (0, -.08, .143), 'Z', optic, root, seg=10)
    if model in ('m110', 'g28'):
        b('upper_receiver', (.039, .27, .06), (0, -.06, .043), finish)
        b('lower_receiver', (.036, .18, .05), (0, .0, -.005), finish)
        b('straight_magazine', (.032, .085, .12), (0, -.08, -.09))
        if model == 'm110':
            b('fixed_stock', (.045, .23, .09), (0, .285, .031), finish)
        else:  # the G28's adjustable stock: a slim butt, a tall cheek riser
            b('adjustable_stock', (.042, .2, .075), (0, .29, .022), finish)
            b('cheek_riser', (.036, .13, .03), (0, .27, .075), finish)
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


def lmg_equipment(mats, model):
    """A squad's light machine gun on the rifle's wrist anchors: a deep
    receiver under a hinged feed cover, the belt's box hanging under the
    left of it, a carry handle over the barrel and a folded bipod. The M249
    has the ribbed heat shield and the fixed stock; the MG4 is all polymer,
    with a long top rail and a side-folding skeleton stock; the PKP is the
    PK's slab receiver, its heavy fluted barrel, the shroud and handle near
    the muzzle and the skeleton stock (references: assets/references/
    machine_gunner/)."""
    root = empty(model, (0, 0, 0))
    black, poly = mats['gun_black'], mats['gun_polymer']
    length = LENGTHS[model]['length_m']
    rear = .40
    front = rear - length / (SCALE * weapons.RIFLE_SCALE)
    bore = .045

    def b(name, size, loc, mat=black, rot=(0, 0, 0)):
        return box(model+'_'+name, size, loc, mat, root, rot=rot, bevel_=.003)

    def c(name, radius, depth, loc, mat=black, axis='Y'):
        return cyl(model+'_'+name, radius, depth, loc, axis, mat, root, seg=12)

    b('pistol_grip', (.032, .042, .10), (0, .06, -.05), poly, rot=(math.radians(20), 0, 0))
    b('trigger_guard', (.034, .065, .013), (0, .017, -.044))
    pkp = model == 'pkp'
    # Receiver and feed cover: the gun's deep, square middle.
    b('receiver', (.05, .30, .08), (0, -.08, .03), poly if model == 'mg4' else black)
    b('feed_cover', (.054, .17, .024), (0, -.07, .082), poly if model == 'mg4' else black)
    b('feed_tray', (.07, .05, .02), (.035, -.10, .05))
    rail = .34 if model == 'mg4' else .14
    b('top_rail', (.024, rail, .012), (0, -.07 - (rail - .14) / 2, .1))
    b('optic', (.032, .07, .04), (0, -.04, .126))
    # The belt's box (the M249's soft pouch, the MG4's and PKP's hard box)
    # under the receiver's left, the belt rising into the feed tray.
    box_size = (.08, .12, .12) if pkp else (.075, .11, .12)
    b('ammo_box', box_size, (.03, -.10, -.065), mats['launcher'] if model == 'm249' else black)
    b('belt', (.012, .04, .07), (.066, -.10, .02), mats['marking'])
    if model == 'm249':
        b('stock', (.046, .22, .09), (0, .29, .025))
        b('stock_tube', (.03, .1, .03), (0, .15, .04))
        b('butt_pad', (.05, .022, .12), (0, rear - .011, .02))
        b('heat_shield', (.05, .22, .04), (0, -.33, .075))
        for j in range(4):
            b('shield_rib_%s' % j, (.054, .012, .044), (0, -.25 - j*.05, .075))
        b('handguard', (.058, .14, .05), (0, -.29, .0), poly)
    elif model == 'mg4':
        b('stock_upper', (.03, .24, .026), (0, .26, .062), poly)
        b('stock_lower', (.026, .23, .022), (0, .265, -.012), poly, rot=(math.radians(-6), 0, 0))
        b('stock_hinge', (.036, .05, .07), (0, .11, .03), poly)
        b('butt_plate', (.044, .03, .12), (0, rear - .015, .02), poly)
        b('handguard', (.058, .18, .055), (0, -.30, .035), poly)
    else:
        # The PKP's open skeleton stock and thumbhole, in black polymer.
        b('stock_top', (.036, .26, .028), (0, .265, .05), poly)
        b('stock_bottom', (.03, .24, .022), (0, .25, -.05), poly, rot=(math.radians(-10), 0, 0))
        b('buttplate', (.042, .027, .13), (0, rear - .013, .0), poly)
        b('receiver_rivets', (.052, .2, .01), (0, -.09, .0))
    # The barrel, with the PKP's fat finned barrel and muzzle shroud.
    barrel_start = -.23
    end = front + .06
    c('barrel', .022 if pkp else .013, barrel_start - end, (0, (barrel_start + end) / 2, bore))
    if pkp:
        c('barrel_shroud', .03, .26, (0, front + .2, bore))
        for j in range(4):
            c('cooling_fin_%s' % j, .028, .012, (0, -.30 - j*.06, bore))
    c('flash_hider', .02 if pkp else .017, .06, (0, front + .03, bore))
    c('gas_tube', .01, .25, (0, -.35, bore - .03))
    # The carry handle: the M249's over the barrel's breech, the MG4's and
    # PKP's near the middle of the barrel.
    handle_y = -.30 if model == 'm249' else front + .3
    b('carry_handle', (.016, .14, .022), (.0, handle_y, bore + .085))
    for k in (-1, 1):
        b('handle_post_%s' % k, (.014, .018, .07), (0, handle_y + k*.06, bore + .05))
    b('front_sight', (.02, .02, .05), (0, front + .1, bore + .04))
    # Folded legs are carried, as on the marksman's rifle.
    for side in (-1, 1):
        b('folded_bipod_'+str(side), (.014, .26, .020), (side*.026, front + .26, bore - .026))
    c('bipod_hinge', .02, .03, (0, front + .13, bore - .018))
    empty('muzzle', (0, front, bore), root, .02)
    return root
