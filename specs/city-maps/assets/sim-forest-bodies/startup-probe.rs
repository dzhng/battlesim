// Temporary C77 paired startup probe; not a production harness.
#[path = "common/instructions.rs"]
mod instructions;
fn main() {
    let enabled = std::env::args().nth(1).as_deref() == Some("on");
    let mut fixture = sim::fixtures::village();
    fixture["forests"]["rule"]["logs_per_ha"] = serde_json::json!(if enabled {5} else {0});
    fixture["forests"]["rule"]["boulders_per_ha"] = serde_json::json!(if enabled {3} else {0});
    let setup = sim::village::scenario(&fixture,"ordinary").unwrap();
    let begin = instructions::resources();
    let wall = std::time::Instant::now();
    let battle = sim::battle::Battle::new(&setup,1);
    let elapsed = wall.elapsed();
    let end = instructions::resources();
    let world = battle.world();
    let count = |name:&str| world.props().filter(|p| world.types().id(p.kind) == name).count();
    let attempts = |density:f64| {
        if density == 0. {return 0usize;}
        let step = (10000./density).sqrt();
        setup.map.forests.iter().map(|forest| {
            let [x0,y0,x1,y1] = forest.shape.limits();
            let mut total = 0;
            let mut y = y0+step/2.;
            while y<=y1 {
                let mut x=x0+step/2.;
                while x<=x1 {total+=1;x+=step;}
                y+=step;
            }
            total
        }).sum::<usize>()
    };
    let logs= count("log"); let boulders=count("boulder");
    let la=attempts(setup.rules.forests.rule.logs_per_ha);
    let ba=attempts(setup.rules.forests.rule.boulders_per_ha);
    let retired = end.zip(begin).map(|(a,b)| (a.0-b.0,a.1-b.1));
    println!("floor={} startup_ms={:.3} resources={:?} logs={} boulders={} attempted_logs={} attempted_boulders={} rejected_logs={} rejected_boulders={} props={} trunks={} digest={:016x}",enabled,elapsed.as_secs_f64()*1000.,retired,logs,boulders,la,ba,la-logs,ba-boulders,world.props().count(),world.props().filter(|p|p.forest_tree).count(),battle.digest());
}
