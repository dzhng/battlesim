//! An authored scenario's director, on test units: chapters open in order,
//! a beat fires once when its trigger holds, and its actions reach only what
//! they may (a directed seat's units, a player's wallet and shop), the same
//! live and in replay.

use crate::scenarios::{document, resolve, rules_with};
use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;

/// The chapter every test director ends on, as every director must: one
/// that closes on the match's result.
pub(crate) fn closing() -> Value {
    json!({"id":"close","beats":[{"id":"won","when":{"match_result":{"kind":"winner","side":"blue"}}}]})
}

/// The sample scenario with a blue rifle squad placed beside red's tank and
/// `chapters` as its director.
fn directed(chapters: Value) -> Value {
    let mut doc = document();
    doc["forces"]["rifles"] =
        json!({"side":"blue","owner":"blue-1","kind":"test_rifle","position":[60.0,60.0]});
    doc["zones"]["east"] = json!({"center":[280.0,60.0],"radius_m":30.0});
    let mut chapters = chapters;
    chapters.as_array_mut().unwrap().push(closing());
    doc["director"] = json!({ "chapters": chapters });
    doc
}

fn battle(chapters: Value) -> Battle {
    Battle::new(&resolve(directed(chapters), rules_with(|_| {})).unwrap(), 7)
}

fn fired(battle: &Battle, beat: &str) -> Option<u64> {
    battle.director().unwrap().fired_at(beat)
}

#[test]
fn chapters_open_in_order_and_a_beat_waits_for_its_own() {
    let mut b = battle(json!([
        {"id":"first","beats":[{"id":"opens","when":"chapter_opens"}]},
        // True from the start, but only armed once its chapter opens.
        {"id":"second","beats":[{"id":"later","when":{"phase":"preparation"}}]}
    ]));
    b.step();
    b.step();
    let (opens, later) = (fired(&b, "opens").unwrap(), fired(&b, "later").unwrap());
    assert_eq!(
        later,
        opens + 1,
        "the next chapter opens on the tick after the last beat"
    );
    for _ in 0..5 {
        b.step();
    }
    assert_eq!(fired(&b, "later"), Some(later), "a beat fires once");
}

#[test]
fn a_beat_reads_a_fact_however_it_came_about() {
    let mut b = battle(
        json!([{"id":"one","beats":[{"id":"holding","when":{"units":{
        "who":{"force":"rifles"},"are":{"engagement":"return_fire_only"}}}}]}]),
    );
    b.step();
    assert_eq!(fired(&b, "holding"), None);
    // The player sets it, by an ordinary order.
    let ack = b.accept_order(
        Side::Blue,
        Some("blue-1".into()),
        contract::command::Order::SetEngagement {
            units: vec![contract::ids::UnitId(0)],
            policy: contract::command::Engagement::ReturnFireOnly,
        },
        false,
    );
    assert_eq!(ack.error, None);
    for _ in 0..3 {
        b.step();
    }
    assert!(fired(&b, "holding").is_some());
}

#[test]
fn a_cue_moves_a_directed_seats_units_and_no_one_elses_unrecorded_and_replayed() {
    // Once the battle starts: nothing moves in preparation.
    let chapters = json!([{"id":"one","beats":[{"id":"go","when":{"phase":"active"},"then":[
        {"order":{"units":{"force":"tank"},"order":{"move_to":"east"}}},
        {"order":{"units":{"force":"rifles"},"order":{"move_to":"east"}}}
    ]}]}]);
    let setup = resolve(directed(chapters), rules_with(|_| {})).unwrap();
    let mut b = Battle::new(&setup, 7);
    let start = |b: &Battle, side: Side| b.observe(side).own[0].position;
    let (tank0, rifles0) = (start(&b, Side::Red), start(&b, Side::Blue));
    let ready = b.accept_order(
        Side::Blue,
        Some("blue-1".into()),
        contract::command::Order::Ready,
        false,
    );
    assert_eq!(ready.error, None);
    let mut digests = Vec::new();
    for _ in 0..(b.rules().tick_hz * 4) {
        b.step();
        digests.push(b.digest());
    }
    let moved = |from: [f64; 3], to: [f64; 3]| (to[0] - from[0]).hypot(to[1] - from[1]);
    let to_goal = |at: [f64; 3]| (280.0 - at[0]).hypot(60.0 - at[1]);
    assert!(
        to_goal(start(&b, Side::Red)) < to_goal(tank0) - 5.0,
        "the directed tank drives for its goal"
    );
    // A squad settles where it stands, but goes nowhere it isn't sent.
    assert!(
        moved(rifles0, start(&b, Side::Blue)) < 5.0,
        "the human's rifles are not the director's"
    );
    assert!(b.director().unwrap().refusals().is_empty());
    let replay = b.replay();
    assert!(
        replay
            .accepted
            .iter()
            .all(|(_, c)| c.player.as_deref() == Some("blue-1")),
        "the director's orders are not recorded, only the player's ready"
    );
    let mut again = Battle::from_replay(&setup, &replay).unwrap();
    for digest in digests {
        again.step();
        assert_eq!(again.digest(), digest, "the replay reruns the director");
    }
}

#[test]
fn a_grant_and_an_offer_reach_the_wallet_and_the_shop() {
    let mut doc = directed(
        json!([{"id":"one","beats":[{"id":"gift","when":"chapter_opens","then":[
            {"grant":{"player":"blue-1","credits":{"exactly":500}}},
            {"offer":{"player":"blue-1","purchases":[{"variant":"test_rifle"}]}}
        ]}]}]),
    );
    doc["offered"] = json!({"blue-1": [{"variant":"test_rifle","carrier":"test_apc"}]});
    let mut b = Battle::new(&resolve(doc, rules_with(|_| {})).unwrap(), 7);
    let credits = |b: &Battle| b.observe(Side::Blue).skirmish.as_ref().unwrap().players[0].credits;
    let before = credits(&b);
    b.step();
    b.step();
    assert_eq!(credits(&b), before + 500.0);
    let offered = b.observe(Side::Blue).skirmish.as_ref().unwrap().players[0]
        .offered
        .clone();
    assert_eq!(
        offered.map(|o| o.len()),
        Some(2),
        "the squad on foot joins its ride"
    );
}

#[test]
fn a_director_naming_what_its_scenario_lacks_is_refused_by_beat() {
    let refused =
        |chapters: Value| resolve(directed(chapters), rules_with(|_| {})).expect_err("refused");
    let beat = |when: Value| json!([{"id":"c","beats":[{"id":"b","when":when}]}]);
    assert!(refused(beat(json!({"fired":{"beat":"b"}}))).starts_with("director.c.b"));
    assert!(
        refused(beat(json!({"contact_in":{"side":"blue","zone":"nowhere"}}))).contains("nowhere")
    );
    assert!(refused(beat(
        json!({"identifies":{"side":"blue","who":{"force":"ghost"}}})
    ))
    .contains("ghost"));
    assert!(refused(
        json!([{"id":"c","beats":[{"id":"b","when":"chapter_opens","then":[
        {"grant":{"player":"nobody","credits":{"exactly":1}}}]}]}])
    )
    .contains("nobody"));
    // The story ends where the match does.
    let mut doc = document();
    doc["director"] = json!({"chapters":[{"id":"c","beats":[{"id":"b","when":"chapter_opens"}]}]});
    let open_ended = resolve(doc, rules_with(|_| {})).expect_err("refused");
    assert!(open_ended.contains("director.c"), "{open_ended}");
}

/// `side`'s publication, read back as the page reads it.
fn read_back(battle: &Battle, side: Side) -> Value {
    let words = sim::publication::Publisher::new()
        .publish(battle, side)
        .unwrap()
        .iter()
        .map(|w| w.to_bits())
        .collect::<Vec<_>>();
    let decoded = sim::publication::reader::Reader::new(&sim::publication::layout_json(battle))
        .unwrap()
        .decode(&words)
        .unwrap()
        .unwrap();
    serde_json::to_value(&decoded).unwrap()
}

#[test]
fn a_beat_is_published_by_name_only_to_its_audience() {
    let mut b = battle(json!([
        {"id":"briefing","beats":[{"id":"told","when":"chapter_opens","audience":["blue"]}]},
        {"id":"next","beats":[{"id":"quiet","when":{"fired":{"beat":"told","after_s":60}}}]}
    ]));
    b.step();
    b.step();
    let blue = read_back(&b, Side::Blue);
    let red = read_back(&b, Side::Red);
    assert_eq!(
        blue["scenario"]["chapter"],
        json!("next"),
        "every side knows the chapter"
    );
    assert_eq!(red["scenario"]["chapter"], json!("next"));
    let told = &blue["scenario"]["beats"];
    assert_eq!(told.as_array().unwrap().len(), 1);
    assert_eq!(told[0]["beat"], json!("told"));
    assert_eq!(
        red["scenario"]["beats"],
        json!([]),
        "a side outside the audience hears nothing"
    );
    // A battle with no director publishes no scenario.
    let plain = Battle::new(&resolve(document(), rules_with(|_| {})).unwrap(), 7);
    assert!(read_back(&plain, Side::Blue)["scenario"].is_null());
}
