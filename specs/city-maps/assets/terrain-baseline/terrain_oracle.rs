use contract::map::MapDefinition;
use sim::math::v3;
use sim::world::{export, WorldGeometry};

fn main() {
    let map: MapDefinition = serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    let rules = serde_json::from_value(sim::fixtures::village()).unwrap();
    let world = WorldGeometry::new(&map, &rules);
    let mut rng = sim::rng::Rng::new(31);
    let bits = |v: Vec<f64>| v.into_iter().map(|x| format!("{:016x}", x.to_bits())).collect::<Vec<_>>();
    let mut queries = Vec::new();
    for k in 0..2048 {
        let x = if k < 128 { (k % 101) as f64 * 4.0 } else { rng.unit() * 400.0 };
        let y = if k < 128 { (k % 76) as f64 * 4.0 } else { rng.unit() * 300.0 };
        let origin = v3(x, y, 60.0);
        let dir = v3(rng.unit() - 0.5, rng.unit() - 0.5, -rng.unit()).normalized();
        queries.push(serde_json::json!({
            "point": bits(vec![x,y]), "surface": bits(export::surface_record(world.surface_at(x,y))),
            "ray": bits(vec![origin.x,origin.y,origin.z,dir.x,dir.y,dir.z,500.0]),
            "hit": bits(export::hit_record(world.raycast(origin,dir,500.0))),
        }));
    }
    println!("{}", serde_json::json!({"source_commit":"6ca090f","map":map,"queries":queries}));
}
