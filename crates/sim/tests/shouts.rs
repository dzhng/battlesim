//! Barks: the simulation decides, every tick, which units shout which bark,
//! at which intensity, and which side hears it. Each test plays a small
//! battle on the test units and judges every shout against the authority's
//! own state on the tick it was spoken.
use crate::common as test_common;
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use std::collections::BTreeMap;

use contract::observation::{
    ActionReason, Intensity, Shout, ShoutSpeaker, ShoutTrigger as T, SoundCategory, SoundCue,
    SuppressionTier,
};
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, Guidance, ProjectileId};
use sim::math::V3;
use sim::units::Unit;

#[test]
fn observation_frame_exposes_shouts_without_affecting_initial_battle() {
    let setup = test_common::scenario(
        test_common::saved_map("geometry"),
        serde_json::json!([
            {"side":"blue","kind":"test_rifle","position":[100,300]},
            {"side":"red","kind":"test_rifle","position":[130,300]}
        ]),
        serde_json::json!([]),
    );
    let battle = Battle::new(&setup, 7);
    let frame = battle.observe(Side::Blue);
    assert!(frame.shouts.is_empty());
}

/// Every source file under `dir`, however deep.
fn rust_sources(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
    found
}

#[test]
fn ai_sources_do_not_read_the_shout_channel() {
    // Both deciders that read observation frames: the skirmish AI and the
    // encounter's defender. A new module in either is covered as it lands.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files: Vec<_> = ["skirmish_ai", "encounter"]
        .iter()
        .flat_map(|dir| rust_sources(&src.join(dir)))
        .collect();
    assert!(files.len() >= 2, "{files:?}");
    for file in files {
        let source = std::fs::read_to_string(&file).unwrap();
        assert!(
            !source.contains(".shouts"),
            "{} reads shouts",
            file.display()
        );
    }
}

#[test]
fn rules_without_a_positive_talk_time_are_refused() {
    let mut fixture = sim::fixtures::test_rules();
    fixture["shouts"]["talk_s"] = json!(0);
    let error = serde_json::from_value::<Rules>(fixture).expect_err("zero talk time");
    assert!(error.to_string().contains("talk_s"), "{error}");
}

#[test]
fn a_fresh_identification_emits_one_contact_bark_for_the_nearest_unit() {
    let setup = test_common::scenario(
        test_common::saved_map("geometry"),
        serde_json::json!([
            {"side":"blue","kind":"test_rifle","position":[100,300]},
            {"side":"red","kind":"test_rifle","position":[130,300]}
        ]),
        serde_json::json!([]),
    );
    let mut battle = Battle::new(&setup, 19);
    battle.step();
    assert!(battle
        .observe(Side::Blue)
        .shouts
        .iter()
        .any(|s| matches!(s.trigger, T::B1ContactInfantry)));
}

// ---------------------------------------------------------------------------
// The harness: a played battle, tick by tick.

/// One completed tick: what each side's frame carried and the authority's
/// units as they stood when it was built.
struct Moment {
    heard: [Vec<Shout>; 3],
    identified: [Vec<[f64; 3]>; 3],
    audible: [Vec<SoundCue>; 3],
    units: Vec<Unit>,
    /// The tick's flight events.
    events: Vec<FlightEvent>,
    /// Every round in flight at the end of the tick.
    rounds: BTreeMap<ProjectileId, Flying>,
}

/// A round in flight: whose it is, where it is and, if guided, where it
/// steers.
#[derive(Clone)]
struct Flying {
    side: Side,
    /// Its weapon row's id.
    weapon: String,
    position: V3,
    velocity: V3,
    guidance: Option<Guidance>,
}

struct Run {
    rules: Rules,
    /// `moments[0]` is the battle as set up, before the first step.
    moments: Vec<Moment>,
    /// The battle as it ended, for its world.
    end: Battle,
}

fn snapshot(b: &Battle) -> Moment {
    let frame = |side| b.observe(side);
    Moment {
        heard: Side::ALL.map(|s| frame(s).shouts.clone()),
        identified: Side::ALL.map(|s| frame(s).identified.iter().map(|u| u.position).collect()),
        audible: Side::ALL.map(|s| frame(s).audible.clone()),
        units: (0..).map_while(|i| b.unit(UnitId(i)).cloned()).collect(),
        events: b.flight_events().to_vec(),
        rounds: b
            .rounds()
            .map(|(p, r)| {
                let flying = Flying {
                    side: r.side,
                    weapon: b.arsenal().weapons[r.weapon].id.clone(),
                    position: p.position,
                    velocity: p.velocity,
                    guidance: p.guidance,
                };
                (p.id, flying)
            })
            .collect(),
    }
}

/// `units` on an empty field `size` metres across, on the test rules as
/// `fixture` edits them.
fn setup(size: [f64; 2], units: Value, fixture: impl FnOnce(&mut Value)) -> ScenarioDefinition {
    setup_events(size, units, json!([]), fixture)
}

/// [`setup`] with authored `events`.
fn setup_events(
    size: [f64; 2],
    units: Value,
    events: Value,
    fixture: impl FnOnce(&mut Value),
) -> ScenarioDefinition {
    let mut rules = test_common::scenario_rules();
    fixture(&mut rules);
    let map = json!({ "size": size, "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [] });
    let map = test_common::physical_map(&map.to_string());
    serde_json::from_value(json!({ "map": map, "rules": rules, "units": units,
        "events": events, "scripts": [] }))
    .unwrap()
}

/// [`brief`], and a bark may be said again on the next tick: a test sees
/// every tick its evidence holds.
fn chatty(rules: &mut Value) {
    brief(rules);
    rules["shouts"]["cooldown_s"] = json!(1.0 / rules["tick_hz"].as_f64().unwrap());
}

/// Every bark takes one tick to say, so a unit is free to speak again on the
/// next: a bark test sees each piece of evidence without the talk-time rule
/// in the way. The cooldown stays as authored.
fn brief(rules: &mut Value) {
    let hz = rules["tick_hz"].as_f64().unwrap();
    rules["shouts"]["talk_s"] = json!(1.0 / hz);
}

fn play(setup: &ScenarioDefinition, seed: u64, seconds: f64, orders: &[(Side, Order)]) -> Run {
    play_with(setup, seed, seconds, orders, |_, _| None)
}

/// [`play`], giving `order` the battle after every tick: an order it returns
/// is sent before the next.
fn play_with(
    setup: &ScenarioDefinition,
    seed: u64,
    seconds: f64,
    orders: &[(Side, Order)],
    mut order: impl FnMut(&Battle, &Moment) -> Option<(Side, Order)>,
) -> Run {
    let mut b = Battle::new(setup, seed);
    let mut seq = 0;
    for (side, order) in orders {
        seq += 1;
        test_common::order(&mut b, *side, seq, order.clone());
    }
    let mut moments = vec![snapshot(&b)];
    for _ in 0..(seconds * b.rules().tick_hz as f64) as u64 {
        b.step();
        moments.push(snapshot(&b));
        if let Some((side, next)) = order(&b, moments.last().unwrap()) {
            seq += 1;
            test_common::order(&mut b, side, seq, next);
        }
    }
    Run {
        rules: b.rules().clone(),
        moments,
        end: b,
    }
}

fn speaker(s: &Shout) -> Option<UnitId> {
    match s.speaker {
        ShoutSpeaker::Own(id) => Some(id),
        ShoutSpeaker::Seen(_) => None,
    }
}

impl Run {
    fn unit(&self, tick: usize, id: u32) -> &Unit {
        &self.moments[tick].units[id as usize]
    }

    /// The ticks `id` said `trigger` on, as its own side heard it.
    fn said(&self, id: u32, trigger: T) -> Vec<usize> {
        let side = self.unit(0, id).side.index();
        (0..self.moments.len())
            .filter(|&t| {
                self.moments[t].heard[side]
                    .iter()
                    .any(|s| s.trigger == trigger && speaker(s) == Some(UnitId(id)))
            })
            .collect()
    }

    /// Everything `id` said on `tick`, as its own side heard it.
    fn spoke(&self, tick: usize, id: u32) -> Vec<&Shout> {
        let side = self.unit(0, id).side.index();
        self.moments[tick].heard[side]
            .iter()
            .filter(|s| speaker(s) == Some(UnitId(id)))
            .collect()
    }

    fn ticks(&self, test: impl Fn(&Unit, &Unit) -> bool, id: u32) -> Vec<usize> {
        (1..self.moments.len())
            .filter(|&t| test(self.unit(t - 1, id), self.unit(t, id)))
            .collect()
    }

    fn talk_ticks(&self) -> u64 {
        (self.rules.shouts.talk_s * self.rules.tick_hz as f64).round() as u64
    }

    fn cooldown_ticks(&self) -> u64 {
        (self.rules.shouts.cooldown_s * self.rules.tick_hz as f64).round() as u64
    }
}

/// Health as the bark rules read it: a vehicle's share of its hull's hp; a
/// squad's living members over its starting members.
fn health(u: &Unit, rules: &Rules) -> f64 {
    if u.members.is_empty() {
        u.hp / u.max_hp(rules)
    } else {
        u.members.iter().filter(|m| m.alive()).count() as f64 / u.members.len() as f64
    }
}

fn dead_members(u: &Unit) -> usize {
    u.members.iter().filter(|m| !m.alive()).count()
}

fn rank(tier: SuppressionTier) -> u8 {
    match tier {
        SuppressionTier::None => 0,
        SuppressionTier::Suppressed => 1,
        SuppressionTier::Pinned => 2,
    }
}

fn entered(reason: ActionReason) -> impl Fn(&Unit, &Unit) -> bool {
    move |old, now| {
        now.mounts
            .iter()
            .zip(&old.mounts)
            .any(|(n, o)| n.reason == reason && o.reason != reason)
    }
}

fn launched(old: &Unit, now: &Unit) -> bool {
    now.mounts
        .iter()
        .zip(&old.mounts)
        .any(|(n, o)| n.shots != o.shots)
}

// ---------------------------------------------------------------------------
// B1–B3: contact, once, as an enemy comes into view.

/// A blue rifle squad and one red `enemy` 300 m off, both holding fire, for
/// 20 s: each sees the other the whole time.
fn standoff(enemy: &str) -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": enemy, "position": [400, 300], "engagement": "return_fire_only" },
        ]),
        |_| {},
    );
    play(&s, 3, 20.0, &[])
}

#[test]
fn contact_infantry_is_called_once_as_an_enemy_squad_comes_into_view() {
    let run = standoff("test_rifle");
    assert_eq!(
        run.said(0, T::B1ContactInfantry).len(),
        1,
        "blue calls the squad it keeps seeing once, not again each cooldown"
    );
    assert_eq!(run.said(1, T::B1ContactInfantry).len(), 1);
}

#[test]
fn contact_vehicle_is_called_for_an_enemy_hull() {
    let run = standoff("test_tank");
    assert_eq!(run.said(0, T::B2ContactVehicle).len(), 1);
    assert!(run.said(0, T::B1ContactInfantry).is_empty());
    assert!(run.said(0, T::B3ContactAir).is_empty());
}

#[test]
fn contact_air_is_called_for_an_enemy_in_the_air() {
    let run = standoff("test_heli");
    assert!(run.unit(1, 1).airborne(), "the helicopter flies");
    assert_eq!(
        run.said(0, T::B3ContactAir).len(),
        1,
        "a flying hull is aircraft, not vehicle: {:?}",
        run.said(0, T::B2ContactVehicle)
    );
    assert!(run.said(0, T::B2ContactVehicle).is_empty());
}

// ---------------------------------------------------------------------------
// B4, B12: a rifle squad and a tank fire on a red squad that cannot die.

fn firing_line() -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "blue", "kind": "test_tank", "position": [100, 360] },
            { "side": "red", "kind": "test_rifle", "position": [350, 330],
              "engagement": "return_fire_only", "condition": { "soldier_hp": 1.0e6 } },
        ]),
        brief,
    );
    play(&s, 5, 40.0, &[])
}

#[test]
fn opens_fire_is_called_on_a_new_target_not_on_every_burst() {
    let run = firing_line();
    for id in [0, 1] {
        let fired = run.ticks(launched, id);
        assert!(fired.len() > 3, "unit {id} keeps firing: {fired:?}");
        let said = run.said(id, T::B4OpensFire);
        assert_eq!(
            said.len(),
            1,
            "unit {id} fires 40 s on one target and opens fire once: {said:?}"
        );
        assert!(fired.contains(&said[0]), "said on a tick it fired");
    }
}

#[test]
fn reloading_is_called_by_infantry_and_never_by_a_vehicle() {
    // A blue AT team (whose launcher reloads after each missile) and a blue
    // tank (whose cannon reloads after each round) engage a red tank and a
    // red squad that cannot die.
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300],
              "condition": { "soldier_hp": 1.0e6 } },
            { "side": "blue", "kind": "test_tank", "position": [100, 360] },
            { "side": "red", "kind": "test_rifle", "position": [400, 330],
              "engagement": "return_fire_only", "condition": { "soldier_hp": 1.0e6 } },
            { "side": "red", "kind": "test_tank", "position": [450, 300], "engagement": "return_fire_only" },
        ]),
        brief,
    );
    let run = play(&s, 5, 40.0, &[]);
    let reloads = run.ticks(entered(ActionReason::Reloading), 0);
    let said = run.said(0, T::B12Reloading);
    assert!(
        !said.is_empty(),
        "the AT team calls its reload: {reloads:?}"
    );
    assert!(
        said.iter().all(|t| reloads.contains(t)),
        "{said:?} / {reloads:?}"
    );
    assert!(
        run.ticks(entered(ActionReason::Reloading), 1).len() > 1,
        "the tank reloads too"
    );
    assert!(run.said(1, T::B12Reloading).is_empty(), "but never says so");
}

// ---------------------------------------------------------------------------
// B6, B7: sustained near misses climb a squad's tiers.

fn near_misses() -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "red", "kind": "test_rifle", "position": [200, 303], "engagement": "return_fire_only" },
        ]),
        |rules| {
            brief(rules);
            // Its grenades cut off short of the tank's armour: the squad only
            // takes the near misses.
            rules["weapons"]["grenade"]["partial_from"] = json!(0.6);
        },
    );
    let ground = Order::Attack {
        units: vec![UnitId(0)],
        target: TargetRef::Ground {
            point: [320.0, 300.0, 0.0],
        },
    };
    play(&s, 4, 15.0, &[(Side::Blue, ground)])
}

fn tier_rises_to(run: &Run, tier: SuppressionTier) -> Vec<usize> {
    let rules = &run.rules.suppression;
    run.ticks(
        |o, n| {
            let (o, n) = (rules.tier(o.suppression), rules.tier(n.suppression));
            n == tier && rank(n) > rank(o)
        },
        1,
    )
}

#[test]
fn suppressed_is_called_as_the_tier_rises_not_at_each_near_miss() {
    let run = near_misses();
    let rises = tier_rises_to(&run, SuppressionTier::Suppressed);
    assert_eq!(rises.len(), 1, "the squad becomes suppressed once");
    assert_eq!(run.said(1, T::B6Suppressed), rises);
}

#[test]
fn pinned_is_called_as_the_tier_rises_to_pinned() {
    let run = near_misses();
    let rises = tier_rises_to(&run, SuppressionTier::Pinned);
    assert_eq!(rises.len(), 1, "the squad is pinned once");
    assert_eq!(run.said(1, T::B7Pinned), rises);
}

// ---------------------------------------------------------------------------
// B8, B9, B10: damage the unit survives.

#[test]
fn hit_is_called_when_a_soldier_is_wounded_and_the_squad_lives() {
    let run = firing_line();
    let wounded = run.ticks(
        |o, n| n.alive() && n.members.iter().zip(&o.members).any(|(n, o)| n.hp < o.hp),
        2,
    );
    assert!(!wounded.is_empty(), "red's soldiers are hit");
    let said = run.said(2, T::B8Hit);
    assert!(!said.is_empty(), "a wounded squad says so");
    assert!(said.iter().all(|t| wounded.contains(t)), "{said:?}");
}

/// Two rifle squads in the open, 150 m apart, both firing: men fall.
fn rifle_duel(red_condition: Value) -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "red", "kind": "test_rifle", "position": [250, 300], "condition": red_condition },
        ]),
        brief,
    );
    play(&s, 9, 60.0, &[])
}

#[test]
fn man_down_is_called_when_a_soldier_dies_and_the_squad_lives() {
    let run = rifle_duel(json!({}));
    let mut called = 0;
    for id in [0, 1] {
        let deaths = run.ticks(|o, n| n.alive() && dead_members(n) > dead_members(o), id);
        let said = run.said(id, T::B9ManDown);
        assert!(
            said.iter().all(|t| deaths.contains(t)),
            "unit {id}: {said:?} / {deaths:?}"
        );
        called += said.len();
    }
    assert!(called > 0, "someone calls a man down");
}

#[test]
fn badly_damaged_is_called_as_a_vehicles_hp_crosses_a_third() {
    // Blue's HMG jeep works on a red truck that starts just above a third.
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_jeep", "position": [100, 300] },
            { "side": "red", "kind": "test_supply", "position": [300, 300],
              "engagement": "return_fire_only", "condition": { "hp": 24 } },
        ]),
        brief,
    );
    let run = play(&s, 2, 40.0, &[]);
    let rules = &run.rules;
    let crossed = run.ticks(
        |o, n| n.alive() && health(o, rules) > 1.0 / 3.0 && health(n, rules) <= 1.0 / 3.0,
        1,
    );
    assert_eq!(crossed.len(), 1, "the truck is worn past a third and lives");
    assert_eq!(run.said(1, T::B10BadlyDamaged), crossed);
}

#[test]
fn ties_within_a_tick_go_to_the_more_specific_bark() {
    // Red has three of eight left; the next to fall leaves a third or less:
    // a man down, a hit and badly damaged all at once. Badly damaged speaks.
    let run = rifle_duel(json!({ "casualties": 5 }));
    let rules = &run.rules;
    let crossed = run.ticks(
        |o, n| n.alive() && health(o, rules) > 1.0 / 3.0 && health(n, rules) <= 1.0 / 3.0,
        1,
    );
    assert_eq!(crossed.len(), 1, "red falls past a third and lives");
    let t = crossed[0];
    let said: Vec<T> = run.spoke(t, 1).iter().map(|s| s.trigger).collect();
    assert_eq!(said, [T::B10BadlyDamaged]);
    assert_eq!(run.said(1, T::B10BadlyDamaged), crossed);
}

// ---------------------------------------------------------------------------
// B11, B16: a tank kills a jeep.

fn tank_kills_jeep() -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "red", "kind": "test_jeep", "position": [400, 300], "engagement": "return_fire_only" },
        ]),
        brief,
    );
    play(&s, 6, 30.0, &[])
}

fn death(run: &Run, id: u32) -> usize {
    let died = run.ticks(|o, n| o.alive() && !n.alive(), id);
    assert_eq!(died.len(), 1, "unit {id} dies once");
    died[0]
}

#[test]
fn kill_is_called_by_the_unit_whose_round_destroyed_an_enemy() {
    let run = tank_kills_jeep();
    let died = death(&run, 1);
    assert_eq!(run.said(0, T::B11Kill), [died]);
}

#[test]
fn destroyed_is_called_once_from_the_dead_units_last_kind_and_position() {
    let run = tank_kills_jeep();
    let died = death(&run, 1);
    let spoke = run.spoke(died, 1);
    assert_eq!(
        spoke.len(),
        1,
        "the jeep says one thing as it dies: {spoke:?}"
    );
    let jeep = run.unit(died, 1);
    assert_eq!(spoke[0].trigger, T::B16Destroyed);
    assert_eq!(spoke[0].kind, jeep.kind);
    assert_eq!(
        spoke[0].position,
        [jeep.position.x, jeep.position.y, jeep.position.z]
    );
    assert_eq!(
        run.said(1, T::B16Destroyed),
        [died],
        "a wreck says nothing more"
    );
    assert!((died + 1..run.moments.len()).all(|t| run.spoke(t, 1).is_empty()));
}

// ---------------------------------------------------------------------------
// B13, B14: a guided launcher with one missile left fires at a tank.

fn last_missile() -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300],
              "condition": { "spent": { "atgm": 3 }, "soldier_hp": 1.0e6 } },
            { "side": "red", "kind": "test_tank", "position": [450, 300], "engagement": "return_fire_only" },
        ]),
        brief,
    );
    play(&s, 8, 40.0, &[])
}

#[test]
fn missile_away_is_called_on_a_guided_launch() {
    let run = last_missile();
    let guided = run.ticks(
        |o, n| {
            n.mounts
                .iter()
                .zip(&o.mounts)
                .any(|(n, o)| n.support.is_some() && n.shots != o.shots)
        },
        0,
    );
    assert_eq!(guided.len(), 1, "one missile, one launch");
    assert_eq!(run.said(0, T::B14MissileAway), guided);
}

#[test]
fn out_of_ammo_is_called_once_when_the_last_round_is_gone() {
    let run = last_missile();
    let out = run.ticks(entered(ActionReason::OutOfAmmo), 0);
    assert_eq!(out.len(), 1, "the launcher runs dry once");
    assert_eq!(
        run.said(0, T::B13OutOfAmmo),
        out,
        "called as it runs dry, not again each cooldown"
    );
}

// ---------------------------------------------------------------------------
// Ledger rules, on the authored talk times.

/// Two rifle squads that cannot die trade fire for 40 s.
fn long_firefight() -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300], "condition": { "soldier_hp": 1.0e6 } },
            { "side": "red", "kind": "test_rifle", "position": [300, 300], "condition": { "soldier_hp": 1.0e6 } },
        ]),
        |_| {},
    );
    play(&s, 11, 40.0, &[])
}

/// Every bark `id` spoke, in order: (tick, trigger).
fn barks(run: &Run, id: u32) -> Vec<(usize, T)> {
    (0..run.moments.len())
        .flat_map(|t| run.spoke(t, id).into_iter().map(move |s| (t, s.trigger)))
        .collect()
}

#[test]
fn the_same_bark_is_not_repeated_within_the_cooldown() {
    let run = long_firefight();
    let mut repeats = 0;
    for id in [0, 1] {
        let barks = barks(&run, id);
        for (i, &(t, trigger)) in barks.iter().enumerate() {
            if let Some(&(prev, _)) = barks[..i].iter().rev().find(|(_, k)| *k == trigger) {
                repeats += 1;
                assert!(
                    (t - prev) as u64 >= run.cooldown_ticks(),
                    "unit {id} said {trigger:?} at {prev} and again at {t}"
                );
            }
        }
    }
    assert!(repeats > 0, "a long fight repeats some bark");
}

#[test]
fn a_unit_never_starts_a_bark_while_it_is_still_talking() {
    let run = long_firefight();
    let mut said = 0;
    for id in [0, 1] {
        let barks = barks(&run, id);
        said += barks.len();
        for pair in barks.windows(2) {
            let ((a, k), (b, _)) = (pair[0], pair[1]);
            assert!(
                (b - a) as u64 >= run.talk_ticks(),
                "unit {id} began {:?} at {b} while saying {k:?} from {a}",
                pair[1].1
            );
        }
    }
    assert!(said > 2, "the squads talk");
}

// ---------------------------------------------------------------------------
// Who hears.

#[test]
fn an_enemy_is_heard_exactly_while_it_is_identified() {
    // A red recon team sees a blue squad that cannot see back; far off, two
    // rifle squads fight in plain view of each other.
    let s = setup(
        [2000.0, 2000.0],
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [725, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_rifle", "position": [500, 1700] },
            { "side": "red", "kind": "test_rifle", "position": [700, 1700] },
        ]),
        brief,
    );
    let run = play(&s, 12, 30.0, &[]);
    let (mut heard, mut unheard) = (0, 0);
    for m in &run.moments {
        for listener in Side::COMBATANTS {
            let l = listener.index();
            let other = if listener == Side::Blue {
                Side::Red
            } else {
                Side::Blue
            };
            let mut expected: Vec<(T, [f64; 3], Intensity)> = m.heard[other.index()]
                .iter()
                .filter(|s| speaker(s).is_some())
                .filter(|s| {
                    let known = m.identified[l].contains(&s.position);
                    if known {
                        heard += 1;
                    } else {
                        unheard += 1;
                    }
                    known
                })
                .map(|s| (s.trigger, s.position, s.intensity))
                .collect();
            let mut got: Vec<(T, [f64; 3], Intensity)> = m.heard[l]
                .iter()
                .filter(|s| matches!(s.speaker, ShoutSpeaker::Seen(_)))
                .map(|s| (s.trigger, s.position, s.intensity))
                .collect();
            let key = |a: &(T, [f64; 3], Intensity)| (a.0, a.1.map(f64::to_bits));
            expected.sort_by_key(key);
            got.sort_by_key(key);
            assert_eq!(
                got, expected,
                "{listener:?} hears exactly the enemies it identifies"
            );
        }
    }
    assert!(heard > 0, "some enemy barks are heard");
    assert!(unheard > 0, "and some are spoken unseen");
}

// ---------------------------------------------------------------------------
// Intensity.

#[test]
fn a_bark_is_stressed_at_half_health_or_under_suppression() {
    // Each blue unit faces a red tank alone and calls contact on the first
    // tick; how it says it depends only on its own state.
    let cases = [
        ("test_rifle", json!({}), Intensity::Steady),
        (
            "test_rifle",
            json!({ "suppression": 0.2 }),
            Intensity::Steady,
        ),
        (
            "test_rifle",
            json!({ "suppression": 0.4 }),
            Intensity::Stressed,
        ),
        ("test_rifle", json!({ "casualties": 3 }), Intensity::Steady),
        (
            "test_rifle",
            json!({ "casualties": 4 }),
            Intensity::Stressed,
        ),
        ("test_tank", json!({ "hp": 51 }), Intensity::Steady),
        ("test_tank", json!({ "hp": 50 }), Intensity::Stressed),
    ];
    let mut units = Vec::new();
    for (i, (kind, condition, _)) in cases.iter().enumerate() {
        let x = 200 + 2000 * i;
        units.push(json!({ "side": "blue", "kind": kind, "position": [x, 300],
            "engagement": "return_fire_only", "condition": condition }));
        units.push(
            json!({ "side": "red", "kind": "test_tank", "position": [x + 300, 300],
            "engagement": "return_fire_only" }),
        );
    }
    let s = setup(
        [2000.0 * cases.len() as f64, 600.0],
        Value::Array(units),
        |_| {},
    );
    let run = play(&s, 1, 1.0, &[]);
    let rules = &run.rules;
    for (i, (kind, condition, intensity)) in cases.iter().enumerate() {
        let id = 2 * i as u32;
        let said = run.said(id, T::B2ContactVehicle);
        assert_eq!(said.len(), 1, "{kind} {condition} calls contact");
        let shout = run.spoke(said[0], id)[0];
        let unit = run.unit(said[0], id);
        assert_eq!(
            shout.intensity,
            *intensity,
            "{kind} {condition}: health {}, tier {:?}",
            health(unit, rules),
            rules.suppression.tier(unit.suppression)
        );
    }
}

// ---------------------------------------------------------------------------
// B5: under fire, from the rounds a unit hears.

impl Run {
    /// The ticks a round of `side`'s passed `id` close enough to be heard
    /// as a near miss, within its weapon's suppression radius, from a
    /// weapon row `weapon` accepts.
    fn near_misses(&self, id: u32, side: Side, weapon: impl Fn(&str) -> bool) -> Vec<usize> {
        (1..self.moments.len())
            .filter(|&t| {
                self.moments[t].events.iter().any(|e| match e {
                    FlightEvent::NearMiss(n) => {
                        n.unit == UnitId(id)
                            && self.moments[t - 1]
                                .rounds
                                .get(&n.projectile)
                                .is_some_and(|r| r.side == side && weapon(&r.weapon))
                    }
                    _ => false,
                })
            })
            .collect()
    }

    /// Whether unit `id` says `trigger` on `tick`, or something that speaks
    /// before it: a contact call, or a bark earlier in the tie-break order.
    fn calls(&self, tick: usize, id: u32, trigger: T) -> bool {
        let rank = |k: &T| sim::shouts::PRIORITY.iter().position(|p| p == k);
        let contact = |k: &T| {
            matches!(
                k,
                T::B1ContactInfantry | T::B2ContactVehicle | T::B3ContactAir
            )
        };
        let spoke: Vec<T> = self.spoke(tick, id).iter().map(|s| s.trigger).collect();
        spoke == [trigger]
            || spoke
                .iter()
                .any(|k| contact(k) || rank(k).unwrap() < rank(&trigger).unwrap())
    }

    /// Weapon row `weapon`'s suppression radius.
    fn radius(&self, weapon: &str) -> f64 {
        let row = self
            .end
            .arsenal()
            .weapons
            .iter()
            .find(|w| w.id == weapon)
            .unwrap();
        row.def.ballistics.suppression_radius_m
    }

    fn tier(&self, tick: usize, id: u32) -> SuppressionTier {
        self.rules.suppression.tier(self.unit(tick, id).suppression)
    }
}

/// `side`'s attack on the enemy standing at `at`, once its frame
/// identifies it.
fn attack_when_seen(
    side: Side,
    units: Vec<UnitId>,
    at: [f64; 2],
) -> impl FnMut(&Battle, &Moment) -> Option<(Side, Order)> {
    let mut sent = false;
    move |b, _| {
        if sent {
            return None;
        }
        let target =
            b.observe(side).identified.iter().find(|u| {
                (u.position[0] - at[0]).abs() < 1.0 && (u.position[1] - at[1]).abs() < 1.0
            })?;
        sent = true;
        let target = TargetRef::Identified { id: target.id };
        Some((
            side,
            Order::Attack {
                units: units.clone(),
                target,
            },
        ))
    }
}

/// Blue's tank duels a red tank 320 m off; its sabots fly past `bystander`,
/// 6 m beside the line of fire 160 m out.
fn sabots_past(bystander: Value) -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [420, 300], "engagement": "return_fire_only" },
            bystander,
        ]),
        chatty,
    );
    play_with(
        &s,
        4,
        20.0,
        &[],
        attack_when_seen(Side::Blue, vec![UnitId(0)], [420.0, 300.0]),
    )
}

#[test]
fn hulls_hear_near_misses_and_a_sabot_passing_a_tank_is_under_fire() {
    let run = sabots_past(
        json!({ "side": "red", "kind": "test_tank", "position": [260, 306],
        "engagement": "return_fire_only" }),
    );
    let passes = run.near_misses(2, Side::Blue, |w| w == "tank_ap");
    assert!(
        !passes.is_empty(),
        "flight reports a sabot's near miss on a hull"
    );
    let said = run.said(2, T::B5UnderFire);
    for t in &passes {
        let spoke: Vec<T> = run.spoke(*t, 2).iter().map(|s| s.trigger).collect();
        assert_eq!(
            spoke,
            [T::B5UnderFire],
            "a sabot passes on tick {t}: {said:?}"
        );
    }
}

#[test]
fn own_side_rounds_passing_close_are_not_taking_fire() {
    // Blue's tank shells a red squad that cannot die and cannot reach back;
    // a blue tank stands beside the line of fire.
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [420, 300], "engagement": "return_fire_only",
              "condition": { "soldier_hp": 1.0e6 } },
            { "side": "blue", "kind": "test_tank", "position": [260, 306], "engagement": "return_fire_only" },
        ]),
        chatty,
    );
    let run = play_with(
        &s,
        4,
        20.0,
        &[],
        attack_when_seen(Side::Blue, vec![UnitId(0)], [420.0, 300.0]),
    );
    assert!(
        !run.near_misses(2, Side::Blue, |_| true).is_empty(),
        "blue's rounds pass its own tank"
    );
    assert!(run.near_misses(2, Side::Red, |_| true).is_empty());
    assert!(
        run.said(2, T::B5UnderFire).is_empty(),
        "check fire, not taking fire"
    );
}

#[test]
fn a_units_own_shells_landing_close_are_not_taking_fire() {
    // A lone tank, its sabots spent, shells the ground 13.5 m before its
    // hull: its own bursts land within their suppression radius of it, just
    // beyond their blast.
    let s = setup(
        [1200.0, 600.0],
        json!([{ "side": "blue", "kind": "test_tank", "position": [100, 300],
                 "condition": { "spent": { "tank_ap": 20 } } }]),
        chatty,
    );
    let ground = Order::Attack {
        units: vec![UnitId(0)],
        target: TargetRef::Ground {
            point: [117.0, 300.0, 0.0],
        },
    };
    let run = play(&s, 4, 30.0, &[(Side::Blue, ground)]);
    let close: Vec<usize> = (1..run.moments.len())
        .filter(|&t| {
            run.moments[t].events.iter().any(|e| match e {
                FlightEvent::Impact(i) => run.moments[t - 1]
                    .rounds
                    .get(&i.projectile)
                    .is_some_and(|r| run.unit(t, 0).hull_distance(i.point) < run.radius(&r.weapon)),
                _ => false,
            })
        })
        .collect();
    assert!(!close.is_empty(), "its own shells burst close by");
    assert!(run.said(0, T::B5UnderFire).is_empty());
}

#[test]
fn a_suppressed_or_pinned_squad_leaves_taking_fire_to_its_suppression() {
    // A blue tank works over a red squad 100 m off for 15 s: the squad is
    // suppressed, then pinned, under steady near misses.
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "red", "kind": "test_rifle", "position": [200, 303], "engagement": "return_fire_only",
              "condition": { "soldier_hp": 1.0e6 } },
        ]),
        chatty,
    );
    let run = play(&s, 4, 15.0, &[]);
    let said = run.said(1, T::B5UnderFire);
    assert!(
        !said.is_empty(),
        "the squad calls taking fire before it is suppressed"
    );
    for &t in &said {
        assert_eq!(run.tier(t, 1), SuppressionTier::None, "tick {t}");
    }
    let suppressed_and_missed: Vec<usize> = run
        .near_misses(1, Side::Blue, |_| true)
        .into_iter()
        .filter(|&t| run.tier(t, 1) != SuppressionTier::None)
        .collect();
    assert!(
        suppressed_and_missed.len() > 10,
        "rounds keep passing the squad once it is suppressed: {suppressed_and_missed:?}"
    );
}

/// A blue rifle squad that cannot die opens fire on a red tank 200 m off
/// that faces `yaw` (0 faces away from the squad).
fn rifles_on_tank(yaw: f64) -> Run {
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300], "condition": { "soldier_hp": 1.0e6 } },
            { "side": "red", "kind": "test_tank", "position": [300, 300], "yaw": yaw,
              "engagement": "return_fire_only" },
        ]),
        chatty,
    );
    play(&s, 6, 20.0, &[])
}

/// The ticks a rifle round glanced off unit `id`'s hull, and those it
/// struck it.
fn pings(run: &Run, id: u32) -> (Vec<usize>, Vec<usize>) {
    let body = sim::flight::BodyId(sim::weapons::VEHICLE_BODY_BASE + id);
    let (mut glanced, mut struck) = (Vec::new(), Vec::new());
    for t in 1..run.moments.len() {
        let rifle = |p: &ProjectileId| {
            run.moments[t - 1]
                .rounds
                .get(p)
                .is_some_and(|r| r.weapon == "rifle")
        };
        for e in &run.moments[t].events {
            match e {
                FlightEvent::Ricochet(r) if r.body == body && rifle(&r.projectile) => {
                    glanced.push(t)
                }
                FlightEvent::Impact(i)
                    if i.struck == sim::flight::Struck::Body(body) && rifle(&i.projectile) =>
                {
                    struck.push(t)
                }
                _ => {}
            }
        }
    }
    (glanced, struck)
}

#[test]
fn a_tank_pinged_by_rifle_fire_calls_taking_fire() {
    let run = rifles_on_tank(std::f64::consts::PI);
    let (glanced, struck) = pings(&run, 1);
    assert!(!glanced.is_empty(), "rifle rounds glance off the tank");
    let mut called = 0;
    for t in glanced.into_iter().chain(struck) {
        assert!(run.calls(t, 1, T::B5UnderFire), "pinged on tick {t}");
        called += usize::from(run.said(1, T::B5UnderFire).contains(&t));
    }
    assert!(called > 0, "the tank says it");
}

#[test]
fn taking_fire_is_called_though_the_shooter_is_unseen() {
    // The tank faces away; the squad behind it is beyond its rear sight.
    let run = rifles_on_tank(0.0);
    let said = run.said(1, T::B5UnderFire);
    let unseen: Vec<usize> = said
        .iter()
        .copied()
        .filter(|&t| run.moments[t].identified[Side::Red.index()].is_empty())
        .collect();
    assert!(
        !unseen.is_empty(),
        "the tank calls taking fire before it sees who fires: {said:?}"
    );
}

// ---------------------------------------------------------------------------
// B15: incoming missile, for the unit it steers onto, once seen.

impl Run {
    /// The guided rounds in flight at the end of `tick`.
    fn missiles(&self, tick: usize) -> impl Iterator<Item = &Flying> {
        self.moments[tick]
            .rounds
            .values()
            .filter(|r| r.guidance.is_some())
    }

    /// Whether unit `id` sees the point `at` on `tick`, by its own sight.
    fn sees(&self, tick: usize, id: u32, at: V3) -> bool {
        let unit = self.unit(tick, id);
        sim::sensing::sees_point(
            self.end.world(),
            sim::sensing::eye(unit, &self.rules),
            at,
            &sim::sight::of(unit, &self.rules),
            1.0,
            &self.rules.sensors,
        )
    }

    /// The unit blue's launcher `id` guides its missile at on `tick`.
    fn guided_at(&self, tick: usize, id: u32) -> Option<UnitId> {
        self.unit(tick, id)
            .mounts
            .iter()
            .find_map(|m| match m.support?.target {
                sim::weapons::Target::Unit(t) => Some(t),
                _ => None,
            })
    }
}

/// Whether `point` lies on hull `unit` seen from above.
fn on_hull(unit: &Unit, point: V3) -> bool {
    let half = unit.hull.unwrap();
    let d = point.xy() - unit.position.xy();
    let (s, c) = (unit.yaw.sin(), unit.yaw.cos());
    let local = [d.x * c + d.y * s, -d.x * s + d.y * c];
    local[0].abs() <= half.x && local[1].abs() <= half.y
}

/// A blue AT team that cannot die and `red` units, 350 m off. It fires one
/// missile: its launcher reloads for longer than the run.
fn missile_at(red: Value, seconds: f64) -> Run {
    let mut units = vec![
        json!({ "side": "blue", "kind": "test_at", "position": [100, 300],
        "condition": { "soldier_hp": 1.0e6 } }),
    ];
    units.extend(red.as_array().unwrap().iter().cloned());
    let s = setup([1200.0, 600.0], Value::Array(units), |rules| {
        chatty(rules);
        rules["weapons"]["atgm"]["reload_s"] = json!(seconds + 1.0);
    });
    play(&s, 8, seconds, &[])
}

#[test]
fn a_missile_aimed_at_the_neighbouring_tank_is_called_by_that_tank_only() {
    let run = missile_at(
        json!([
            { "side": "red", "kind": "test_tank", "position": [450, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [450, 312], "engagement": "return_fire_only" },
        ]),
        20.0,
    );
    let target = (1..run.moments.len())
        .find_map(|t| run.guided_at(t, 0))
        .expect("the launcher guides a missile");
    let neighbour = if target == UnitId(1) { 2 } else { 1 };
    assert!(
        !run.said(target.0, T::B15IncomingMissile).is_empty(),
        "the tank it steers onto calls it"
    );
    assert!(
        run.said(neighbour, T::B15IncomingMissile).is_empty(),
        "the tank beside it does not"
    );
}

#[test]
fn an_incoming_missile_is_silent_until_the_tank_sees_it() {
    // The tank faces away; the missile comes from beyond its rear sight.
    let run = missile_at(
        json!([{ "side": "red", "kind": "test_tank", "position": [450, 300], "yaw": 0.0,
                 "engagement": "return_fire_only" }]),
        20.0,
    );
    let (mut unseen, mut seen) = (0, 0);
    for t in 1..run.moments.len() {
        let tank = run.unit(t, 1);
        if !tank.alive() {
            break;
        }
        let aimed: Vec<V3> = run
            .missiles(t)
            .filter(|m| m.side == Side::Blue && on_hull(tank, m.guidance.unwrap().point))
            .map(|m| m.position)
            .collect();
        let sees = aimed.iter().any(|&at| run.sees(t, 1, at));
        let spoke: Vec<T> = run.spoke(t, 1).iter().map(|s| s.trigger).collect();
        if !aimed.is_empty() && !sees {
            unseen += 1;
        }
        if sees {
            seen += 1;
            assert!(
                run.calls(t, 1, T::B15IncomingMissile),
                "tick {t}: {spoke:?}"
            );
        } else {
            assert!(!spoke.contains(&T::B15IncomingMissile), "tick {t}: unseen");
        }
    }
    assert!(
        unseen > 0,
        "the missile flies a while out of the tank's sight"
    );
    assert!(seen > 0, "and then into it");
    assert!(!run.said(1, T::B15IncomingMissile).is_empty());
}

#[test]
fn a_sides_own_missile_is_never_incoming() {
    // A blue squad stands beside the red tank its own launcher fires at: the
    // missile steers onto the squad's area too.
    let run = missile_at(
        json!([
            { "side": "red", "kind": "test_tank", "position": [450, 300], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_rifle", "position": [450, 304], "engagement": "return_fire_only",
              "condition": { "soldier_hp": 1.0e6 } },
        ]),
        20.0,
    );
    let r = run.rules.physics.soldier_radius_m;
    let overhead = (1..run.moments.len()).any(|t| {
        let squad = run.unit(t, 2);
        run.missiles(t).any(|m| {
            (m.guidance.unwrap().point.xy() - squad.position.xy()).length()
                <= squad.footprint_radius(r)
                && run.sees(t, 2, m.position)
        })
    });
    assert!(
        overhead,
        "the squad sees its side's missile steer onto its area"
    );
    assert!(
        !run.said(1, T::B15IncomingMissile).is_empty(),
        "the red tank calls it"
    );
    assert!(
        run.said(2, T::B15IncomingMissile).is_empty(),
        "the blue squad does not"
    );
    assert!(
        run.said(0, T::B15IncomingMissile).is_empty(),
        "nor does the launcher"
    );
}

#[test]
fn a_released_missile_whose_fixed_point_lies_on_a_tank_is_still_incoming() {
    // The AT team walks off as its missile closes on the tank: the missile
    // is let go, and flies on at a point fixed a coast ahead, on the tank.
    let s = setup(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300], "condition": { "soldier_hp": 1.0e6 } },
            { "side": "red", "kind": "test_tank", "position": [450, 300], "engagement": "return_fire_only" },
        ]),
        chatty,
    );
    let coast = test_common::game()["guided"]["release_coast_s"]
        .as_f64()
        .unwrap();
    let mut walked = false;
    let run = play_with(&s, 8, 20.0, &[], |b, m| {
        let tank = b.unit(UnitId(1))?.position;
        let missile = m
            .rounds
            .values()
            .find(|r| r.guidance.is_some_and(|g| g.supported))?;
        // Walk off once a coast's flight would reach the tank: the release,
        // on the next tick, fixes the point there.
        let coast_m = (missile.velocity * coast).xy().length();
        if walked || coast_m < (tank.xy() - missile.position.xy()).length() {
            return None;
        }
        walked = true;
        let walk = Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [100.0, 200.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        };
        Some((Side::Blue, walk))
    });
    let released: Vec<usize> = (1..run.moments.len())
        .filter(|&t| {
            let tank = run.unit(t, 1);
            tank.alive()
                && run.missiles(t).any(|m| {
                    let g = m.guidance.unwrap();
                    !g.supported && on_hull(tank, g.point) && run.sees(t, 1, m.position)
                })
        })
        .collect();
    assert!(
        !released.is_empty(),
        "the tank sees a released missile fly on at a point on it"
    );
    for &t in &released {
        assert!(run.calls(t, 1, T::B15IncomingMissile), "tick {t}");
    }
}

// ---------------------------------------------------------------------------
// B17: friendly destroyed nearby, called by one witness.

/// Blue's tank kills a red jeep 300 m off; `red` friendlies look on (or
/// don't) from where they stand.
fn jeep_lost(red: Value) -> Run {
    let mut units = vec![
        json!({ "side": "blue", "kind": "test_tank", "position": [100, 300] }),
        json!({ "side": "red", "kind": "test_jeep", "position": [400, 300], "engagement": "return_fire_only" }),
    ];
    units.extend(red.as_array().unwrap().iter().cloned());
    let s = setup([2000.0, 2000.0], Value::Array(units), brief);
    play(&s, 6, 30.0, &[])
}

/// Every B17 spoken in the run: (tick, speaker).
fn losses_called(run: &Run) -> Vec<(usize, u32)> {
    (0..run.moments.len())
        .flat_map(|t| {
            run.moments[t]
                .heard
                .iter()
                .flatten()
                .filter(|s| s.trigger == T::B17FriendlyDestroyedNearby)
                .filter_map(move |s| speaker(s).map(|id| (t, id.0)))
        })
        .collect()
}

#[test]
fn a_loss_nobody_sees_is_not_called() {
    // The only red friendly stands 1.3 km off, beyond its own sight.
    let run = jeep_lost(json!([
        { "side": "red", "kind": "test_rifle", "position": [400, 1600], "engagement": "return_fire_only" },
    ]));
    death(&run, 1);
    assert!(losses_called(&run).is_empty());
}

#[test]
fn of_three_friendlies_watching_only_the_nearest_calls_the_loss() {
    // Three red squads 150, 200 and 250 m behind the jeep, out of the tank's
    // sight, all see it go.
    let run = jeep_lost(json!([
        { "side": "red", "kind": "test_rifle", "position": [650, 300], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_rifle", "position": [550, 300], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_rifle", "position": [600, 300], "engagement": "return_fire_only" },
    ]));
    let died = death(&run, 1);
    let jeep = sim::sensing::eye(run.unit(died, 1), &run.rules);
    for id in [2, 3, 4] {
        assert!(
            run.sees(died, id, jeep),
            "squad {id} sees where the jeep's crew sat"
        );
    }
    assert_eq!(losses_called(&run), [(died, 3)], "the nearest, once");
}

#[test]
fn the_destroyed_unit_never_witnesses_its_own_loss() {
    // The jeep is the only red unit that sees where it died: nobody calls it,
    // and it says only that it is destroyed.
    let run = jeep_lost(json!([]));
    let died = death(&run, 1);
    assert!(losses_called(&run).is_empty());
    let said: Vec<T> = run.spoke(died, 1).iter().map(|s| s.trigger).collect();
    assert_eq!(said, [T::B16Destroyed]);
}

// ---------------------------------------------------------------------------
// The unseen shout: an anonymous cue through hearing.

/// A red tank 500 m from a blue tank, beyond its sight, gives itself away
/// by firing every second; the blue crew shells the area and the red crew
/// calls the rounds landing round it. A blue truck, as blind to it, stands
/// farther off. Nothing blue sees the red tank. Sound buckets last
/// `bucket_s`.
fn unseen_shouter(bucket_s: f64) -> Run {
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 2 } }))
        .collect();
    let s = setup_events(
        [1200.0, 600.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "blue", "kind": "test_supply", "position": [60, 340] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        json!(fire),
        |rules| {
            chatty(rules);
            rules["sensors"]["sound_bucket_s"] = json!(bucket_s);
        },
    );
    play(&s, 3, 20.0, &[])
}

impl Run {
    fn bucket_ticks(&self) -> usize {
        (self.rules.sensors.sound_bucket_s * self.rules.tick_hz as f64).round() as usize
    }

    /// Blue's shout cues on `tick`.
    fn shout_cues(&self, tick: usize) -> Vec<&SoundCue> {
        self.moments[tick].audible[Side::Blue.index()]
            .iter()
            .filter(|c| c.category == SoundCategory::Shout)
            .collect()
    }
}

#[test]
fn an_unseen_enemys_shout_reaches_the_nearest_listener_as_one_anonymous_cue() {
    let run = unseen_shouter(0.5);
    let bucket = run.bucket_ticks();
    let said = barks(&run, 2);
    assert!(said.len() > 3, "the red crew talks: {said:?}");
    let mut cued = 0;
    for t in (bucket..run.moments.len()).step_by(bucket) {
        assert!(
            run.moments[t].identified[Side::Blue.index()].is_empty(),
            "blue never sees the red tank"
        );
        assert!(
            !run.moments[t].heard[Side::Blue.index()]
                .iter()
                .any(|s| matches!(s.speaker, ShoutSpeaker::Seen(_))),
            "nor hears what it shouts"
        );
        let shouted = said.iter().any(|&(s, _)| s + bucket >= t && s < t);
        let cues = run.shout_cues(t);
        if !shouted {
            assert!(cues.is_empty(), "tick {t}: a cue with no shout");
            continue;
        }
        cued += 1;
        // East of the nearer blue unit, its tank, beyond half the red
        // tank's 650 m carry.
        assert_eq!(
            cues,
            [&SoundCue {
                listener: UnitId(0),
                category: SoundCategory::Shout,
                sector: 0,
                band: contract::observation::SoundBand::Far,
                moving: false,
            }],
            "tick {t}"
        );
    }
    assert!(cued > 0);
}

#[test]
fn many_shouts_from_one_unseen_source_in_a_bucket_are_one_cue() {
    let run = unseen_shouter(10.0);
    let bucket = run.bucket_ticks();
    let said = barks(&run, 2);
    let mut crowded = 0;
    for t in (bucket..run.moments.len()).step_by(bucket) {
        let shouts = said
            .iter()
            .filter(|&&(s, _)| s + bucket >= t && s < t)
            .count();
        if shouts >= 5 {
            crowded += 1;
            assert_eq!(run.shout_cues(t).len(), 1, "tick {t}: {shouts} shouts");
        }
    }
    assert!(
        crowded > 0,
        "the red crew shouts five times in some bucket: {said:?}"
    );
}
