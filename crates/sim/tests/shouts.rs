//! Barks (slice 11): the simulation decides, every tick, which units shout
//! which bark, at which intensity, and which side hears it. Each test plays a
//! small real battle on the test units and judges every shout against the
//! authority's own state on the tick it was spoken.
use crate::common as test_common;
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::{
    ActionReason, Intensity, Shout, ShoutSpeaker, ShoutTrigger as T, SuppressionTier,
};
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::{json, Value};
use sim::battle::Battle;
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

#[test]
fn ai_sources_do_not_read_the_shout_channel() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/skirmish_ai/mod.rs"
    ))
    .unwrap();
    assert!(!source.contains(".shouts"));
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
    heard: [Vec<Shout>; 2],
    identified: [Vec<[f64; 3]>; 2],
    units: Vec<Unit>,
}

struct Run {
    rules: Rules,
    /// `moments[0]` is the battle as set up, before the first step.
    moments: Vec<Moment>,
}

fn snapshot(b: &Battle) -> Moment {
    let frame = |side| b.observe(side);
    Moment {
        heard: Side::ALL.map(|s| frame(s).shouts.clone()),
        identified: Side::ALL.map(|s| frame(s).identified.iter().map(|u| u.position).collect()),
        units: (0..).map_while(|i| b.unit(UnitId(i)).cloned()).collect(),
    }
}

/// `units` on an empty field `size` metres across, on the test rules as
/// `fixture` edits them.
fn setup(size: [f64; 2], units: Value, fixture: impl FnOnce(&mut Value)) -> ScenarioDefinition {
    let mut rules = test_common::scenario_rules();
    fixture(&mut rules);
    let map = json!({ "size": size, "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [] });
    let map = test_common::physical_map(&map.to_string());
    serde_json::from_value(json!({ "map": map, "rules": rules, "units": units,
        "events": [], "scripts": [] }))
    .unwrap()
}

/// Every bark takes one tick to say, so a unit is free to speak again on the
/// next: a bark test sees each piece of evidence without the talk-time rule
/// in the way. The cooldown stays as authored.
fn brief(rules: &mut Value) {
    let hz = rules["tick_hz"].as_f64().unwrap();
    for v in rules["shouts"]["talk_s"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        *v = json!(1.0 / hz);
    }
}

fn play(setup: &ScenarioDefinition, seed: u64, seconds: f64, orders: &[(Side, Order)]) -> Run {
    let mut b = Battle::new(setup, seed);
    for (seq, (side, order)) in orders.iter().enumerate() {
        test_common::order(&mut b, *side, seq as u64 + 1, order.clone());
    }
    let mut moments = vec![snapshot(&b)];
    for _ in 0..(seconds * b.rules().tick_hz as f64) as u64 {
        b.step();
        moments.push(snapshot(&b));
    }
    Run {
        rules: b.rules().clone(),
        moments,
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

    fn talk_ticks(&self, trigger: T) -> u64 {
        (self.rules.shouts.talk_s[&trigger] * self.rules.tick_hz as f64).round() as u64
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
        brief,
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
                (b - a) as u64 >= run.talk_ticks(k),
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
        for listener in Side::ALL {
            let l = listener.index();
            let mut expected: Vec<(T, [f64; 3], Intensity)> = m.heard[1 - l]
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
