//! Side objectives (D55): help, hold and claim parties through the planner.

use super::*;
use contract::ids::UnitId;

/// Party bounds these tests set for themselves.
fn tuning() -> TeamTuning {
    TeamTuning {
        party_budget_share: 0.5,
        claim_party_size: 2,
        hold_party_size: 2,
        open_objective_bonus: 100.0,
        influence_blend: 0.0,
        help_trend_threshold: 0.1,
        ..TeamTuning::shipped()
    }
}

fn blue_ai(count: usize) -> Vec<PlayerState> {
    players().into_iter().take(count).collect()
}

/// A match at `tick` with `units` (card, position, owner) on blue, and the
/// three objectives owned as `owners` (west, centre, east).
fn frame(
    rules: &Rules,
    units: &[(&str, [f64; 2], u16)],
    owners: [Option<Side>; 3],
) -> ObservationFrame {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(rules).unwrap(),
        "units":units.iter().map(|(kind, at, _)| serde_json::json!(
            {"side":"blue","kind":kind,"position":at}
        )).collect::<Vec<_>>(),
        "events":[],"scripts":[]
    }))
    .unwrap();
    let mut frame = crate::battle::Battle::new(&setup, 1)
        .observe(Side::Blue)
        .clone();
    for (unit, (_, _, owner)) in frame.own.iter_mut().zip(units) {
        unit.owner = Some(PlayerIndex(*owner));
    }
    let count = |player: u16| units.iter().filter(|u| u.2 == player).count() as u32;
    frame.skirmish = Some(contract::skirmish::SkirmishView {
        phase: contract::skirmish::Phase::Active,
        players: (0..2)
            .map(|index| contract::skirmish::PlayerView {
                index: PlayerIndex(index),
                id: format!("blue-{}", index + 1),
                controller: ControllerKind::ActiveAi,
                ready: true,
                credits: 0.0,
                unit_count: count(index),
                cap: 30,
                pending: vec![],
            })
            .collect(),
        preparation_remaining_s: 0.0,
        objectives: sites()
            .objectives
            .iter()
            .zip(owners)
            .map(|(o, owner)| contract::skirmish::ObjectiveView {
                id: o.id.clone(),
                center: o.center,
                radius_m: o.radius_m,
                owner,
                capturing: None,
                capture_progress: 0.0,
                contested: false,
            })
            .collect(),
        scores: [0.0; 2],
        result: None,
        report: None,
    });
    frame
}

fn set_owner(frame: &mut ObservationFrame, id: &str, owner: Option<Side>, contested: bool) {
    let view = frame.skirmish.as_mut().unwrap();
    let objective = view.objectives.iter_mut().find(|o| o.id == id).unwrap();
    objective.owner = owner;
    objective.contested = contested;
}

/// An enemy of `cost` seen at `at`.
fn enemy_at(rules: &Rules, frame: &mut ObservationFrame, at: [f64; 2], cost: u32) {
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(frame.identified.len() as u32 + 1),
            kind: rules.catalog.index("test_tank").unwrap(),
            cost,
            position: [at[0], at[1], 0.0],
            yaw: 0.0,
            velocity: [0.0, 0.0],
            members: vec![],
            member_ids: vec![],
            member_slots: vec![],
            member_active_mounts: vec![],
            weapon_poses: vec![],
            reversing: false,
            smoking: false,
            member_leans: vec![],
        });
}

/// Plan the next decision: far enough on that the planner replans.
fn step(
    rules: &Rules,
    frame: &mut ObservationFrame,
    players: &[PlayerState],
    memory: &mut TeamMemory,
    tuning: TeamTuning,
) -> TeamPlan {
    frame.tick += 10_000;
    plan(Side::Blue, rules, frame, players, &sites(), memory, tuning).clone()
}

fn tanks(count: usize, at: [f64; 2], owner: u16) -> Vec<(&'static str, [f64; 2], u16)> {
    (0..count)
        .map(|i| ("test_tank", [at[0] + 12.0 * i as f64, at[1]], owner))
        .collect()
}

/// No unit is in two parties, of any kinds.
fn assert_disjoint(plan: &TeamPlan) {
    let mut seen = BTreeSet::new();
    for party in &plan.parties {
        for id in &party.units {
            assert!(seen.insert(*id), "{id:?} is in two parties: {plan:?}");
        }
    }
}

fn party<'a>(plan: &'a TeamPlan, kind: PartyKind, objective: &str) -> Option<&'a Party> {
    plan.parties
        .iter()
        .find(|p| p.kind == kind && p.objective.id == objective)
}

#[test]
fn a_taken_main_objective_moves_on_and_leaves_a_hold_party() {
    let rules = team_rules();
    let players = blue_ai(1);
    let mut memory = TeamMemory::default();
    let mut frame = frame(
        &rules,
        &tanks(6, [100.0, 420.0], 0),
        [None, Some(Side::Red), Some(Side::Red)],
    );
    let first = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_eq!(first.players[0].main_objective.id, "west");

    set_owner(&mut frame, "west", Some(Side::Blue), false);
    let next = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_ne!(next.players[0].main_objective.id, "west", "it moves on");
    let hold = party(&next, PartyKind::Hold, "west").expect("a hold party on the taken one");
    assert_eq!(hold.owner, PlayerIndex(0));
    assert_eq!(hold.units.len(), tuning().hold_party_size);
}

#[test]
fn a_taken_main_objective_still_contested_stays_the_main() {
    let rules = team_rules();
    let players = blue_ai(1);
    let mut memory = TeamMemory::default();
    let mut frame = frame(
        &rules,
        &tanks(6, [100.0, 420.0], 0),
        [None, Some(Side::Red), Some(Side::Red)],
    );
    let first = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_eq!(first.players[0].main_objective.id, "west");

    // Taken, but the enemy is on it: the group stays to defend it.
    set_owner(&mut frame, "west", Some(Side::Blue), true);
    let next = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_eq!(next.players[0].main_objective.id, "west", "{next:?}");
}

#[test]
fn an_objective_a_human_teammate_is_going_to_gets_no_claim() {
    let rules = team_rules();
    let mut players = blue_ai(2);
    players[0].controller = ControllerKind::Human;
    // Your squad is ordered onto the centre; your AI ally's tanks hold the
    // east lane with room for claims.
    let mut units = vec![("test_rifle", [300.0, 300.0], 0)];
    units.extend(tanks(6, [500.0, 100.0], 1));
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units, [None, None, None]);
    frame.own[0].goal = Some([300.0, 500.0]);
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert!(
        plan.parties.iter().all(|p| p.objective.id != "centre"),
        "{plan:?}"
    );
    assert_ne!(plan.players[0].main_objective.id, "centre", "{plan:?}");
}

#[test]
fn an_open_objective_near_the_lane_gets_a_claim_and_the_main_group_goes_elsewhere() {
    let rules = team_rules();
    let players = blue_ai(1);
    let mut memory = TeamMemory::default();
    let mut frame = frame(
        &rules,
        &tanks(6, [100.0, 100.0], 0),
        [None, None, Some(Side::Red)],
    );
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning());
    let main = &plan.players[0].main_objective.id;
    let claim = plan
        .parties
        .iter()
        .find(|p| p.kind == PartyKind::Claim)
        .expect("the other open objective is claimed");
    assert_ne!(
        &claim.objective.id, main,
        "the main group does not go there"
    );
    assert!(!claim.units.is_empty() && claim.units.len() <= tuning().claim_party_size);
    let assignment = plan.assignment_for(PlayerIndex(0)).unwrap();
    assert!(assignment
        .parties
        .iter()
        .any(|task| task.kind == PartyKind::Claim && task.units == claim.units));
}

#[test]
fn a_party_is_capable_holders_never_aircraft_alone_and_never_over_budget() {
    let rules = team_rules();
    let players = blue_ai(1);
    // Nearest the open centre: a helicopter (fights, cannot hold) and an
    // air-defence vehicle (holds, cannot fight on the ground); the tanks
    // are farther back. A claim of three is wanted, a budget of two allows.
    let units = |tanks: usize| {
        let mut units = vec![
            ("test_heli", [300.0, 470.0], 0),
            ("test_gun_jeep", [300.0, 480.0], 0),
        ];
        units.extend(self::tanks(tanks, [100.0, 100.0], 0));
        units
    };
    let tuning = TeamTuning {
        claim_party_size: 3,
        ..tuning()
    };
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units(2), [None, None, Some(Side::Red)]);
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning);
    let kind = |id: &UnitId| frame.own.iter().find(|u| u.id == *id).unwrap().kind;
    let holds = |id: &UnitId| crate::objectives::can_hold(rules.catalog.get(kind(id)));
    assert!(!plan.parties.is_empty(), "a claim is sent: {plan:?}");
    for party in &plan.parties {
        assert!(party.units.iter().any(holds), "no holder in {party:?}");
        assert!(
            party
                .units
                .iter()
                .all(|id| super::super::super::unit_can_engage_ground(&rules, kind(id))),
            "a unit that cannot engage ground went: {party:?}"
        );
    }
    let on_duty: usize = plan.parties.iter().map(|p| p.units.len()).sum();
    assert!(on_duty <= 2, "over the budget of 2: {plan:?}");
    let in_party: BTreeSet<_> = plan.parties.iter().flat_map(|p| p.units.clone()).collect();
    assert!(
        frame
            .own
            .iter()
            .any(|u| !in_party.contains(&u.id) && holds(&u.id) && u.kind == kind(&frame.own[2].id)),
        "the main group keeps a tank to hold its objective"
    );

    // With one tank, the main group's last holder stays: the helicopter
    // never goes alone, so no claim goes at all.
    let mut memory = TeamMemory::default();
    let mut frame = self::frame(&rules, &units(1), [None, None, Some(Side::Red)]);
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning);
    assert!(plan.parties.is_empty(), "{plan:?}");
}

#[test]
fn a_unit_is_in_one_party_and_an_engaged_party_is_not_retasked() {
    let rules = team_rules();
    let players = blue_ai(2);
    let mut units = tanks(6, [100.0, 100.0], 0);
    units.extend(tanks(6, [500.0, 100.0], 1));
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units, [None, None, None]);
    let first = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_disjoint(&first);
    let claim = first
        .parties
        .iter()
        .find(|p| p.kind == PartyKind::Claim)
        .expect("the third objective is claimed")
        .clone();

    // The other player's lane comes under pressure, and the claim's owner
    // answers. Its help is never a unit already in its claim.
    let pressed_player = PlayerIndex(1 - claim.owner.0);
    let lane = first
        .for_player(pressed_player)
        .unwrap()
        .main_objective
        .clone();
    // Not so strong that the lane gives its objective up.
    enemy_at(&rules, &mut frame, lane.center, 300);
    let pressed = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert!(
        pressed
            .help_requests
            .iter()
            .any(|r| r.objective.id == lane.id
                && r.answered_by == Some(claim.owner)
                && pressed.help_party(r).is_some_and(|p| !p.units.is_empty())),
        "{pressed:?}"
    );
    assert_disjoint(&pressed);

    // The claim is engaged when its objective falls to the enemy: it is not
    // called back or re-tasked while it fights.
    let fighter = frame
        .own
        .iter_mut()
        .find(|u| claim.units.contains(&u.id))
        .unwrap();
    fighter.engagement = contract::command::Engagement::FireAtWill;
    fighter
        .sees
        .push(contract::observation::ObservedTargetId(99));
    set_owner(&mut frame, &claim.objective.id, Some(Side::Red), false);
    let fighting = step(&rules, &mut frame, &players, &mut memory, tuning());
    let kept = party(&fighting, PartyKind::Claim, &claim.objective.id).expect("claim kept");
    assert_eq!(kept.units, claim.units);
    assert_disjoint(&fighting);
}

#[test]
fn help_is_answered_from_surplus_when_side_parties_fill_the_budget() {
    let rules = team_rules();
    let players = blue_ai(2);
    let mut units = tanks(6, [100.0, 100.0], 0);
    units.extend(tanks(6, [500.0, 100.0], 1));
    // Each player's side-party budget is one claim's worth of its six tanks.
    let tuning = TeamTuning {
        party_budget_share: 0.34,
        ..tuning()
    };
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units, [None, None, None]);
    let first = step(&rules, &mut frame, &players, &mut memory, tuning);
    let claim = first
        .parties
        .iter()
        .find(|p| p.kind == PartyKind::Claim)
        .expect("an open objective is claimed")
        .clone();
    assert_eq!(claim.units.len(), 2, "the claim fills its owner's budget");

    // The other lane comes under pressure: the claim's owner still has
    // surplus beyond its claim, and answers.
    let pressed_player = PlayerIndex(1 - claim.owner.0);
    let lane = first
        .for_player(pressed_player)
        .unwrap()
        .main_objective
        .clone();
    enemy_at(&rules, &mut frame, lane.center, 300);
    let pressed = step(&rules, &mut frame, &players, &mut memory, tuning);
    assert!(
        pressed
            .help_requests
            .iter()
            .any(|r| r.objective.id == lane.id
                && r.answered_by == Some(claim.owner)
                && pressed.help_party(r).is_some_and(|p| !p.units.is_empty())),
        "{pressed:?}"
    );
    assert_disjoint(&pressed);
}

#[test]
fn a_lost_claim_is_replaced_and_never_blocks_its_objective() {
    let rules = team_rules();
    let players = blue_ai(1);
    let all = tanks(6, [100.0, 100.0], 0);
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &all, [None, None, Some(Side::Red)]);
    let fielded = frame.own.clone();
    let first = step(&rules, &mut frame, &players, &mut memory, tuning());
    let claim = first
        .parties
        .iter()
        .find(|p| p.kind == PartyKind::Claim)
        .unwrap()
        .clone();

    // Its units die on the way: others replace them.
    frame.own.retain(|u| !claim.units.contains(&u.id));
    let replaced = step(&rules, &mut frame, &players, &mut memory, tuning());
    let again = party(&replaced, PartyKind::Claim, &claim.objective.id).expect("replaced");
    assert!(!again.units.is_empty());
    assert!(again.units.iter().all(|id| !claim.units.contains(id)));

    // Nobody left to send: the claim goes, so it blocks nothing; once units
    // are back, the objective is claimed again.
    frame.own.truncate(1);
    let empty = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert!(party(&empty, PartyKind::Claim, &claim.objective.id).is_none());
    frame.own = fielded;
    let back = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert!(party(&back, PartyKind::Claim, &claim.objective.id).is_some());
}

#[test]
fn a_side_objective_under_pressure_keeps_its_hold_and_raises_help() {
    let rules = team_rules();
    let players = blue_ai(2);
    let mut units = tanks(6, [100.0, 420.0], 0);
    units.extend(tanks(6, [500.0, 100.0], 1));
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units, [None, Some(Side::Red), Some(Side::Red)]);
    let first = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_eq!(
        first.for_player(PlayerIndex(0)).unwrap().main_objective.id,
        "west"
    );
    set_owner(&mut frame, "west", Some(Side::Blue), false);
    let held = step(&rules, &mut frame, &players, &mut memory, tuning());
    let hold = party(&held, PartyKind::Hold, "west").expect("hold").clone();

    // The enemy arrives and contests it: the hold stays, and help comes.
    enemy_at(&rules, &mut frame, sites().objectives[0].center, 600);
    set_owner(&mut frame, "west", Some(Side::Blue), true);
    let pressed = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert_eq!(
        party(&pressed, PartyKind::Hold, "west").map(|p| &p.units),
        Some(&hold.units),
        "a contested hold is kept"
    );
    let request = pressed
        .help_requests
        .iter()
        .find(|r| r.objective.id == "west")
        .expect("the side objective raises help");
    assert_eq!(request.requester, PlayerIndex(0));
    assert_eq!(request.answered_by, Some(PlayerIndex(1)));
    assert!(pressed
        .help_party(request)
        .is_some_and(|p| !p.units.is_empty()));
    assert_disjoint(&pressed);
}

#[test]
fn a_player_answering_help_keeps_its_other_parties_on_their_goals() {
    let site = |i: usize| sites().objectives[i].clone();
    let party = |kind, objective: ObjectiveSite, unit| Party {
        kind,
        owner: PlayerIndex(1),
        objective,
        units: vec![UnitId(unit)],
        size: 1,
        raised_tick: 0,
        clear_since: None,
    };
    let plan = TeamPlan {
        players: vec![PlayerPlan {
            player: PlayerIndex(1),
            lane: EntryIndex(1),
            main_objective: site(1),
            staging_point: [300.0, 300.0],
        }],
        help_requests: vec![HelpRequest {
            requester: PlayerIndex(0),
            lane: EntryIndex(0),
            objective: site(0),
            answered_by: Some(PlayerIndex(1)),
            raised_tick: 0,
        }],
        parties: vec![
            party(PartyKind::Help, site(0), 1),
            party(PartyKind::Claim, site(2), 2),
        ],
        stance: TeamStance::Hold,
        push_started_tick: None,
        outgunned: BTreeSet::new(),
    };
    let tasks = plan.assignment_for(PlayerIndex(1)).unwrap().parties;
    for (unit, goal) in [(1, site(0).center), (2, site(2).center)] {
        assert!(
            tasks
                .iter()
                .any(|t| t.units == vec![UnitId(unit)] && t.objective == goal),
            "unit {unit} keeps its party goal: {tasks:?}"
        );
    }
}

#[test]
fn the_main_objective_prefers_an_open_objective_over_a_defended_one() {
    // Three lanes, each as far from its objective; only the east is open.
    let rules = team_rules();
    let players = blue_ai(1);
    let choose = |bonus| {
        let mut memory = TeamMemory::default();
        let mut frame = frame(&rules, &[], [Some(Side::Red), Some(Side::Red), None]);
        let tuning = TeamTuning {
            open_objective_bonus: bonus,
            ..tuning()
        };
        step(&rules, &mut frame, &players, &mut memory, tuning).players[0]
            .main_objective
            .id
            .clone()
    };
    assert_eq!(choose(100.0), "east");
    assert_ne!(choose(0.0), "east", "the bonus, not the order, decides");
}

#[test]
fn a_group_readies_without_its_air_defence_at_the_backline() {
    // Two tanks gathered at staging, and an air-defence vehicle at its post
    // far behind them: the group is ready.
    let rules = team_rules();
    let players = blue_ai(1);
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &[], [None, Some(Side::Red), Some(Side::Red)]);
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning());
    let staging = plan.players[0].staging_point;
    let units = [
        ("test_tank", staging, 0),
        ("test_tank", [staging[0] + 10.0, staging[1]], 0),
        ("test_gun_jeep", [staging[0], staging[1] - 250.0], 0),
    ];
    let mut gathered = self::frame(&rules, &units, [None, Some(Side::Red), Some(Side::Red)]);
    gathered.tick = frame.tick + 1_000_000;
    memory.latch_group_readiness(&rules, &gathered, tuning().group_radius_m);
    assert!(memory.ready_groups.contains(&PlayerIndex(0)));
}

#[test]
fn a_carrier_and_its_passengers_go_to_a_party_together() {
    let rules = team_rules();
    let players = blue_ai(1);
    // Nearest the open centre, 150 m short of it: a tank, then a carrier
    // with a rifle squad aboard. The claim has room for two; the main
    // group's tanks stand back in the west lane.
    let mut units = tanks(4, [100.0, 100.0], 0);
    units.extend([
        ("test_tank", [300.0, 360.0], 0),
        ("test_apc", [300.0, 350.0], 0),
        ("test_rifle", [300.0, 350.0], 0),
    ]);
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units, [None, None, Some(Side::Red)]);
    let (tank, carrier, squad) = (frame.own[4].id, frame.own[5].id, frame.own[6].id);
    frame.own[6].hold = Some(contract::observation::HoldState {
        host: contract::observation::HostRef::Carrier { id: carrier.0 },
        phase: contract::observation::HoldPhase::Inside,
        progress: 1.0,
    });
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning());
    let claim = party(&plan, PartyKind::Claim, "centre").expect("the centre is claimed");
    assert!(claim.units.len() <= tuning().claim_party_size, "{claim:?}");
    // The tank takes one place; the carrier and its squad need two, so a
    // tank from the main group takes the other.
    assert!(
        claim.units.contains(&tank),
        "the nearest goes first: {claim:?}"
    );
    assert_eq!(
        claim.units.len(),
        2,
        "the pair left room for another: {claim:?}"
    );
    for party in &plan.parties {
        assert_eq!(
            party.units.contains(&carrier),
            party.units.contains(&squad),
            "a passenger never goes without its carrier, nor a carrier without it: {party:?}"
        );
    }
}

#[test]
fn a_carrier_carrying_air_defence_goes_to_no_party() {
    let rules = team_rules();
    let players = blue_ai(1);
    // Nearest the open centre: an armed carrier with an air-defence team
    // aboard (the test cards' armed tank and air-defence vehicle stand in,
    // seated as the observation shows a passenger). The main group's tanks
    // stand back in the west lane.
    let mut units = tanks(4, [100.0, 100.0], 0);
    units.extend([
        ("test_tank", [300.0, 440.0], 0),
        ("test_gun_jeep", [300.0, 440.0], 0),
    ]);
    let mut memory = TeamMemory::default();
    let mut frame = frame(&rules, &units, [None, None, Some(Side::Red)]);
    let (carrier, aboard) = (frame.own[4].id, frame.own[5].id);
    frame.own[5].hold = Some(contract::observation::HoldState {
        host: contract::observation::HostRef::Carrier { id: carrier.0 },
        phase: contract::observation::HoldPhase::Inside,
        progress: 1.0,
    });
    let plan = step(&rules, &mut frame, &players, &mut memory, tuning());
    assert!(
        plan.parties
            .iter()
            .all(|p| !p.units.contains(&carrier) && !p.units.contains(&aboard)),
        "air defence holds the backline, even aboard: {plan:?}"
    );
}
