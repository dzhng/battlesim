pub use sim::{math, world};
#[path = "candidate.rs"]
#[allow(dead_code)]
mod candidate;
#[path = "core.rs"]
mod core;
#[path = "intent.rs"]
mod intent;
#[path = "topology.rs"]
mod topology;
use candidate::{Mobility, NavGrid, Refinement, RefinementStatus};
use contract::{command::RoutePolicy, map::MapDefinition, scenario::Rules};
use core::{Physical, Request, Status};
use math::v2;
use serde_json::{json, Value};
use topology::{fold, Point, Prepared, Segment};
use world::WorldGeometry;
struct GridCheck<'a> {
    grid: &'a NavGrid,
    m: Mobility,
    policy: RoutePolicy,
    id: u64,
}
impl Physical for GridCheck<'_> {
    type Job = Refinement;
    fn identity(&self) -> u64 {
        self.id
    }
    fn begin(&self, a: Point, b: Point, _: bool) -> Refinement {
        Refinement::new(v2(a.0, a.1), v2(b.0, b.1), self.m, self.policy)
    }
    fn poll(&self, j: &mut Refinement, budget: &mut usize) -> Option<Option<f64>> {
        match j.poll(self.grid, budget) {
            RefinementStatus::Pending => None,
            RefinementStatus::Complete(v) => Some(v),
        }
    }
    fn digest(&self, j: &Refinement) -> u64 {
        j.digest()
    }
    fn reference_digest(&self, j: &Refinement) -> u64 {
        j.reference_digest()
    }
}
fn input_hash(s: &str) -> u64 {
    s.bytes().fold(0, |h, b| fold(h, b as u64))
}
fn mobility(rules: &Rules, kind: &str) -> Mobility {
    let m = sim::units::mobility(rules.catalog.by_id(kind), rules);
    Mobility {
        off_road_mps: m.off_road_mps,
        road_mps: m.road_mps,
        forest_multiplier: m.forest_multiplier,
        half_width_m: m.half_width_m,
        class: m.class,
        push: m.push,
        drive: None,
    }
}
fn record<P: Physical>(job: &Request<P>, trace: Vec<[u64; 3]>) -> Value {
    json!({"status":format!("{:?}",job.status()),"points":job.route().map(|p|[p.0.to_bits(),p.1.to_bits()]).collect::<Vec<_>>(),"actual_cost_bits":job.actual_cost.to_bits(),"coarse_cost_bits":job.coarse_cost.to_bits(),"work":job.work,"searches":job.searches,"physical_checks":job.physical_checks,"retained":job.retained(),"digest":job.digest(),"poll_trace":trace})
}
fn finish<P: Physical>(job: &mut Request<P>, quantum: usize) -> Vec<[u64; 3]> {
    let mut trace = Vec::new();
    let mut one = 1;
    assert_eq!(job.poll(&mut one), Status::Pending);
    assert_eq!(one, 0);
    assert_eq!(job.digest(), job.reference_digest());
    trace.push([job.work as u64, job.digest(), 0]);
    for _ in 0..100000 {
        let mut budget = quantum;
        let before = job.work;
        let status = job.poll(&mut budget);
        assert_eq!(job.work - before, quantum - budget);
        assert!(job.work - before <= quantum);
        assert_eq!(
            job.digest(),
            job.reference_digest(),
            "incremental core+physical state digest"
        );
        trace.push([
            job.work as u64,
            job.digest(),
            match status {
                Status::Pending => 0,
                Status::Ready => 1,
                Status::CorridorDeclined => 2,
            },
        ]);
        if status != Status::Pending {
            return trace;
        }
    }
    panic!("finite immutable road request must progress")
}
fn check_cost(job: &Request<GridCheck<'_>>) {
    if job.status() == Status::Ready {
        let points: Vec<_> = job.route().collect();
        let mut total = 0.0;
        for pair in points.windows(2) {
            total += job
                .physical_context()
                .grid
                .reference_segment_cost(
                    v2(pair[0].0, pair[0].1),
                    v2(pair[1].0, pair[1].1),
                    &job.physical_context().m,
                    job.physical_context().policy,
                )
                .expect("returned route's every segment fits original owner");
        }
        assert_eq!(
            total.to_bits(),
            job.actual_cost.to_bits(),
            "complete original segment-cost sequence"
        );
    }
}
pub fn compute(raw_rules: Value) -> Vec<Value> {
    let rules: Rules = serde_json::from_value(raw_rules.clone()).unwrap();
    let mut records = Vec::new();
    let rules_identity = input_hash(&raw_rules.to_string());
    let boundary:Vec<_>=[1999.999,2000.0,2000.001].into_iter().map(|distance|{
        let active=intent::QueuedMove{goal:Point(distance,0.0)}.activate(Point(0.0,0.0));
        json!({"distance_bits":distance.to_bits(),"automatic_roads":active.automatic_roads(),"long_replan":active.replan_from(Point(-3000.0,0.0)).automatic_roads,"short_replan":active.replan_from(Point(distance-1.0,0.0)).automatic_roads})
    }).collect();
    assert_eq!(
        boundary
            .iter()
            .map(|v| v["automatic_roads"].as_bool().unwrap())
            .collect::<Vec<_>>(),
        vec![false, false, true]
    );
    let queued = intent::QueuedMove {
        goal: Point(3000.0, 0.0),
    };
    let later = queued.activate(Point(2500.0, 0.0));
    assert!(!later.automatic_roads());
    records.push(json!({"accepted_trigger_m":intent::AUTOMATIC_ROAD_TRIGGER_M,"boundary":boundary,"queued_activate_early":queued.activate(Point(0.0,0.0)).automatic_roads(),"queued_activate_later":later.automatic_roads(),"later_replan":later.replan_from(Point(0.0,0.0)).automatic_roads}));
    let lines = [
        vec![[20.0, 128.0], [236.0, 128.0]],
        vec![[20.0, 128.0], [100.0, 80.0], [156.0, 80.0], [236.0, 128.0]],
    ];
    let roads: Vec<_> = lines
        .iter()
        .flat_map(|line| {
            line.windows(2).map(|p| Segment {
                a: Point(p[0][0], p[0][1]),
                b: Point(p[1][0], p[1][1]),
            })
        })
        .collect();
    let prepared = Prepared::prepare(&roads);
    records.push(json!({"prepared_identity":prepared.identity,"nodes":prepared.points.len(),"edges":prepared.edges.len(),"preparation":format!("{:?}",prepared.preparation),"preparation_scope":"loading only; tiny geography, no scale or latency admission"}));
    let mut maps = Vec::new();
    for case in 0..4 {
        let mut raw = json!({"size":[256,256],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"roads":lines.iter().map(|p|json!({"points":p,"width_m":12})).collect::<Vec<_>>()});
        if case == 1 {
            raw["props"] =
                json!([{"kind":"wall","center":[128,128],"yaw":0.4,"half_extents":[4,16,2]}]);
        }
        if case >= 2 {
            raw["water"] = json!([{"rect":[118,0,20,256],"bed_z":-2,"surface_z":-0.5}]);
        }
        if case == 3 {
            raw["bridges"] = json!([{"deck":"bridge_deck","center":[128,128],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]);
        }
        maps.push(raw);
    }
    for (case, raw) in maps.iter().enumerate() {
        let map: MapDefinition = serde_json::from_value(raw.clone()).unwrap();
        let world = WorldGeometry::new(&map, &rules);
        let grid = NavGrid::build(
            &world,
            world.props().cloned(),
            rules.physics.soldier_radius_m,
        );
        for kind in ["rifle", "jeep"] {
            for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
                let m = mobility(&rules, kind);
                let id = fold(
                    fold(rules_identity, input_hash(&raw.to_string())),
                    fold(input_hash(kind), policy as u64),
                );
                let context = GridCheck {
                    grid: &grid,
                    m,
                    policy,
                    id,
                };
                let mut job = Request::new(
                    prepared.clone(),
                    context,
                    Point(22.0, 130.0),
                    Point(234.0, 130.0),
                    8.0,
                    if policy == RoutePolicy::Shortest {
                        1.0
                    } else {
                        1.0 / m.speed(true, false, 0.0).max(0.1)
                    },
                );
                let trace = finish(&mut job, 128);
                assert_eq!(
                    job.status(),
                    if case == 2 {
                        Status::CorridorDeclined
                    } else {
                        Status::Ready
                    }
                );
                check_cost(&job);
                let result = record(&job, trace);
                records.push(json!({"case":case,"input":raw,"kind":kind,"policy":format!("{policy:?}"),"result":result}));
            }
        }
    }
    // Independent captured belief proxies, same public road topology, interleaved work.
    let worlds: Vec<_> = [0, 1]
        .into_iter()
        .map(|i| WorldGeometry::new(&serde_json::from_value(maps[i].clone()).unwrap(), &rules))
        .collect();
    let grids: Vec<_> = worlds
        .iter()
        .map(|w| NavGrid::build(w, w.props().cloned(), rules.physics.soldier_radius_m))
        .collect();
    let m = mobility(&rules, "jeep");
    let mut jobs: Vec<_> = (0..2)
        .map(|i| {
            Request::new(
                prepared.clone(),
                GridCheck {
                    grid: &grids[i],
                    m,
                    policy: RoutePolicy::Fastest,
                    id: fold(rules_identity, input_hash(&maps[i].to_string())),
                },
                Point(22.0, 130.0),
                Point(234.0, 130.0),
                8.0,
                1.0 / m.road_mps.max(0.1),
            )
        })
        .collect();
    assert!(std::sync::Arc::ptr_eq(
        jobs[0].prepared(),
        jobs[1].prepared()
    ));
    let mut traces = [Vec::new(), Vec::new()];
    for _ in 0..100000 {
        for (i, j) in jobs.iter_mut().enumerate() {
            if j.status() != Status::Pending {
                continue;
            }
            let mut budget = 64;
            let before = j.work;
            let state = j.poll(&mut budget);
            assert_eq!(j.work - before, 64 - budget);
            assert_eq!(j.digest(), j.reference_digest());
            traces[i].push([
                j.work as u64,
                j.digest(),
                if state == Status::Ready { 1 } else { 0 },
            ]);
        }
        if jobs.iter().all(|j| j.status() != Status::Pending) {
            break;
        }
    }
    assert!(jobs.iter().all(|j| j.status() == Status::Ready));
    assert!(jobs[1].searches > jobs[0].searches);
    for j in &jobs {
        check_cost(j);
    }
    records.push(json!({"concurrent_belief_proxies":true,"shared_topology":true,"complete":[record(&jobs[0],std::mem::take(&mut traces[0])),record(&jobs[1],std::mem::take(&mut traces[1]))]}));
    records
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let raw = sim::fixtures::village();
    std::fs::write(
        "throwaway/navigation-counted/input.json",
        serde_json::to_vec(&raw).unwrap(),
    )
    .unwrap();
    let a = compute(raw.clone());
    let b = compute(raw);
    assert_eq!(a, b, "complete same-build replay");
    println!("{}", serde_json::to_string(&a).unwrap());
}
