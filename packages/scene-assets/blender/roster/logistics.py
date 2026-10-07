"""Worker-owned authoring for the three named cargo truck families.

This module shares chassis/detail construction only. Family-specific cab profiles,
axles, dimensions and crane equipment remain explicit in their entrypoints.
It uses the scene-assets primitive, texture, tier and exporter owners unchanged.
"""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parts import *
from catalog_frames import requested_variant


def build(FAMILY, AXLES, CAB_STYLE):
    ARGS = script_args()
    VARIANT, OUT = requested_variant(FAMILY, ARGS)
    TYPE_ID = VARIANT['id']
    L, W, H = VARIANT['frame']['body_dimensions_m']
    PREVIEW = next((a.split('=', 1)[1] for a in ARGS if a.startswith('--preview=')), None)
    reset()
    paint_m = textured('cargo_olive', 'olive_paint', tint=1.0, chip=.35, dirt=.65, rise=1.25)
    chassis_m = textured('chassis_dark', 'olive_paint', colour=(.047,.056,.035), chip=.25, dirt=.7)
    rubber_m = textured('rubber', 'rubber', dirt=.3, chip=0, rise=.9)
    canvas_m = textured('canvas', 'canvas', colour=(.14,.17,.105), chip=.1, dirt=.35)
    steel_m = textured('steel', 'bare_steel', chip=.3, dirt=.45)
    wood_m = textured('supply_wood', 'pallet_wood', dirt=.4)
    glass_m = flat_paint('glass', (.028,.059,.062), rough=.13, grime=.1)
    black_m = flat_paint('grille', (.018,.025,.018), rough=.8, grime=.3)
    lamp_m = flat_paint('lamp', (.62,.58,.41), rough=.25, grime=.05)
    red_m = flat_paint('tail_lamp', (.17,.025,.012), rough=.3, grime=.05)
    truck = empty(FAMILY.replace('-','_'))
    body = empty('body', parent=truck)
    R = .63 if FAMILY=='hemtt' else (.67 if FAMILY=='man-hx' else .57)
    TYRE_DEPTH = .40 if FAMILY!='ural' else .39
    WHEEL_Y = W/2 - TYRE_DEPTH/2 - .025

    # Open twin-rail chassis. Body-center ground origin matches the frozen frame.
    for side in (-1,1):
     box(f'chassis_rail_{side}',(L-.4,.12,.22),(0,side*.42,.84),chassis_m,body)
    for i,x in enumerate([-L*.40,-L*.19,0,L*.20,L*.40]):
     box(f'chassis_cross_{i}',(.12,1.0,.16),(x,0,.84),chassis_m,body,lods=(0,1,2))
    for i,(row,x) in enumerate(AXLES):
     cyl(f'axle_{row}',.10,W-.30,(x,0,R),'Y',chassis_m,body,seg=12,lods=(0,1,2))
     cyl(f'diff_{row}',.21,.36,(x,0,R),'Y',chassis_m,body,seg=12,lods=(0,1))
     for side in (-1,1):
      sn='L' if side>0 else 'R'; node=empty(f'wheel_{row}{sn}',(x,side*WHEEL_Y,R),body,props={'radius_m':R})
      cyl(f'tyre_{row}{sn}',R,TYRE_DEPTH,(0,0,0),'Y',rubber_m,node,seg=32)
      cyl(f'tyre_wall_{row}{sn}',R*.87,.026,(0,side*(TYRE_DEPTH/2+.007),0),'Y',rubber_m,node,seg=24,lods=(0,1,2))
      cyl(f'rim_{row}{sn}',R*.53,.055,(0,side*(TYRE_DEPTH/2+.013),0),'Y',paint_m,node,seg=24)
      cyl(f'hub_{row}{sn}',R*.20,.09,(0,side*(TYRE_DEPTH/2+.045),0),'Y',chassis_m,node,seg=12,lods=(0,1,2))
      for j in range(8):
       a=j*math.tau/8
       cyl(f'rim_bolt_{row}{sn}_{j}',.022,.04,(R*.34*math.sin(a),side*(TYRE_DEPTH/2+.06),R*.34*math.cos(a)),'Y',steel_m,node,seg=6,lods=(0,))
      for j in range(20):
       a=j*math.tau/20
       # Raised tread, smaller than ground tolerance; bottom tread is seated at z=0.
       box(f'tread_{row}{sn}_{j}',(.13,TYRE_DEPTH*.76,.030),((R-.015)*math.sin(a),0,(R-.015)*math.cos(a)),rubber_m,node,rot=(0,a,side*.2),lods=(0,1))
      box(f'leaf_spring_{row}{sn}',(.8,.10,.10),(x,side*.68,R+.22),chassis_m,body,bevel=.01,lods=(0,1))
      box(f'mudflap_{row}{sn}',(.04,.48,.40),(x-R-.07,side*WHEEL_Y,R*.63),rubber_m,body,lods=(0,1,2))

    if CAB_STYLE=='bonnet':
     cab_back=.25; cab_front=1.85; cab_top=2.68; cab_width=2.05
     prism('ural_cab',[ (cab_back,1.10),(cab_front,1.10),(cab_front,2.12),(1.62,2.62),(.43,2.68),(cab_back,2.48)],cab_width,mat=paint_m,parent=body,bevel=.07)
     # Rounded bonnet and high vertical grille are the Ural's primary recognition cues.
     loft('bonnet',[(1.30,[(1.68,-.625),(3.42,-.625),(3.42,.625),(1.68,.625)]),(1.96,[(1.66,-.625),(3.43,-.625),(3.43,.625),(1.66,.625)]),(2.10,[(1.66,-.55),(3.25,-.55),(3.25,.55),(1.66,.55)]),(2.16,[(1.66,-.46),(3.10,-.46),(3.10,.46),(1.66,.46)])],paint_m,body,bevel=.045)
     box('bonnet_center_ridge',(1.68,.025,.02),(2.54,0,2.15),paint_m,body,lods=(0,1))
     box('grille_back',(.055,1.15,.72),(3.465,0,1.77),black_m,body,bevel=.04)
     for j in range(9):box(f'grille_vertical_{j}',(.045,.042,.68),(3.495,-.50+j*.125,1.78),paint_m,body,bevel=.018,lods=(0,1,2))
     box('front_bumper',(.22,2.32,.25),(L/2-.11,0,.82),chassis_m,body,bevel=.025)
     for side in (-1,1):
      sn='L' if side>0 else 'R'
      arch_angles=[math.radians(15+j*150/10) for j in range(11)]
      arch=[(2.48+.79*math.cos(a),R+.79*math.sin(a)) for a in arch_angles]
      arch += [(2.48+.72*math.cos(a),R+.72*math.sin(a)) for a in reversed(arch_angles)]
      prism(f'front_fender_{sn}',arch,.49,loc=(0,side*.91,0),mat=paint_m,parent=body,bevel=.012)
      cyl(f'headlamp_{sn}',.115,.06,(3.33,side*.89,1.59),'X',lamp_m,body,seg=20)
      cyl(f'headlamp_rim_{sn}',.145,.045,(3.29,side*.89,1.59),'X',steel_m,body,seg=20,lods=(0,1))
      if side>0:
       cyl('air_filter',.20,.54,(1.93,.77,2.15),'X',chassis_m,body,seg=20,lods=(0,1,2))
      else:
       cyl('exhaust_stack',.06,1.40,(.32,-.91,2.15),'Z',chassis_m,body,seg=12,lods=(0,1,2))
     # Windscreen pitched back, center divider, both side windows.
     for side in (-1,1):
      box(f'windscreen_{side}',(.035,.86,.49),(1.754,side*.47,2.34),glass_m,body,rot=(0,math.radians(-23),0))
     box('windscreen_center',(.07,.055,.59),(1.77,0,2.35),paint_m,body,rot=(0,math.radians(-23),0),lods=(0,1,2))
     bed_front=.03; bed_back=-L/2+.13; bed_z=1.28; rail_top=2.12; cover_top=H-.045
    else:
     cab_front=L/2-.12; cab_back=cab_front-2.22; cab_top=H-.04
     cab_width=W-.17
     if CAB_STYLE=='hemtt':
      prism('hemtt_cab',[(cab_back,1.22),(cab_front-.03,1.22),(cab_front,1.82),(cab_front-.50,cab_top-.03),(cab_back+.08,cab_top)],cab_width,mat=paint_m,parent=body,bevel=.045)
      wind_x=cab_front-.23; wind_z=2.36; wind_h=.70; wind_pitch=-26
      box('cab_nose',(.06,cab_width-.08,.42),(cab_front,0,1.58),paint_m,body,rot=(0,math.radians(-10),0))
     else:
      prism('hx_cab',[(cab_back,1.18),(cab_front-.05,1.18),(cab_front,2.08),(cab_front-.28,cab_top-.02),(cab_back+.10,cab_top)],cab_width,mat=paint_m,parent=body,bevel=.035)
      wind_x=cab_front-.13; wind_z=2.57; wind_h=.78; wind_pitch=-14
      box('hx_upper_brow',(.25,cab_width-.08,.18),(cab_front-.37,0,cab_top-.13),paint_m,body,bevel=.03)
      box('hx_roof_vent',(.05,1.35,.15),(cab_front-.34,0,cab_top-.20),black_m,body,lods=(0,1))
     for side in (-1,1):
      box(f'windscreen_{side}',(.045,cab_width*.43,wind_h),(wind_x,side*cab_width*.23,wind_z),glass_m,body,rot=(0,math.radians(wind_pitch),0))
     box('windscreen_divider',(.075,.055,wind_h+.02),(wind_x+.03,0,wind_z),paint_m,body,rot=(0,math.radians(wind_pitch),0),lods=(0,1,2))
     box('front_bumper',(.22,W-.03,.27),(cab_front-.02,0,.92),chassis_m,body,bevel=.025)
     box('grille_recess',(.045,1.10,.39),(cab_front+.027,0,1.72),black_m,body)
     for j in range(7):box(f'grille_bar_{j}',(.04,1.06,.025),(cab_front+.057,0,1.55+j*.055),paint_m,body,lods=(0,1))
     for side in (-1,1):
      cyl(f'headlamp_{side}',.108,.05,(cab_front+.07,side*(W*.35),1.55),'X',lamp_m,body,seg=20)
      box(f'front_fender_{side}',(1.75,.40,.12),(AXLES[0][1],side*(WHEEL_Y-.04),1.48),paint_m,body,bevel=.025)
     # Equipment immediately aft of cab, not an invented raised service mast.
     box('equipment_pack',(.58,W-.40,1.20),(cab_back-.40,0,1.84),chassis_m,body,bevel=.035)
     cyl('spare_tyre',R,.40,(cab_back-.46,0,2.20),'X',rubber_m,body,seg=28)
     cyl('spare_hub',R*.48,.44,(cab_back-.44,0,2.20),'X',paint_m,body,seg=18,lods=(0,1,2))
     bed_front=cab_back-.90; bed_back=-L/2+.16; bed_z=1.40; rail_top=2.03
     cover_top=H-.12

    # Doors/windows/steps retain family-specific cab shapes at all useful tiers.
    for side in (-1,1):
     sn='L' if side>0 else 'R'; y=side*(cab_width/2+.012)
     door_x=(cab_back+cab_front)/2
     box(f'door_panel_{sn}',(1.29,.025,.81),(door_x,y,1.72),paint_m,body,bevel=.025,lods=(0,1,2))
     box(f'side_window_{sn}',(1.08,.028,.54),(door_x+.02,y+side*.015,2.34 if CAB_STYLE=='bonnet' else 2.51),glass_m,body,bevel=.025)
     box(f'door_handle_{sn}',(.16,.035,.035),(door_x-.41,y+side*.025,2.00),steel_m,body,lods=(0,1))
     box(f'cab_step_{sn}',(.70,.16,.07),(door_x,side*(W/2-.045),1.08),steel_m,body,lods=(0,1,2))
     box(f'lower_step_{sn}',(.57,.15,.065),(door_x,side*(W/2-.04),.86),steel_m,body,lods=(0,1))
     box(f'mirror_arm_{sn}',(.075,.19,.06),(cab_front-.52,side*(W/2-.045),2.59),chassis_m,body,lods=(0,1))
     box(f'mirror_{sn}',(.07,.10,.28),(cab_front-.52,side*(W/2+.024),2.57),black_m,body,bevel=.02,lods=(0,1,2))
     box(f'wiper_{sn}',(.025,.55,.02),((1.85 if CAB_STYLE=='bonnet' else wind_x+.065),side*.42,2.23 if CAB_STYLE=='bonnet' else wind_z-.19),chassis_m,body,rot=(0,0,side*.25),lods=(0,1))

    BED_LEN=bed_front-bed_back; BX=(bed_front+bed_back)/2; BW=W-.17
    box('cargo_floor',(BED_LEN,BW,.16),(BX,0,bed_z),paint_m,body,bevel=.01)
    for side in (-1,1):
     y=side*(BW/2-.025);sn='L' if side>0 else 'R'
     box(f'cargo_sideboard_{sn}',(BED_LEN,.07,rail_top-bed_z),(BX,y,(bed_z+rail_top)/2),paint_m,body)
     for j in range(7):
      x=bed_back+(j+.5)*BED_LEN/7
      box(f'cargo_post_{sn}_{j}',(.07,.09,rail_top-bed_z+.05),(x,y+side*.045,(bed_z+rail_top)/2),chassis_m,body,lods=(0,1,2))
     for j in range(3):
      box(f'sideboard_rib_{sn}_{j}',(BED_LEN,.025,.024),(BX,y+side*.05,bed_z+.14+j*.16),chassis_m,body,lods=(0,1))
     # Fuel tank within physical width; long gaps preserve the visible rear bogie.
     cyl(f'fuel_tank_{sn}',.27,1.15,(.10,side*.76,1.03),'X',paint_m,body,seg=20,lods=(0,1,2))
     for j in (-1,1):cyl(f'fuel_band_{sn}_{j}',.282,.065,(.10+j*.39,side*.76,1.03),'X',steel_m,body,seg=20,lods=(0,1))
    box('tailgate',(.09,BW,rail_top-bed_z),(bed_back+.025,0,(bed_z+rail_top)/2),paint_m,body)
    box('rear_bumper',(.13,W-.2,.16),(-L/2+.045,0,.76),chassis_m,body)
    for side in (-1,1):box(f'tail_lamp_{side}',(.05,.21,.09),(-L/2+.01,side*.85,1.06),red_m,body,lods=(0,1,2))
    if FAMILY=='hemtt':
     # Open load is visible; folded rear crane is genuine M977 equipment.
     for i in range(5):
      for j in (-1,1):
       x=bed_back+.72+i*(BED_LEN-1.2)/5;y=j*.56
       box(f'supply_crate_{i}_{j}',(.90,.85,.55),(x,y,bed_z+.37),wood_m,body,bevel=.012)
       for sx in (-.28,.28):box(f'crate_strap_{i}_{j}_{sx}',(.065,.87,.58),(x+sx,y,bed_z+.37),chassis_m,body,lods=(0,1))
     cyl('crane_base',.22,.45,(bed_back+.35,-.72,1.92),'Z',chassis_m,body,seg=20)
     box('crane_column',(.25,.27,.77),(bed_back+.35,-.72,2.23),paint_m,body,bevel=.025)
     box('folded_crane_boom',(1.76,.23,.26),(bed_back+1.01,-.72,2.59),paint_m,body,bevel=.025)
     box('folded_crane_tip',(1.18,.17,.18),(bed_back+.78,-.72,2.83),paint_m,body,bevel=.02)
     cyl('crane_ram',.045,.59,(bed_back+.42,-.86,2.42),'Z',steel_m,body,seg=10,lods=(0,1))
     # Rear crane stays within the envelope and packed; it is not an ability animation.
    else:
     # Canvas has an actual rounded roof profile, rather than an ISO shelter box.
     def canvas_builder(bm,lod):
      cross=[(-BW*.47,rail_top-.02),(-BW*.46,cover_top-.34),(-BW*.30,cover_top-.07),(0,cover_top),(BW*.30,cover_top-.07),(BW*.46,cover_top-.34),(BW*.47,rail_top-.02)]
      xs=[bed_back+.08,bed_front-.08]
      a=[bm.verts.new((xs[0],y,z)) for y,z in cross];b=[bm.verts.new((xs[1],y,z)) for y,z in cross]
      bm.faces.new(a[::-1]);bm.faces.new(b)
      for i in range(len(cross)-1):bm.faces.new((a[i],a[i+1],b[i+1],b[i]))
      bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
     mesh_part('canvas_cover',canvas_builder,canvas_m,body)
     for j in range(6):
      x=bed_back+.2+j*(BED_LEN-.4)/5
      for side in (-1,1):box(f'canvas_tie_{j}_{side}',(.035,.028,cover_top-rail_top-.23),(x,side*(BW*.47+.017),(cover_top+rail_top)/2-.12),chassis_m,body,lods=(0,1))
     for j in range(4):box(f'canvas_rear_tie_{j}',(.028,.032,.65),(bed_back+.051,-.70+j*.46,rail_top+.25),chassis_m,body,lods=(0,1))
     # Small uncovered top supplies at the rear make logistics purpose clear.

    rest_on_ground()
    finish(ao_distance=.8,ao_rays=8)
    print('ROSTER_TRUCK',json.dumps({'family':FAMILY,'type':TYPE_ID,'dimensions':[L,W,H],'axles':len(AXLES),'tris':triangles_by_tier(),'nodes':sorted(o.name for o in bpy.data.objects if o.type=='EMPTY')}))
    export(OUT,texture_px=256)
    if PREVIEW:
     # Review the exported material/vertex-color path, rather than an unconnected source shader.
     bpy.ops.wm.read_factory_settings(use_empty=True)
     bpy.ops.import_scene.gltf(filepath=os.path.abspath(OUT))
     os.makedirs(PREVIEW,exist_ok=True)
     for o in bpy.data.objects:
      if o.type=='MESH':o.hide_render=('_LOD' in o.name and not o.name.endswith('_LOD0'))
     scene=bpy.context.scene;scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=960;scene.render.resolution_y=640;scene.render.resolution_percentage=100
     scene.world=bpy.data.worlds.new('review_world');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs['Color'].default_value=(.32,.32,.32,1);scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.7
     ground_mat=bpy.data.materials.new('review_ground');ground_mat.diffuse_color=(.18,.20,.17,1)
     bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.012));bpy.context.object.data.materials.append(ground_mat)
     for loc,power,size in [((3,-6,12),2200,9),((-5,5,7),1200,8)]:
      bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.shape='DISK';light.data.size=size;light.rotation_euler=(Vector((0,0,1.1))-light.location).to_track_quat('-Z','Y').to_euler()
     bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.ortho_scale=L*1.26
     for label,loc in [('front',(L*1.5,-L*1.8,L*.85)),('side',(0,-L*2,L*.55)),('rear',(-L*1.6,L*1.7,L*.8))]:
      cam.location=loc;cam.rotation_euler=(Vector((0,0,H*.45))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=os.path.join(PREVIEW,label+'.png');bpy.ops.render.render(write_still=True)
