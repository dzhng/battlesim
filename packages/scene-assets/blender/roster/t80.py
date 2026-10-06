"""T-80BVM: compact cast turret, broad continuous Relikt cheek kit, turbine deck."""
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from eastern_tanks import *


def build(variant,frame,hull,turret,m):
    chassis(frame,hull,m,turbine=True);round_turret(turret,m,1.54,1.36);roof(turret,m);slats(hull,frame,m)
    # BVM broad polygonal Relikt cheeks differ from the spaced Kontakt-5 fan.
    for side in (-1,1):
        lo=[(.42,side*.40),(1.67,side*.46),(1.09,side*1.49),(-.32,side*1.49)]
        hi=[(.32,side*.43),(1.46,side*.48),(.98,side*1.39),(-.32,side*1.36)]
        if side<0:lo=lo[::-1];hi=hi[::-1]
        loft(f'bvm_relikt_cheek_{side}',[(.12,lo),(.45,hi)],mat=m['armor'],parent=turret,bevel=.015)
        for k in range(3):box(f'bvm_cheek_seam_{side}_{k}',(.027,.45,.027),(.15+k*.36,side*1.14,.47),m['dark'],turret,lods=(0,1))
    for side in (-1,1):
        for k in range(3):box(f'bvm_side_era_{side}_{k}',(.39,.19,.36),(-.71-k*.35,side*1.31,.28),m['armor'],turret,bevel=.015,lods=(0,1,2))
    box('bvm_rear_stowage',(.34,1.25,.29),(-1.61,0,.24),m['armor'],turret,bevel=.02,lods=(0,1,2))


export_family('t80',build)
