from pathlib import Path
import json, subprocess
root=Path('/Users/david/.codex/worktrees/city-sim-bodies')
fixture=root/'fixtures/village.json'
original=fixture.read_bytes()
binaries=Path('/Users/david/dev/battlegame/target/wt/city-sim-bodies/release/examples')
out=root/'throwaway/c77'
try:
    for arm in ['off','on']:
        data=json.loads(original)
        data['forests']['rule']['logs_per_ha']=5 if arm=='on' else 0
        data['forests']['rule']['boulders_per_ha']=3 if arm=='on' else 0
        fixture.write_text(json.dumps(data,indent=2)+'\n')
        args=[str(binaries/'village_report'),'--quick','--threads','1','--fresh','--save',str(out/f'village-{arm}.jsonl')]
        if arm=='on':args+=['--compare',str(out/'village-off.jsonl')]
        print('Starting paired quick arm',arm,flush=True)
        with (out/f'village-{arm}.txt').open('w') as log:
            subprocess.run(args,cwd=root,stdout=log,stderr=subprocess.STDOUT,check=True)
        print((out/f'village-{arm}.txt').read_text(),flush=True)
        print('Starting paired 5min endurance arm',arm,flush=True)
        with (out/f'endurance-{arm}.txt').open('w') as log:
            subprocess.run([str(binaries/'endurance_report'),'5'],cwd=root,stdout=log,stderr=subprocess.STDOUT,check=True)
        print((out/f'endurance-{arm}.txt').read_text(),flush=True)
finally:
    fixture.write_bytes(original)
