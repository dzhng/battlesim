use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use contract::map::MapDefinition;
use contract::scenario::{Rules, ScenarioDefinition};
use contract::ids::Side;
use sim::battle::Battle;
use sim::publication::Publisher;
use sim::world::WorldGeometry;

struct Counter;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(n: usize) { let live = CURRENT.fetch_add(n, Ordering::Relaxed) + n; PEAK.fetch_max(live, Ordering::Relaxed); }
unsafe impl GlobalAlloc for Counter {
 unsafe fn alloc(&self, l: Layout) -> *mut u8 { let p = System.alloc(l); if !p.is_null() { add(l.size()); } p }
 unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 { let p = System.alloc_zeroed(l); if !p.is_null() { add(l.size()); } p }
 unsafe fn dealloc(&self, p: *mut u8, l: Layout) { System.dealloc(p,l); CURRENT.fetch_sub(l.size(),Ordering::Relaxed); }
 unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { let q=System.realloc(p,l,n); if !q.is_null() { if n>=l.size() { add(n-l.size()); } else { CURRENT.fetch_sub(l.size()-n,Ordering::Relaxed); } } q }
}
#[global_allocator] static ALLOC: Counter = Counter;
fn stage<T>(name: &str, work: impl FnOnce()->T) -> T {
 let before=CURRENT.load(Ordering::Relaxed); PEAK.store(before,Ordering::Relaxed); let start=Instant::now();
 let value=work(); let current=CURRENT.load(Ordering::Relaxed); let peak=PEAK.load(Ordering::Relaxed);
 println!("{}",serde_json::json!({"stage":name,"ms":start.elapsed().as_secs_f64()*1000.0,"before_bytes":before,"live_bytes":current,"stage_peak_bytes":peak})); value
}
fn main() {
 let args:Vec<_>=std::env::args().collect(); let side:f64=args.get(1).unwrap().parse().unwrap();
 let mode=args.get(2).map(String::as_str).unwrap_or("world");
 let fixture=sim::fixtures::village(); let rules:Rules=serde_json::from_value(fixture.clone()).unwrap();
 let map:MapDefinition=serde_json::from_value(serde_json::json!({"size":[side,side],"height_grid_m":4,"slope_cutoff_deg":50})).unwrap();
 println!("{}",serde_json::json!({"side_m":side,"mode":mode,"pointer_bytes":std::mem::size_of::<usize>(),"v3_bytes":std::mem::size_of::<sim::math::V3>(),"fog_cell_m":rules.sensors.fog_cell_m,"ground_cell_m":rules.ground.cell_m}));
 if mode=="world" {
  let world=stage("world",||WorldGeometry::new(&map,&rules));
  let positions=stage("terrain_positions",||world.export_terrain_positions());
  let indices=stage("terrain_indices",||world.export_terrain_indices());
  let surfaces=stage("terrain_surfaces",||world.export_terrain_triangle_surfaces());
  let foliage=stage("foliage",||world.export_foliage());
  println!("{}",serde_json::json!({"positions_bytes":positions.len()*4,"indices_bytes":indices.len()*4,"surface_bytes":surfaces.len(),"foliage_bytes":foliage.len()*4,"sw_height":world.surface_at(8.0,8.0).unwrap().z,"ne_height":world.surface_at(side-8.0,side-8.0).unwrap().z}));
 } else {
  let setup:ScenarioDefinition=serde_json::from_value(serde_json::json!({"map":map,"rules":fixture,"units":[],"events":[],"scripts":[]})).unwrap();
  let mut battle=stage("battle",||Battle::new(&setup,1)); let mut publisher=Publisher::new();
  let first=stage("initial_publication",||publisher.publish(&battle,Side::Blue).len());
  stage("tick",||battle.step());
  let steady=stage("steady_publication",||publisher.publish(&battle,Side::Blue).len());
  let red=stage("side_switch_publication",||publisher.publish(&battle,Side::Red).len());
  println!("{}",serde_json::json!({"initial_bytes":first*4,"steady_bytes":steady*4,"red_bytes":red*4,"digest":format!("{:016x}",battle.digest()),"sw_height":battle.world().surface_at(8.0,8.0).unwrap().z,"ne_height":battle.world().surface_at(side-8.0,side-8.0).unwrap().z}));
 }
}
