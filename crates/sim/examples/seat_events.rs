//! Usage: cargo run -p sim --release --example seat_events
//! A native 5v5, recorded transfers, and tick-by-tick replay proof.
use contract::ids::Side;
use contract::players::{ControllerKind, Player};
use sim::battle::Battle;

fn main() {
    let players: Vec<Player> = [Side::Blue, Side::Red]
        .into_iter()
        .flat_map(|side| {
            (1..=5).map(move |slot| Player {
                id: format!("{}-{slot}", if side == Side::Blue { "blue" } else { "red" }),
                side,
                faction: Some(if side == Side::Blue {
                    contract::catalog::Faction::Us
                } else {
                    contract::catalog::Faction::Eastern
                }),
                controller: ControllerKind::ActiveAi,
                human_capable: true,
            })
        })
        .collect();
    let mut rules = sim::skirmish::starter_rules();
    rules.preparation_s = 0;
    let size = [1200.0, 800.0];
    let skirmish = sim::skirmish::setup_for(
        &players,
        sim::skirmish::synthetic_sites(size, contract::players::entries_per_side(&players)),
        rules,
    );
    let setup = serde_json::from_value(serde_json::json!({
        "map":{"size":size,"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":sim::fixtures::rules(), "players":players,"units":[],"skirmish":skirmish
    }))
    .unwrap();
    let mut live = Battle::new(&setup, 7);
    let hz = u64::from(live.rules().tick_hz);
    let transfer_tick = 30 * hz;
    let end = 60 * hz;
    let mut digests = vec![live.digest()];
    let mut first_arrival = false;
    while live.tick() < end && !live.finished() {
        if live.tick() == transfer_tick {
            let view = live.observe(Side::Blue).skirmish.as_ref().unwrap();
            let source = view.players.iter().find(|p| p.id == "blue-1").unwrap();
            println!(
                "hand_over tick={} units={} credits={:.6}",
                live.tick() + 1,
                source.unit_count,
                source.credits
            );
            live.hand_over("blue-1").unwrap();
        }
        if live.tick() == transfer_tick + 1 {
            live.set_controllers(vec![("blue-1".into(), ControllerKind::Human)]);
            live.reset_seat("blue-1", true).unwrap();
            println!("reset tick={} opening=true", live.tick() + 1);
        }
        if live.tick() == transfer_tick + 2 {
            let variant = live
                .rules()
                .catalog
                .cards()
                .filter(|card| {
                    card.disabled_reason.is_none()
                        && card
                            .roster
                            .factions
                            .contains(&contract::catalog::Faction::Us)
                })
                .filter(|card| {
                    live.rules().catalog.index(card.id).is_some_and(|kind| {
                        sim::units::ground_mobility(live.rules().catalog.get(kind), live.rules())
                            .is_some()
                    })
                })
                .min_by_key(|card| card.cost)
                .expect("a ground unit in the faction")
                .id
                .to_owned();
            let ack = live.accept_order(
                Side::Blue,
                Some("blue-1".into()),
                contract::command::Order::ConfirmPurchase {
                    variant: variant.clone(),
                    carrier: None,
                    destination: [600.0, 180.0],
                },
                false,
            );
            assert_eq!(ack.error, None);
            println!(
                "first purchase tick={} seq={} variant={variant}",
                ack.applied_tick, ack.seq
            );
        }
        live.step();
        if live.tick() > transfer_tick + 2 && !first_arrival {
            if let Some(unit) = live
                .observe(Side::Blue)
                .own
                .iter()
                .find(|u| u.owner == Some(contract::players::PlayerIndex(0)))
            {
                println!(
                    "first entry tick={} owner=blue-1 side=blue position={:?}",
                    live.tick(),
                    unit.position
                );
                first_arrival = true;
            }
        }
        digests.push(live.digest());
    }
    assert!(first_arrival, "the reset seat’s purchase must drive in");
    let record = live.replay();
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    assert_eq!(replay.digest(), digests[0]);
    while replay.tick() < live.tick() {
        replay.step();
        assert_eq!(
            replay.digest(),
            digests[replay.tick() as usize],
            "tick {}",
            replay.tick()
        );
        if replay.tick().is_multiple_of(10 * hz) || replay.tick() == transfer_tick + 1 {
            println!(
                "tick={} live={:016x} replay={:016x}",
                replay.tick(),
                digests[replay.tick() as usize],
                replay.digest()
            );
        }
    }
    println!(
        "matched {} ticks, {} seat events",
        live.tick(),
        record.seat_events.len()
    );
}
