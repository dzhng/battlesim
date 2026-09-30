import json, subprocess, os
from pathlib import Path
out=Path('throwaway/city-spike/native');out.mkdir(exist_ok=True)
probe=str(Path(os.environ['CARGO_TARGET_DIR'])/'release/examples/city_allocation_probe')
for side,mode in [(1600,'world'),(1600,'battle'),(12000,'world'),(15000,'world'),(18000,'world'),(12000,'battle'),(15000,'battle'),(18000,'battle')]:
 # Admission: existing constructor/exports without navigation; reject later route arms independently.
 h=(side//4+1)**2; f=(side//8)**2; c=(side//4)**2; buckets=((side+31)//32)**2
 world=8*h+32*f+side*side//8+24*buckets
 projected=world+12*h+24*c+24*h+24*c+4*c
 assert projected < 4*1024**3, ('ceiling',side,projected)
 with (out/f'{side}-{mode}.jsonl').open('w') as stdout, (out/f'{side}-{mode}.time.txt').open('w') as stderr:
  subprocess.run(['/usr/bin/time','-l',probe,str(side),mode],stdout=stdout,stderr=stderr,check=True,timeout=120)
 rows=[json.loads(s) for s in (out/f'{side}-{mode}.jsonl').read_text().splitlines()]
 stages=[r for r in rows if 'stage' in r]
 print(json.dumps({'side_m':side,'mode':mode,'largest_stage_heap_bytes':max(r['stage_peak_bytes'] for r in stages),'stages_ms':{r['stage']:round(r['ms'],2) for r in stages},'out':rows[-1]}),flush=True)
