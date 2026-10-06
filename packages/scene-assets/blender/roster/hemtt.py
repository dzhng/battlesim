"""M977 HEMTT cargo truck: eight wheels, angular forward cab, open supplies, folded rear crane.

Rebuild through the pinned asset CLI; optional --preview=<scratch-dir> writes
candidate views after export. References and frozen fit frame are in the family
manifest. Stationary service is simulation state, not fictional deployment art.
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from logistics import build

FAMILY = 'hemtt'
TYPE_ID = 'us_m977_hemtt_general_resupply'
L, W, H = (10.4, 2.4, 3.0)
AXLES = [('F', 3.58), ('F2', 1.93), ('M', -2.45), ('R', -3.93)]
CAB_STYLE = 'hemtt'

build(FAMILY, TYPE_ID, (L, W, H), AXLES, CAB_STYLE)
