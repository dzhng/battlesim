"""Named boxer exports; adopted manifest frames."""
import os,sys
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from europe_carriers import build
build("boxer")
