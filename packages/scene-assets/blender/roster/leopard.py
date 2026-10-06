"""Leopard2 A6/A7V/A8: long L55, wedge cheeks, seven wheels and explicit kit/Trophy.
Run in Blender with --preview=DIR; four genuine scene-assets tiers.
"""
import sys,math
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from armor import *


def build(variant,frame,hull,turret,m):
    L,W,H=frame['body_dimensions_m']
    prism('leopard_upper_hull',[(-3.82,.91),(3.52,.82),(3.84,1.18),(2.45,1.66),(-3.75,1.70)],W-.09,mat=m['armor'],parent=hull,bevel=.03)
    prism('leopard_lower_hull',[(-3.61,.38),(3.10,.38),(3.65,.94),(-3.78,.98)],2.15,mat=m['dark'],parent=hull,bevel=.03)
    for side in (-1,1):
        for k in range(7):
            x=-3.43+k*.99
            box(f'leopard_skirt_{side}_{k}',(.95,.065,.48),(x+.44,side*(W/2-.035),1.08),m['armor'],hull,bevel=.012)
            if k<4:box(f'armored_front_skirt_{side}_{k}',(.94,.10,.10),(x+.44,side*(W/2-.025),1.37),m['armor'],hull,lods=(0,1,2))
        box(f'front_light_guard_{side}',(.12,.37,.25),(3.38,side*1.25,1.35),m['armor'],hull,bevel=.015)
        box(f'headlamp_{side}',(.028,.25,.15),(3.45,side*1.25,1.37),m['lamp'],hull,lods=(0,1,2))
        box(f'skirt_handle_{side}',(.12,.04,.16),(1.0,side*(W/2+.005),.88),m['dark'],hull,lods=(0,1))
    # Main turret box plus pointed add-on cheeks: A5+ characteristic wedge profile.
    bottom=[(-2.39,-1.50),(1.60,-1.35),(1.98,-.52),(1.98,.52),(1.60,1.35),(-2.39,1.50)]
    top=[(-2.26,-1.44),(1.42,-1.23),(1.69,-.47),(1.69,.47),(1.42,1.23),(-2.26,1.44)]
    loft('leopard_turret',[(.02,bottom),(.80,top)],mat=m['armor'],parent=turret,bevel=.025)
    for side in (-1,1):
        # Convex, sharply pointed frontal armor wedge, physically separate from core.
        lower=[(.26,side*.46),(2.27,side*.62),(1.30,side*1.69),(-.35,side*1.49)]
        upper=[(.26,side*.51),(1.65,side*.62),(1.15,side*1.50),(-.35,side*1.38)]
        if side<0:lower=lower[::-1];upper=upper[::-1]
        loft(f'wedge_cheek_{side}',[(.10,lower),(.63,upper)],mat=m['armor'],parent=turret,bevel=.015)
        box(f'bustle_side_bin_{side}',(1.31,.34,.49),(-1.74,side*1.40,.49),m['armor'],turret,bevel=.025)
        for j in range(8):
            cyl(f'smoke_tube_{side}_{j}',.037,.24,(-.44+(j%4)*.15,side*1.52,.68+(j//4)*.13),'X',m['dark'],turret,seg=12,rot=(0,-.30,side*.57),lods=(0,1,2))
    hatches(turret,.81,m)
    # PERI commander optic and forward EMES sight distinguish the turret equipment.
    cyl('peri_mount',.17,.21,(-.26,.71,.97),'Z',m['armor'],turret,seg=20,lods=(0,1,2))
    box('peri_head',(.32,.27,.24),(-.26,.71,1.12),m['armor'],turret,bevel=.02,lods=(0,1,2))
    box('peri_lens',(.025,.19,.11),(-.085,.71,1.13),m['glass'],turret,lods=(0,1))
    box('emes_sight',(.35,.40,.27),(1.29,-.79,.82),m['armor'],turret,bevel=.02,lods=(0,1,2))
    box('emes_lens',(.025,.26,.16),(1.48,-.79,.83),m['glass'],turret,lods=(0,1))
    cyl('driver_hatch',.29,.04,(2.20,-.59,1.65),'Z',m['armor'],hull,seg=24,lods=(0,1,2))
    # Two large circular rear cooling fans, unlike Abrams turbine slats.
    for side in (-1,1):
        cyl(f'cooling_grille_{side}',.62,.035,(-2.85,side*.71,1.716),'Z',m['black'],hull,seg=28,lods=(0,1,2))
        for k in range(10):
            angle=k*math.pi/10
            box(f'cooling_spoke_{side}_{k}',(1.17,.024,.02),(-2.85,side*.71,1.742),m['dark'],hull,rot=(0,0,angle),lods=(0,1))
        box(f'rear_grille_{side}',(.035,.77,.43),(-3.855,side*.91,1.11),m['black'],hull,lods=(0,1,2))
        for k in range(8):box(f'rear_grille_bar_{side}_{k}',(.025,.74,.028),(-3.875,side*.91,.925+k*.052),m['dark'],hull,lods=(0,1))
    if variant['variant'] in ('2A7V','2A8'):
        prism('a7v_glacis_addon',[(2.40,1.67),(3.61,1.24),(3.70,1.37),(2.47,1.85)],2.57,mat=m['armor'],parent=hull,bevel=.02,lods=(0,1,2))
        box('a7v_bustle_cooling',(1.03,1.36,.31),(-2.18,0,.90),m['armor'],turret,bevel=.02,lods=(0,1,2))
        for side in (-1,1):
            box(f'a7v_extra_side_module_{side}',(.99,.17,.47),(.12,side*1.63,.54),m['armor'],turret,bevel=.02,lods=(0,1,2))
            box(f'a7v_rear_cooling_vent_{side}',(.04,.46,.20),(-2.715,side*.43,.90),m['black'],turret,lods=(0,1))
    if variant['variant']=='2A8':trophy(turret,.65,W-.09,m)


export_family('leopard',build)
