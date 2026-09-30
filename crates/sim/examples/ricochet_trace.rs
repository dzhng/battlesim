//! Ricochet trace: fire a weapon row at the fixture's tank, its front turned
//! `incidence_deg` off the line of fire, and print each round's flown
//! polyline and ending, then how often each face it met turned rounds away
//! (past about 27° the side is the face presented).
//! Hulls are judged by the battle's own policy (`damage::decide`).
//!
//!     cargo run -p sim --release --example ricochet_trace [row] [incidence_deg] [rounds] [penetration]
//!
//! `penetration` overrides the row's, e.g. a spent AP round:
//! `ricochet_trace tank_ap 20 20 120`.
use std::collections::BTreeMap;

use contract::ballistics::{FlightRules, WeaponBallistics};
use contract::ids::UnitId;
use contract::map::MapDefinition;
use contract::scenario::{RicochetRules, Rules};
use sim::damage::{decide, struck_face, RoundPower, StruckHull};
use sim::flight::{
    advance_projectiles, prepare_launch, Aim, Body, BodyId, FlightConfig, FlightEvent,
    ImpactContext, ImpactDecision, Pose, Projectiles, Struck,
};
use sim::math::{v3, V3};
use sim::rng::Rng;
use sim::world::WorldGeometry;

fn main() {
    let arg = |n: usize| std::env::args().nth(n);
    let row = arg(1).unwrap_or_else(|| "hmg".into());
    let incidence: f64 = arg(2).and_then(|s| s.parse().ok()).unwrap_or(60.0);
    let rounds: usize = arg(3).and_then(|s| s.parse().ok()).unwrap_or(20);
    let fixture = sim::fixtures::village();
    let section = |name: &str| fixture[name].clone();
    let flight: FlightRules = serde_json::from_value(section("physics")).unwrap();
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let tank = rules
        .catalog
        .by_id("tank")
        .hull()
        .expect("the tank has a hull");
    let ricochet: RicochetRules = serde_json::from_value(section("ricochet")).unwrap();
    let weapon_row = &fixture["weapons"][&row];
    assert!(weapon_row.is_object(), "no weapon row {row}");
    let weapon: WeaponBallistics = serde_json::from_value(weapon_row.clone()).unwrap();
    let power = RoundPower {
        penetration: arg(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(weapon_row["penetration"].as_f64().unwrap()),
        bursts: weapon_row["blast_radius_m"].as_f64().unwrap() > 0.0,
    };
    let [hx, hy, hz] = tank.half_extents_m;
    let half = v3(hx, hy, hz);
    let tick_hz = fixture["tick_hz"].as_u64().unwrap() as u32;
    let config = FlightConfig::new(&flight, tick_hz).unwrap();
    let map: MapDefinition = serde_json::from_value(serde_json::json!({
        "size": [600, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35
    }))
    .unwrap();
    let rules: contract::scenario::Rules = serde_json::from_value(fixture.clone()).unwrap();
    let world = WorldGeometry::new(&map, &rules);
    // The shooter stands 150 m west; the tank's front turns `incidence` off
    // the line of fire.
    let pose = Pose {
        base: v3(300.0, 300.0, 0.0),
        yaw: std::f64::consts::PI + incidence.to_radians(),
    };
    let body = Body {
        id: BodyId(1),
        unit: UnitId(1),
        shape: sim::flight::Shape::Box { half },
        from: pose,
        to: pose,
    };
    let profile = config.profile(&weapon).unwrap();
    let mut rng = Rng::new(20260925);
    let mut store = Projectiles::new(config.clone());
    for _ in 0..rounds {
        let aim = Aim {
            origin: v3(150.0, 300.0, 1.4),
            target: pose.base + v3(0.0, 0.0, half.z),
            target_velocity: V3::default(),
        };
        if let Ok((launch, _)) = prepare_launch(
            &world,
            &config,
            &profile,
            &aim,
            profile.scatter_mrad,
            &mut rng,
            None,
        ) {
            store.launch(launch);
        }
    }
    let mut paths: BTreeMap<u64, Vec<V3>> = store
        .active()
        .iter()
        .map(|p| (p.id.0, vec![p.position]))
        .collect();
    let mut endings: BTreeMap<u64, String> = BTreeMap::new();
    let mut faces: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let armor = tank.armor;
    let mut stream = Rng::new(7);
    let mut resolver = |hit: &ImpactContext| {
        let hull = match (hit.struck, hit.pose) {
            (Struck::Body(_), Some(pose)) => Some(StruckHull {
                armor: &armor,
                half,
                pose,
            }),
            _ => None,
        };
        let decision = decide(power, hull, hit, &ricochet, &mut stream);
        if let Some(h) = hull {
            let face = struck_face(half, h.pose, hit.point, hit.normal);
            let tally = faces.entry(format!("{face:?}")).or_default();
            tally.0 += 1;
            tally.1 += matches!(decision, ImpactDecision::Bounce { .. }) as u32;
        }
        decision
    };
    let mut events = Vec::new();
    for _ in 0..(10 * tick_hz) {
        if store.active().is_empty() {
            break;
        }
        events.clear();
        advance_projectiles(&mut store, &world, &[body], &mut events, &mut resolver);
        for e in &events {
            let id = e.projectile().0;
            match e {
                FlightEvent::Ricochet(r) => paths.get_mut(&id).unwrap().push(r.point),
                FlightEvent::Impact(i) => {
                    paths.get_mut(&id).unwrap().push(i.point);
                    let what = match i.struck {
                        Struck::Body(_) => "hull",
                        Struck::Terrain => "ground",
                        Struck::Prop(_) => "prop",
                    };
                    let how = if i.detonated {
                        "burst on"
                    } else {
                        "stopped by"
                    };
                    endings.insert(id, format!("{how} {what}, {} ricochets", i.bounces));
                }
                FlightEvent::Expired(x) => {
                    paths.get_mut(&id).unwrap().push(x.point);
                    endings.insert(id, format!("expired ({:?})", x.cause));
                }
                FlightEvent::NearMiss(_) | FlightEvent::Pass(_) => {}
            }
        }
    }
    println!(
        "{rounds} × {row} (penetration {}) at the tank, its front {incidence}° off the line of fire; \
         max {} ricochets, speed kept {}, penetration kept {}, scatter {}°\n",
        power.penetration,
        ricochet.max_bounces,
        ricochet.speed_kept,
        ricochet.penetration_kept,
        ricochet.scatter_deg
    );
    println!("| round | corners (x, y, z) | ending |");
    println!("|---|---|---|");
    for (id, path) in &paths {
        let corners: Vec<String> = path
            .iter()
            .map(|p| format!("({:.1}, {:.1}, {:.2})", p.x, p.y, p.z))
            .collect();
        let ending = endings.get(id).map_or("in flight", String::as_str);
        println!("| {id} | {} | {ending} |", corners.join(" → "));
    }
    println!("\n| face | hits | glanced |");
    println!("|---|---|---|");
    for (face, (hits, glanced)) in &faces {
        println!("| {face} | {hits} | {glanced} |");
    }
}
