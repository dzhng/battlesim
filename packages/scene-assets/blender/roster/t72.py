"""T-72B3 (2016): cast dome, Kontakt-5 front blocks, Relikt side kit and slats."""
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from eastern_tanks import *


def build(variant,frame,hull,turret,m):
    chassis(frame,hull,m);round_turret(turret,m,1.65,1.40);era_cheeks(turret,m);roof(turret,m);slats(hull,frame,m)
    for side in (-1,1):
        box(f'b3_side_kit_{side}',(.55,.17,.37),(-1.10,side*1.39,.25),m['armor'],turret,bevel=.02,lods=(0,1,2))
    box('b3_rear_stowage',(.36,1.00,.31),(-1.72,0,.22),m['armor'],turret,bevel=.025,lods=(0,1,2))


export_family('t72',build)
