use super::{
    candidate::{NavGrid, Refinement, RefinementStatus},
    math::v2,
};
use contract::{
    command::{CommandEnvelope, MoveDirection, Order, RoutePolicy},
    ids::{Side, UnitId},
    observation::MoveState,
    scenario::ScenarioDefinition,
};
use sim::{battle::Battle, world::WorldGeometry};
pub fn run() {
    let mut rules = sim::fixtures::village();
    for document in rules["catalog"].as_array_mut().unwrap() {
        if document["units"]["jeep"].is_object() {
            document["units"]["jeep"]["mobility"]["wheeled"]["road_mps"] =
                serde_json::json!(110.0 / 3.6);
            document["units"]["jeep"]["mobility"]["wheeled"]["mps"] = serde_json::json!(55.0 / 3.6);
        }
    }
    let map = serde_json::json!({"size":[10000,10000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"roads":[{"points":[[0,5000],[10000,5000]],"width_m":12}]});
    let raw = serde_json::json!({"map":map,"rules":rules,"units":[{"side":"blue","kind":"jeep","position":[2,5000],"yaw":0}],"events":[],"scripts":[]});
    let scenario: ScenarioDefinition = serde_json::from_value(raw.clone()).unwrap();
    let mut b = Battle::new(&scenario, 1);
    let actual = b.unit(UnitId(0)).unwrap().mobility;
    let m = super::candidate::Mobility {
        off_road_mps: actual.off_road_mps,
        road_mps: actual.road_mps,
        forest_multiplier: actual.forest_multiplier,
        half_width_m: actual.half_width_m,
        class: actual.class,
        push: actual.push,
        drive: None,
    };
    assert_eq!(m.road_mps.to_bits(), (110.0f64 / 3.6).to_bits());
    let world = WorldGeometry::new(&scenario.map, &scenario.rules);
    let grid = NavGrid::build(
        &world,
        world.props().cloned(),
        scenario.rules.physics.soldier_radius_m,
    );
    let roads = [super::road_graph::Segment {
        a: super::road_graph::Point(0.0, 5000.0),
        b: super::road_graph::Point(10000.0, 5000.0),
    }];
    let coarse = super::road_graph::plan(
        &roads,
        super::road_graph::Point(2.0, 5000.0),
        super::road_graph::Point(5000.0, 5000.0),
        256.0,
        |a, b, road| Some(a.distance(b) / if road { m.road_mps } else { m.off_road_mps }),
    )
    .unwrap();
    assert_eq!(
        coarse.points,
        vec![
            super::road_graph::Point(2.0, 5000.0),
            super::road_graph::Point(5000.0, 5000.0)
        ]
    );
    let mut poll_instructions = Vec::new();
    let mut poll_ms = Vec::new();
    let mut refinement =
        Refinement::new(v2(2.0, 5000.0), v2(5000.0, 5000.0), m, RoutePolicy::Fastest);
    let mut planning_ticks = 0;
    let expected = loop {
        let mut budget = 8192;
        let before = refinement.work;
        let instructions_before = super::instructions::instructions();
        let start = std::time::Instant::now();
        let status = refinement.poll(&grid, &mut budget);
        poll_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        poll_instructions.push(
            super::instructions::instructions()
                .zip(instructions_before)
                .map(|(a, b)| a - b),
        );
        assert!(refinement.work - before <= 8192);
        planning_ticks += 1;
        b.step();
        match status {
            RefinementStatus::Pending => assert!(
                planning_ticks < scenario.rules.tick_hz * 15,
                "planning allowance inside the stated arrival proof is finite"
            ),
            RefinementStatus::Complete(Some(c)) => break c,
            RefinementStatus::Complete(None) => {
                panic!("connected clear road failed physical refinement")
            }
        }
    };
    let accepted = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [5000.0, 5000.0],
            route: RoutePolicy::Fastest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(accepted.error, None);
    let mut movement_ticks = 0;
    let mut actual_length = 0.0;
    let mut previous = b.unit(UnitId(0)).unwrap().position.xy();
    let mut tick_digests = Vec::new();
    let mut arrived = false;
    for _ in 0..scenario.rules.tick_hz * 200 {
        b.step();
        movement_ticks += 1;
        let unit = b.unit(UnitId(0)).unwrap();
        let p = unit.position.xy();
        actual_length += (p - previous).length();
        previous = p;
        tick_digests.push(format!("{:016x}", b.digest()));
        if unit.state == MoveState::Idle && (p - v2(5000.0, 5000.0)).length() < 2.0 {
            arrived = true;
            break;
        }
    }
    let elapsed = (planning_ticks + movement_ticks) as f64 / scenario.rules.tick_hz as f64;
    println!(
        "{}",
        serde_json::json!({"transit_proof":true,"allocator_bytes":super::allocation::bytes(),"profile":"proof-only Jeep light vehicle, 110/55 km/h; existing wheel/body/steering unchanged","playable_m":[10000,10000],"physical_world_m":[10000,10000],"rendered_margin":"not represented in this native route arm","start":[2,5000],"goal":[5000,5000],"planned_length_m":4998,"actual_movement_length_m":actual_length,"first_last_local_connectors_m":[0,0],"planning_ticks":planning_ticks,"planning_work":refinement.work,"planning_budget_per_tick":8192,"planning_poll_instructions":poll_instructions,"planning_poll_ms":poll_ms,"coarse_graph_nodes":coarse.nodes,"coarse_graph_relaxed":coarse.relaxed,"coarse_road_time_s":coarse.cost,"tick_hz":scenario.rules.tick_hz,"movement_ticks":movement_ticks,"order_request_to_arrival_s":elapsed,"reference_segment_time_s":expected,"arrived":arrived,"fixture":raw,"tick_digests":tick_digests})
    );
    assert!(
        arrived && elapsed <= 180.0,
        "trial LightVehicle midpoint transit target remains red: {elapsed}s"
    );
}
