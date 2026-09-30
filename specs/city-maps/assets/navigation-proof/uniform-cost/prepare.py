from pathlib import Path
import hashlib
r=Path(__file__).resolve().parents[2];p=Path(__file__).parent
s=(r/'crates/sim/src/navigation.rs').read_text()
assert hashlib.sha256(s.encode()).hexdigest()=='8b0c1d4e18843426a09cafb52205227e5a214ae8b4bcb54a6984749f04f9d2d9'
def replace(a,b,count=1):
 global s
 assert s.count(a)==count,(a,s.count(a));s=s.replace(a,b)
replace('fn index(&self, at: usize) -> &Cell {','fn index(&self, at: usize) -> &Cell {\n        attr_count(0);')
replace('if let Some(cell) = self.changed.get(&at) {','if let Some(cell) = self.changed.get(&at) {\n            attr_count(1);')
replace('fn get(&self, k: &usize) -> Option<Search> {','fn get(&self, k: &usize) -> Option<Search> {\n        attr_count(2);')
replace('let tile = self.tiles.get(&key)?;','let tile = self.tiles.get(&key)?;\n        attr_count(3);')
replace('fn relax(&mut self, k: usize, value: Search, admit: impl FnOnce() -> bool) -> bool {','fn relax(&mut self, k: usize, value: Search, admit: impl FnOnce() -> bool) -> bool {\n        attr_count(4);')
replace('if !admit() {','let admitted = admit();\n                attr_count(if admitted { 5 } else { 6 });\n                if !admitted {',2)
replace('o.f.total_cmp(&self.f).then_with(|| o.cell.cmp(&self.cell))','let order = o.f.total_cmp(&self.f);\n        attr_compare(order == Ordering::Equal);\n        order.then_with(|| o.cell.cmp(&self.cell))')
replace('fn rejects(&self, cell: usize, g: f64) -> bool {','fn rejects(&self, cell: usize, g: f64) -> bool {\n        attr_count(10);')
replace('fn distance(&self, a: usize, b: usize, nx: usize) -> f64 {','fn distance(&self, a: usize, b: usize, nx: usize) -> f64 {\n        attr_count(11);')
replace('fn cell_cost(c: &Cell, m: &Mobility, policy: RoutePolicy, length: f64) -> f64 {','fn cell_cost(c: &Cell, m: &Mobility, policy: RoutePolicy, length: f64) -> f64 {\n        attr_count(12);')
replace('fn clearance_at(&self, push: PushClass, at: usize) -> f64 {','fn clearance_at(&self, push: PushClass, at: usize) -> f64 {\n        attr_count(13);')
replace('fn fits(&self, cell: usize, m: &Mobility) -> bool {','fn fits(&self, cell: usize, m: &Mobility) -> bool {\n        attr_count(14);')
replace('fn segment_cost(&self, a: V2, b: V2, m: &Mobility, policy: RoutePolicy) -> Option<f64> {','fn segment_cost(&self, a: V2, b: V2, m: &Mobility, policy: RoutePolicy) -> Option<f64> {\n        attr_count(15);')
replace('let h = |k: usize, nx: usize| {','let h = |k: usize, nx: usize| {\n            attr_count(16);')
replace('let uniform = self.uniform_stencil(cell, m);','let uniform = self.uniform_stencil(cell, m);\n            attr_count(if uniform { 17 } else { 18 });')
replace('self.regions.iter().any(|&[x, y, w, h]| {','self.regions.iter().any(|&[x, y, w, h]| {\n            attr_count(19);',2)
replace('let mut open = BinaryHeap::new();','attribution_phase(2);\n        let mut open = BinaryHeap::new();')
replace('open.push(Open {','attribution_queue(0);\n        open.push(Open {',2)
replace('while let Some(Open { f, cell }) = open.pop() {','while let Some(Open { f, cell }) = { attribution_queue(1); open.pop() } {')
replace('let mut cells = vec![target];','attribution_phase(3);\n        let mut cells = vec![target];')
s+=r'''
#[derive(Default)]
struct Attribution {phase:usize,queue:usize,counts:[[u64;20];5]}
thread_local! {static ATTR:std::cell::RefCell<Attribution>=Default::default();}
fn attr_count(k:usize){ATTR.with(|a|{let mut a=a.borrow_mut();let p=a.phase;a.counts[p][k]+=1;});}
fn attr_compare(equal:bool){ATTR.with(|a|{let mut a=a.borrow_mut();let p=a.phase;
    let k=if a.queue==0 {7}else{8};a.counts[p][k]+=1;if equal{a.counts[p][9]+=1;}});}
pub fn attribution_reset(phase:usize){ATTR.with(|a|*a.borrow_mut()=Attribution {phase,..Default::default()});}
pub fn attribution_phase(phase:usize){ATTR.with(|a|a.borrow_mut().phase=phase);}
fn attribution_queue(queue:usize){ATTR.with(|a|a.borrow_mut().queue=queue);}
pub fn attribution()->serde_json::Value{ATTR.with(|a|{let a=a.borrow();
    let names=["cell_get","cell_hit","scratch_get","scratch_tile_hit","scratch_relax","admitted","bound_rejected","heap_push_compare","heap_pop_compare","heap_equal_f_compare","bound_calls","cut_distance_calls","cell_cost_calls","clearance_calls","fits_calls","segment_cost_calls","h_calls","uniform_stencil","nonuniform_stencil","region_tests"];
    let mut map=serde_json::Map::new();for (p,name) in ["build","prepare","search","finalize","outside"].iter().enumerate(){
        let row:serde_json::Map<String,serde_json::Value>=names.iter().enumerate().map(|(k,n)|(n.to_string(),a.counts[p][k].into())).collect();map.insert(name.to_string(),row.into());}
    map.into()})}
'''
p.joinpath('counted.rs').write_text(s)
main=(r/'crates/sim/examples/navigation_resources.rs').read_text().replace('use sim::navigation::{Mobility, NavGrid, Plan};','use counted::{Mobility, NavGrid, Plan};')
main='pub use sim::{math,world};\n#[allow(dead_code)]\nmod counted;\n'+main
main=main.replace('let p = stage("route", || g.plan(from, to, &m, policy));','counted::attribution_reset(1);\n                let p = stage("route", || g.plan(from, to, &m, policy));\n                counted::attribution_phase(4);')
main=main.replace('"storage":g.storage(),','"storage":g.storage(),"attribution":counted::attribution(),')
p.joinpath('main.rs').write_text(main)
p.joinpath('Cargo.toml').write_text(f'''[package]
name="navigation-remaining-attribution"
version="0.0.0"
edition="2021"
[workspace]
[dependencies]
sim={{path="{r}/crates/sim"}}
contract={{path="{r}/crates/contract"}}
serde_json="1"
serde={{version="1",features=["derive"]}}
[[bin]]
name="navigation-remaining-attribution"
path="main.rs"
[profile.release]
opt-level=3
lto="thin"
''')
print('Disposable source generated; production unchanged')
