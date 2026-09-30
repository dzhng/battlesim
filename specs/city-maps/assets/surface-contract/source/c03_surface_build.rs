//! Temporary paired C03 attribution probe; frozen with its evidence before removal.
#[path = "common/instructions.rs"]
mod instructions;
fn main() {
    let input = sim::fixtures::village();
    let rules: contract::scenario::Rules=serde_json::from_value(input.clone()).unwrap();
    let map: contract::map::MapDefinition=serde_json::from_value(input["map"].clone()).unwrap();
    let world=sim::world::WorldGeometry::new(&map,&rules);
    let before=instructions::instructions().unwrap();
    let grid=std::hint::black_box(sim::navigation::NavGrid::build(&world,world.props().cloned(),0.3));
    let spent=instructions::instructions().unwrap()-before;
    println!("{}",serde_json::json!({"nav_build_instructions":spent,"storage":grid.storage()}));
}
