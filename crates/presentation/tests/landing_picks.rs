//! Pick flags follow the same published grounding as the rendered aircraft.
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use presentation::presentation::{Presentation, Ui};
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use sim::publication::{self, Publisher};

#[test]
fn a_helicopters_pick_is_aloft_only_until_touchdown_for_either_side() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for side in [Side::Blue, Side::Red] {
        let rules = presentation::static_world::files::rules(&root, "supply").unwrap();
        let scenario = sim::maps::encounter_scenario("supply", "heli-fire", rules).unwrap();
        let mut battle = sim::battle::Battle::new(&scenario, 13);
        let kinds = presentation::fielded::encounter_kinds(&root, "supply", "heli-fire").unwrap();
        let fielded = presentation::fielded::fielded(&root, battle.rules(), &kinds).unwrap();
        let hz = f64::from(battle.rules().tick_hz);
        let mut p = Presentation::new(Some(&publication::layout_json(&battle)), hz).unwrap();
        p.load_world(
            WorldInput {
                battle: Some(BattleInput {
                    units: fielded.units,
                    effects: fielded.effects,
                    side: if side == Side::Blue { "blue" } else { "red" }.into(),
                    tick_hz: hz,
                    soldier: SoldierBody {
                        radius_m: 0.2,
                        height_m: 1.8,
                    },
                    xray: Xray {
                        own: [0.0; 4],
                        selected: [0.0; 4],
                        ally: [0.0; 4],
                    },
                    blocked: [0.0; 4],
                    stand_in: None,
                }),
                sun_azimuth: 0.0,
                sun_elevation: 0.7,
                buildings: None,
            },
            Default::default(),
            None,
        )
        .unwrap();
        let camera = Camera {
            eye: [180.0, 100.0, 100.0],
            target: [180.0, 200.0, 0.0],
            up: [0.0, 0.0, 1.0],
            fov_y: 0.9,
            near: 0.1,
            size: [1280, 800],
        };
        let mut publisher = Publisher::new();
        let mut flying = false;
        let mut landed = false;
        let mut packed = false;
        for _ in 0..900 {
            battle.step();
            let words = publisher
                .publish(&battle, side)
                .unwrap()
                .iter()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>();
            let time = battle.tick() as f64 / hz;
            p.push_publication(&words, time).unwrap();
            let o = p.observation().unwrap();
            let heli = o
                .own
                .iter()
                .find(|u| &*u.kind == "test_supply_heli")
                .map(|u| (u.id, u.grounded))
                .or_else(|| {
                    o.identified
                        .iter()
                        .find(|u| &*u.kind == "test_supply_heli")
                        .map(|u| (u.id, u.grounded))
                });
            let frame = p
                .frame(time + 1.0 / hz, &camera, &Ui::default(), 0)
                .unwrap();
            let Some((id, grounded)) = heli else {
                continue;
            };
            let owner_flag = if side == Side::Blue { 1 } else { 2 };
            let pick = frame
                .picks
                .iter()
                .find(|p| p.unit == id as u32 && p.flags & 3 == owner_flag)
                .unwrap();
            assert_eq!(pick.flags & 4 != 0, !grounded);
            flying |= !grounded;
            landed |= grounded;
            if flying && landed {
                break;
            }
            // A side may first identify it after touchdown. Lift it so that
            // viewer also observes flight through the same public pipeline.
            if landed && !packed {
                let ack = battle.accept(CommandEnvelope {
                    side: Side::Blue,
                    player: None,
                    seq: 1,
                    order: Order::SetDeployment {
                        units: vec![UnitId(0)],
                        deployed: false,
                    },
                    queued: false,
                });
                assert_eq!(ack.error, None);
                packed = true;
            }
        }
        assert!(
            flying && landed,
            "both flight and touchdown are observed: {side:?}, flying={flying}, landed={landed}"
        );
    }
}
