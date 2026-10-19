//! When a skirmish purchase leaves the queue, and what it arrives as.

use contract::command::{CommandEnvelope, Order};
use contract::ids::Side;
use sim::battle::Battle;

fn buy(battle: &mut Battle, player: &str, seq: u64, destination: [f64; 2]) {
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: Some(player.into()),
        seq,
        order: Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination,
        },
        queued: false,
    });
    assert_eq!(ack.error, None, "purchase {seq} refused");
}

fn run(battle: &mut Battle, seconds: u32) {
    for _ in 0..seconds * battle.rules().tick_hz {
        battle.step();
    }
}

#[test]
fn a_purchase_blocked_at_one_entry_does_not_hold_back_the_same_players_purchase_at_another() {
    let mut setup = crate::skirmish_team::two_entry_setup();
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 0;
    let mut battle = Battle::new(&setup, 5);
    let entry_a = [100.0, 10.0];
    // The first tank parks on entry A, so the second waits there, blocked.
    buy(&mut battle, "blue-1", 1, entry_a);
    buy(&mut battle, "blue-1", 2, entry_a);
    run(&mut battle, 3);
    let status = battle.observe(Side::Blue).skirmish.clone().unwrap();
    assert_eq!(status.pending.len(), 1);
    assert!(status.pending[0].blocked, "entry A is not blocked");
    // The same player's next purchase is routed through entry B.
    buy(&mut battle, "blue-1", 3, [700.0, 60.0]);
    run(&mut battle, 2);
    let frame = battle.observe(Side::Blue);
    assert!(
        frame.own.iter().any(|unit| unit.position[0] > 650.0),
        "the entry-B purchase never arrived: {:?}",
        frame
            .own
            .iter()
            .map(|unit| unit.position)
            .collect::<Vec<_>>()
    );
    let status = frame.skirmish.as_ref().unwrap();
    assert_eq!(status.pending.len(), 1, "the entry-A purchase still waits");
}
