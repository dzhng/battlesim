//! Generated battle guards: working mounts, setup and movement.
use super::*;

/// A durable rifle squad and tank in reach, holding fire until fired on.
fn targets(range: f64) -> Value {
    json!([
        { "side": "red", "kind": "test_rifle", "position": [200.0 + range * 0.5, 300.0 - range * 0.3], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [200.0 + range * 0.5, 300.0 + range * 0.3], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
    ])
}

/// A battle on open ground: the type under test at [200, 300], and `others`.
fn battle(fixture: &Value, id: &str, others: Value) -> Battle {
    battle_at(fixture, id, others, [200.0, 300.0], [700.0, 600.0], 0.0)
}

fn battle_at(
    fixture: &Value,
    id: &str,
    others: Value,
    at: [f64; 2],
    size: [f64; 2],
    yaw: f64,
) -> Battle {
    let mut units = vec![json!({ "side": "blue", "kind": id, "position": at, "yaw": yaw })];
    units.extend(others.as_array().unwrap().iter().cloned());
    let setup = serde_json::from_value(json!({
        "map": { "size": size, "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// Every test unit type: it sets up, fires each mount within a minute at a
/// squad and a tank in reach, and on open ground drives or walks where it
/// is ordered.
#[test]
fn every_unit_type_sets_up_fires_each_mount_and_moves() {
    let mut fixture = with_m1_family();
    grenade_firing_profile(&mut fixture);
    exercise(fixture, |_| true);
}

/// Every aircraft that lands uses the same mount and movement guard, including
/// game cards: sharing a model must not leave a mount inert or block take-off.
#[test]
fn every_deployable_aircraft_sets_up_fires_each_mount_and_moves() {
    let mut fixture = fixtures::rules();
    fixture["catalog"] = json!(fixtures::catalog_documents(fixtures::CatalogSet::Test));
    exercise(fixture, |unit| {
        unit.mobility.flight().is_some() && unit.capabilities.deploy.is_some()
    });
}

fn exercise(mut fixture: Value, include: impl Fn(&contract::catalog::UnitType) -> bool) {
    fixtures::patch_catalog(
        &mut fixture,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_tank",
        json!({ "body": { "hull": { "hp": 1.0e6 } } }),
    );
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let mut failures = Vec::new();
    for t in rules.catalog.indices() {
        if !include(rules.catalog.get(t)) {
            continue;
        }
        let id = rules.catalog.id(t);
        let range = rules
            .catalog
            .mounts(t)
            .iter()
            .flat_map(|m| &m.def.weapons)
            .map(|w| rules.weapons[w].ballistics.range_m)
            .fold(f64::INFINITY, f64::min);
        let never_stops = rules.catalog.get(t).mobility.flight()
            == Some(contract::catalog::FlightModel::FixedWing);
        let mut others = if never_stops {
            json!([
                { "side": "red", "kind": "test_rifle", "position": [1100, 995], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_tank", "position": [1100, 1005], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_heli", "position": [1200, 1000], "engagement": "return_fire_only" },
            ])
        } else {
            targets(range.min(300.0))
        };
        // A weapon that reaches only aircraft gets a helicopter hovering in
        // reach; the rest never see one, so none takes its return fire.
        let air_only = rules.catalog.mounts(t).iter().any(|m| {
            m.def.weapons.iter().all(|w| {
                !rules.weapons[w]
                    .targets
                    .contains_key(&AltitudeLayer::Ground)
            })
        });
        if air_only {
            others.as_array_mut().unwrap().push(json!(
                { "side": "red", "kind": "test_heli", "position": [400, 300], "engagement": "return_fire_only" }
            ));
        }
        let mut b = if never_stops {
            battle_at(&fixture, id, others, [500.0, 1000.0], [3000.0, 2000.0], 0.0)
        } else {
            battle(&fixture, id, others)
        };
        if never_stops {
            let ack = b.accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 1,
                order: Order::Attack {
                    units: vec![UnitId(0)],
                    target: TargetRef::Ground {
                        point: [1100.0, 1000.0, 0.0],
                    },
                },
                queued: false,
            });
            assert_eq!(ack.error, None, "{id}: {ack:?}");
        }
        for _ in 0..60 * rules.tick_hz {
            b.step();
        }
        for (m, mount) in b.unit(UnitId(0)).unwrap().mounts.iter().enumerate() {
            // Sibling door guns deliberately share a priority target and
            // only the facing one fires. Exercise the other arc in its own
            // orientation, rather than asking it to shoot through the hull.
            let spec = &rules.catalog.mounts(t)[m].def;
            let shots = match (mount.shots, spec.arc) {
                (0, Some(arc)) => {
                    let yaw = -arc.centre_deg.to_radians();
                    let mut facing = battle_at(
                        &fixture,
                        id,
                        targets(range.min(300.0)),
                        [200.0, 300.0],
                        [700.0, 600.0],
                        yaw,
                    );
                    for _ in 0..60 * rules.tick_hz {
                        facing.step();
                    }
                    facing.unit(UnitId(0)).unwrap().mounts[m].shots
                }
                (shots, _) => shots,
            };
            if shots == 0 {
                let name = &spec.id;
                failures.push(format!(
                    "{id}: mount {name} never fired ({:?})",
                    mount.reason
                ));
            }
        }

        // A jet flies to its goal on the open field it fought on: the small
        // field lies wholly inside the edge guard's margin.
        let (mut b, goal) = if never_stops {
            (
                battle_at(
                    &fixture,
                    id,
                    json!([]),
                    [500.0, 1000.0],
                    [3000.0, 2000.0],
                    0.0,
                ),
                sim::math::v2(1500.0, 1300.0),
            )
        } else {
            (battle(&fixture, id, json!([])), sim::math::v2(200.0, 500.0))
        };
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            order: Order::Move {
                units: vec![UnitId(0)],
                gesture: 1,
                goal: [goal.x, goal.y],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: false,
        });
        assert_eq!(ack.error, None, "{id}: {ack:?}");
        // A jet never stops and cannot turn inside its turning circle, so it
        // has arrived once it comes within one turning radius of its goal.
        let turning_radius = match b.unit(UnitId(0)).unwrap().motion {
            sim::units::Motion::Air(flight)
                if flight.model == contract::catalog::FlightModel::FixedWing =>
            {
                Some(flight.cruise_mps / flight.turn_rad_s)
            }
            _ => None,
        };
        if let Some(radius) = turning_radius {
            let mut closest = f64::INFINITY;
            for _ in 0..90 * rules.tick_hz {
                b.step();
                closest = closest.min((b.unit(UnitId(0)).unwrap().position.xy() - goal).length());
            }
            if closest > radius + 10.0 {
                failures.push(format!(
                    "{id}: never came within its {radius:.0} m turning radius of its goal ({closest:.1} m)"
                ));
            }
        } else {
            for _ in 0..90 * rules.tick_hz {
                b.step();
            }
            let kind = rules.catalog.get(t);
            if kind.mobility.flight().is_some() && kind.capabilities.deploy.is_some() {
                let own = &b.observe(Side::Blue).own[0];
                assert!(
                    own.grounded && own.deployment.as_ref().is_some_and(|d| d.progress == 1.0),
                    "{id}: did not land and finish setting up after its move"
                );
            }
            let short = (b.unit(UnitId(0)).unwrap().position.xy() - goal).length();
            if short > 10.0 {
                failures.push(format!("{id}: stopped {short:.1} m short of its goal"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
