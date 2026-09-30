"""Generate disposable counters; this is not a production timing candidate."""
from pathlib import Path
import sys,hashlib
root=Path(sys.argv[1]).resolve()
out=root/'throwaway/navigation-attribution'
out.mkdir(parents=True,exist_ok=True)
base=root/'crates/sim/src/navigation.rs'
assert hashlib.sha256(base.read_bytes()).hexdigest()=='afb26660578c4147b1d6da9cc8c9a17c7139a1dec832ad2795f72045384ebc6f'
s=base.read_text()
def replace(a,b):
    global s
    assert s.count(a)==1,(a,s.count(a))
    s=s.replace(a,b)
replace('fn get(&self, k: &usize) -> Option<Search> {','fn get(&self, k: &usize) -> Option<Search> {\n        attr_get();')
replace('(tile.stamp[at] == self.generation).then(|| Search {','(tile.stamp[at] == self.generation).then(|| { attr_hit(); Search {')
replace('parent: tile.parent[at],\n        })','parent: tile.parent[at],\n        }})')
replace('fn insert(&mut self, k: usize, value: Search) {','fn insert(&mut self, k: usize, value: Search) {\n        attr_insert();')
replace('let h = |k: usize, nx: usize| {','let h = |k: usize, nx: usize| {\n            attr_h(k);')
s += (root/'specs/city-maps/assets/navigation-proof/uniform-parent/attribution-support.rs').read_text()
out.joinpath('counted.rs').write_text(s)
resources=root/'crates/sim/examples/navigation_resources.rs'
assert hashlib.sha256(resources.read_bytes()).hexdigest()=='1f4851109cd067457e852c02974ea0dacf9cb34c4a333cfe18e38c1fd14b6077'
main=resources.read_text()
main='pub use sim::{math,world};\n#[allow(dead_code)]\nmod counted;\n'+main.replace('use sim::navigation::{Mobility, NavGrid, Plan};','use counted::{Mobility, NavGrid, Plan};')
main=main.replace('let p = stage("route", || g.plan(from, to, &m, policy));','counted::attribution_reset((side as usize / 2).pow(2));\n                let p = stage("route", || g.plan(from, to, &m, policy));')
main=main.replace('"storage":g.storage(),','"storage":g.storage(),"attribution":counted::attribution(),')
main=main.replace('//!','//')
out.joinpath('main.rs').write_text(main)
manifest=(root/'throwaway/navigation-uniform-proof/Cargo.toml').read_text()
manifest=manifest.replace('navigation-uniform-proof','navigation-attribution')
manifest=manifest.replace('serde_json="1"','serde_json="1"\nserde={version="1",features=["derive"]}')
out.joinpath('Cargo.toml').write_text(manifest)
print(out/'Cargo.toml')
