//! Proof only: scalar rectangle macro admission against the frozen executable search.
use contract::{
    command::RoutePolicy,
    map::{MapDefinition, MoverClass},
    scenario::PushClass,
};
use math::v2;
pub use sim::{math, world};
#[allow(dead_code)]
#[path = "@DENSE@"]
mod dense;
#[allow(dead_code)]
mod traced;
fn same_public(a: dense::Plan, b: traced::Plan) -> bool {
    match (a, b) {
        (dense::Plan::Route(a), traced::Plan::Route(b)) => a == b,
        (dense::Plan::Blocked(a), traced::Plan::Blocked(b)) => format!("{a:?}") == format!("{b:?}"),
        _ => false,
    }
}
fn main() {
    let reject_assert = std::env::args().any(|a| a == "--assert-scalar");
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::village()).unwrap();
    let map: MapDefinition = serde_json::from_value(serde_json::json!({
        "size":[32,32],"height_grid_m":4,"slope_cutoff_deg":35}))
    .unwrap();
    let w = world::WorldGeometry::new(&map, &rules);
    let labels = [
        0.0,
        0.1,
        1.0,
        2.0,
        4.0,
        2.0 + 2.0 * std::f64::consts::SQRT_2,
        f64::from_bits(1.0f64.to_bits() - 1),
        f64::from_bits(4096.0f64.to_bits() - 1),
        4096.0,
        1e12,
    ];
    let mut cases = 0;
    let mut zero_crosschecks = 0;
    let mut first = None;
    let mut first_block_failure = None;
    let mut first_zero_block_failure = None;
    // Increasing displacement, then dy defines smallest in this enumerated family.
    for total in 2usize..=10 {
        for dy in 1..total {
            let dx = total - dy;
            if dx == dy || dx > 6 || dy > 6 {
                continue;
            }
            for (sx, sy) in [(1isize, 1isize), (-1, 1), (1, -1), (-1, -1)] {
                let start = [7usize, 7usize];
                let target = [
                    (7isize + sx * dx as isize) as usize,
                    (7isize + sy * dy as isize) as usize,
                ];
                let rectangle = [
                    start[0].min(target[0]),
                    start[1].min(target[1]),
                    start[0].max(target[0]),
                    start[1].max(target[1]),
                ];
                for policy in [RoutePolicy::Shortest, RoutePolicy::Fastest] {
                    for class in [MoverClass::Infantry, MoverClass::Vehicle] {
                        let m = traced::Mobility {
                            off_road_mps: 6.0,
                            road_mps: 12.0,
                            forest_multiplier: 0.4,
                            half_width_m: 0.1,
                            class,
                            push: PushClass::Medium,
                            drive: None,
                        };
                        let old = dense::Mobility {
                            off_road_mps: m.off_road_mps,
                            road_mps: m.road_mps,
                            forest_multiplier: m.forest_multiplier,
                            half_width_m: m.half_width_m,
                            class,
                            push: m.push,
                            drive: None,
                        };
                        let from = v2(2.0 * start[0] as f64 + 1.0, 2.0 * start[1] as f64 + 1.0);
                        let goal = v2(2.0 * target[0] as f64 + 1.0, 2.0 * target[1] as f64 + 1.0);
                        for incoming in labels {
                            let mut t = traced::NavGrid::build(&w, w.props().cloned(), 0.3);
                            let (plan, g, chain, events) =
                                t.proof_run(from, goal, &m, policy, incoming, rectangle);
                            if incoming == 0.0 {
                                let mut d = dense::NavGrid::build(&w, w.props().cloned(), 0.3);
                                assert!(
                                    same_public(d.plan(from, goal, &old, policy), plan),
                                    "zero-label instrumentation changed untouched oracle"
                                );
                                zero_crosschecks += 1;
                            }
                            cases += 1;
                            let speed = if policy == RoutePolicy::Shortest {
                                1.0
                            } else {
                                6.0
                            };
                            // Same single-edge half-sum; candidate only reassociates accumulation.
                            let cardinal = ((2.0 / speed) + (2.0 / speed)) / 2.0;
                            let diagonal = ((2.0 * std::f64::consts::SQRT_2 / speed)
                                + (2.0 * std::f64::consts::SQRT_2 / speed))
                                / 2.0;
                            let macro_g = incoming
                                + (dx.max(dy) - dx.min(dy)) as f64 * cardinal
                                + dx.min(dy) as f64 * diagonal;
                            let mut cardinal_first = incoming;
                            for _ in 0..dx.max(dy) - dx.min(dy) {
                                cardinal_first += cardinal;
                            }
                            for _ in 0..dx.min(dy) {
                                cardinal_first += diagonal;
                            }
                            let mut diagonal_first = incoming;
                            for _ in 0..dx.min(dy) {
                                diagonal_first += diagonal;
                            }
                            for _ in 0..dx.max(dy) - dx.min(dy) {
                                diagonal_first += cardinal;
                            }
                            if g.to_bits() != cardinal_first.min(diagonal_first).to_bits()
                                && (first_block_failure.is_none()
                                    || (incoming == 0.0 && first_zero_block_failure.is_none()))
                            {
                                let witness = serde_json::json!({
                                    "start":start,"target":target,"rectangle":rectangle,
                                    "policy":format!("{policy:?}"),"class":format!("{class:?}"),
                                    "incoming_g_bits":format!("{:016x}",incoming.to_bits()),
                                    "original_g_bits":format!("{:016x}",g.to_bits()),
                                    "cardinal_first_bits":format!("{:016x}",cardinal_first.to_bits()),
                                    "diagonal_first_bits":format!("{:016x}",diagonal_first.to_bits()),
                                    "original_parent_chain":chain,"accepted_boundary_updates":events});
                                if first_block_failure.is_none() {
                                    first_block_failure = Some(witness.clone());
                                }
                                if incoming == 0.0 {
                                    first_zero_block_failure = Some(witness);
                                }
                            }
                            if first.is_none() && g.to_bits() != macro_g.to_bits() {
                                assert!(
                                    chain.iter().all(|c| {
                                        let (x, y) = (c % 16, c / 16);
                                        x >= rectangle[0]
                                            && x <= rectangle[2]
                                            && y >= rectangle[1]
                                            && y <= rectangle[3]
                                    }),
                                    "winner left certified rectangle"
                                );
                                first = Some(
                                    serde_json::json!({"start":start,"target":target,"grid":[16,16],
                                    "rectangle":rectangle,"policy":format!("{policy:?}"),"class":format!("{class:?}"),
                                    "incoming_g_bits":format!("{:016x}",incoming.to_bits()),
                                    "cardinal_bits":format!("{:016x}",cardinal.to_bits()),
                                    "diagonal_bits":format!("{:016x}",diagonal.to_bits()),
                                    "original_g_bits":format!("{:016x}",g.to_bits()),
                                    "scalar_macro_g_bits":format!("{:016x}",macro_g.to_bits()),
                                    "original_parent_chain":chain,"accepted_boundary_updates":events}),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    let counterexample = first.expect("scalar macro unexpectedly passed; widen labels");
    let report = serde_json::json!({"verdict":"REJECT scalar rectangle cost quotient",
        "scope":"one entry, positive normal fixed costs, asymmetric rectangles; no production change",
        "cases":cases,"zero_label_public_oracle_matches":zero_crosschecks,
        "smallest_in_enumerated_family":counterexample,
        "best_of_two_step_blocks_counterexample":first_block_failure,
        "zero_incoming_two_step_blocks_counterexample":first_zero_block_failure});
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert!(
        !reject_assert,
        "scalar rectangle candidate failed exact boundary-label admission"
    );
}
