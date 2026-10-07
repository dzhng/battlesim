"""M977 HEMTT cargo truck: eight wheels, angular forward cab, open supplies, folded rear crane.

Rebuild through the pinned asset CLI; optional --preview=<scratch-dir> writes
candidate views after export. Its frame is the unit type's in the resolved
catalog. Stationary service is simulation state, not fictional deployment art.
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from logistics import build

FAMILY = 'hemtt'
AXLES = [('F', 3.58), ('F2', 1.93), ('M', -2.45), ('R', -3.93)]
CAB_STYLE = 'hemtt'

build(FAMILY, AXLES, CAB_STYLE)
