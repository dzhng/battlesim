use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use sim::battle::Battle;

fn main() {
    let fixture = sim::fixtures::village();
    let setup = sim::village::scenario(&fixture, "prepared_crossfire").unwrap();
    let mut battle = Battle::new(&setup, 21);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue, seq: 1, queued: false,
        order: Order::Move { units: vec![UnitId(4),UnitId(5)], gesture:1,goal:[900.0,800.0],
            route:RoutePolicy::Shortest,direction:MoveDirection::Forward,facing:None }
    });
    assert_eq!(ack.error,None);
    for _ in 0..60 * fixture["tick_hz"].as_u64().unwrap() {
        battle.step();
        println!("DIGEST {} {:016x}",battle.tick(),battle.digest());
    }
}
