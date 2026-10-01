use contract::map::MapDefinition;
use contract::scenario::Rules;
use sim::world::WorldGeometry;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct Counter;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(n: usize) {
    let live = CURRENT.fetch_add(n, Ordering::Relaxed) + n;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if CURRENT.load(Ordering::Relaxed).saturating_add(l.size()) > 4 * 1024 * 1024 * 1024 {
            std::process::exit(70);
        }
        let p = System.alloc(l);
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if CURRENT.load(Ordering::Relaxed).saturating_add(l.size()) > 4 * 1024 * 1024 * 1024 {
            std::process::exit(70);
        }
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        CURRENT.fetch_sub(l.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if CURRENT.load(Ordering::Relaxed).saturating_add(n) > 4 * 1024 * 1024 * 1024 {
            std::process::exit(70);
        }
        let q = System.realloc(p, l, n);
        if !q.is_null() {
            if n >= l.size() {
                add(n - l.size());
            } else {
                CURRENT.fetch_sub(l.size() - n, Ordering::Relaxed);
            }
        }
        q
    }
}
#[global_allocator]
static ALLOC: Counter = Counter;
fn stage<T>(name: &str, work: impl FnOnce() -> T) -> T {
    let before = CURRENT.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let start = Instant::now();
    let value = work();
    let current = CURRENT.load(Ordering::Relaxed);
    let peak = PEAK.load(Ordering::Relaxed);
    println!(
        "{}",
        serde_json::json!({"stage":name,"ms":start.elapsed().as_secs_f64()*1000.0,"before_bytes":before,"live_bytes":current,"stage_peak_bytes":peak})
    );
    value
}
fn main() {
    use contract::command::RoutePolicy;
    use contract::map::MoverClass;
    use contract::scenario::PushClass;
    use sim::math::v2;
    use sim::navigation::{Leg, Mobility, NavGrid, Plan, RoadNet};
    let args: Vec<_> = std::env::args().collect();
    let side: f64 = args[1].parse().unwrap();
    let arm = args.get(2).map(String::as_str).unwrap_or("empty");
    let rules: Rules = serde_json::from_value(sim::fixtures::village()).unwrap();
    let mut input = serde_json::json!({"size":[side,side],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35});
    if arm == "sparse" {
        input["props"] = serde_json::json!([
 {"kind":"crate","center":[20.0,side*0.15],"yaw":0.4,"half_extents":[3,3,2]},
 {"kind":"crate","center":[side-20.0,side*0.85],"yaw":-0.4,"half_extents":[3,3,2]}]);
    }
    if arm == "bridge" || arm == "disconnected" {
        input["water"] =
            serde_json::json!([{ "rect":[side*0.5-10.0,0,20,side],"bed_z":-2,"surface_z":-0.5}]);
    }
    if arm == "bridge" {
        input["bridges"] = serde_json::json!([{ "deck":"bridge_deck","center":[side*0.5,side*0.2],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]);
    }
    let map: MapDefinition = serde_json::from_value(input).unwrap();
    println!(
        "{}",
        serde_json::json!({"side_m":side,"arm":arm,"heap_ceiling_bytes":4u64*1024*1024*1024,"resource_exit":70})
    );
    let w = stage("world", || WorldGeometry::new(&map, &rules));
    let a = stage("blue_grid", || NavGrid::build(&w, w.props().cloned(), 0.3));
    let b = stage("red_grid", || NavGrid::build(&w, w.props().cloned(), 0.3));
    let roads = stage("roads", || RoadNet::build(&w));
    let from = v2(side * 0.05, side * 0.35);
    let to = v2(side * 0.95, side * 0.65);
    for class in [MoverClass::Infantry, MoverClass::Vehicle] {
        for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
            let m = Mobility {
                off_road_mps: 6.0,
                road_mps: 12.0,
                forest_multiplier: 0.4,
                half_width_m: if class == MoverClass::Infantry {
                    0.5
                } else {
                    1.8
                },
                class,
                push: PushClass::Heavy,
                drive: None,
            };
            for (name, g, from, to) in [("blue", &a, from, to), ("red", &b, to, from)] {
                let leg = Leg {
                    from,
                    goal: to,
                    m: &m,
                    policy,
                    avoid: &[],
                };
                let (p, work) = stage("route", || {
                    sim::navigation::plan(g, &roads, leg, &rules.navigation)
                });
                if arm == "disconnected" {
                    assert!(matches!(p, Plan::Blocked(_)));
                } else {
                    let Plan::Route(ref r) = p else {
                        panic!("unexpected blocked: {p:?}")
                    };
                    assert!(g.route_fits(from, r, &m));
                    assert_eq!(r.last(), Some(&to));
                }
                println!(
                    "{}",
                    serde_json::json!({"side":name,"mover":format!("{class:?}"),"policy":format!("{policy:?}"),"storage":g.storage(),"search":work,"route":format!("{p:?}")})
                );
            }
        }
    }
}
