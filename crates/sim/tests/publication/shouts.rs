//! A shout in the publication: a heard bark's speaker, kind, place, trigger
//! and intensity read back by the layout alone, and an unseen enemy's shout
//! only as an anonymous audible cue.
use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication;

use super::{decode, full_fog, integer, names};
use crate::common;

#[test]
fn a_shout_publishes_its_speaker_kind_place_trigger_and_intensity() {
    // Read back by the layout alone: whose voice (own unit or seen enemy,
    // each id exact past float32's 2²⁴), its type, where it was said, which
    // bark and how. An unseen enemy's cue rides in `audible` as a shout.
    use contract::ids::UnitId;
    use contract::observation::{
        Intensity, ObservedTargetId, Shout, ShoutSpeaker, ShoutTrigger, SoundBand, SoundCategory,
        SoundCue,
    };
    let map =
        json!({ "size": [400, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    let b = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [100, 100] }]),
            json!([]),
        ),
        1,
    );
    let mut frame = b.observe(Side::Blue).clone();
    let big = (1u32 << 24) + 3;
    frame.shouts = vec![
        Shout {
            speaker: ShoutSpeaker::Own(UnitId(u32::MAX)),
            kind: common::unit_kind("test_rifle"),
            position: [120.5, 80.25, 1.5],
            trigger: ShoutTrigger::B17FriendlyDestroyedNearby,
            intensity: Intensity::Stressed,
        },
        Shout {
            speaker: ShoutSpeaker::Seen(ObservedTargetId(big)),
            kind: common::unit_kind("test_tank"),
            position: [300.0, 250.0, 2.0],
            trigger: ShoutTrigger::B1ContactInfantry,
            intensity: Intensity::Steady,
        },
    ];
    frame.audible = vec![SoundCue {
        listener: UnitId(0),
        category: SoundCategory::Shout,
        sector: 3,
        band: SoundBand::Far,
        moving: false,
    }];
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let bits = layout["limbBits"].as_u64().unwrap() as u32;
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: false,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(
        &frame,
        &[],
        &patch,
        &full_fog(),
        std::iter::empty(),
        &mut data,
    )
    .unwrap();
    let groups = decode(&layout, &data);
    let tag = |table: &str, i: f32| layout[table][i as usize].as_str().unwrap().to_owned();
    let read: Vec<_> = groups["shouts"]
        .iter()
        .map(|s| {
            let f = &s.fields;
            (
                f["own"],
                integer(bits, f, "id"),
                tag("unitKinds", f["kind"]),
                [f["x"], f["y"], f["z"]],
                tag("shoutTriggers", f["trigger"]),
                tag("intensities", f["intensity"]),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (
                1.0,
                u32::MAX,
                "test_rifle".to_owned(),
                [120.5, 80.25, 1.5],
                "b17_friendly_destroyed_nearby".to_owned(),
                "stressed".to_owned()
            ),
            (
                0.0,
                big,
                "test_tank".to_owned(),
                [300.0, 250.0, 2.0],
                "b1_contact_infantry".to_owned(),
                "steady".to_owned()
            ),
        ]
    );
    let cue = &groups["audible"][0].fields;
    assert_eq!(tag("soundCategories", cue["category"]), "shout");
    assert_eq!([cue["listener"], cue["sector"]], [0.0, 3.0]);
}

#[test]
fn the_layout_names_every_bark_and_intensity_in_contract_order() {
    // The tags are the contract's serde names, B1 first: TypeScript reads
    // them from here and never hard-codes them.
    use contract::observation::{Intensity, ShoutTrigger};
    let b = Battle::new(
        &common::scenario(
            &json!({ "size": [64, 64], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
                .to_string(),
            json!([]),
            json!([]),
        ),
        1,
    );
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let triggers = names(&layout["shoutTriggers"]);
    assert_eq!(triggers.len(), 17);
    for (i, name) in triggers.iter().enumerate() {
        assert!(name.starts_with(&format!("b{}_", i + 1)), "{name}");
        let parsed: ShoutTrigger = serde_json::from_value(json!(name)).unwrap();
        assert_eq!(json!(parsed), json!(name));
    }
    assert_eq!(
        names(&layout["intensities"]),
        [Intensity::Steady, Intensity::Stressed].map(|i| json!(i).as_str().unwrap().to_owned())
    );
    assert_eq!(
        names(&layout["soundCategories"]),
        ["infantry", "vehicle", "shot", "shout"]
    );
}

/// A red tank 500 m from a blue tank, beyond its sight, gives itself away
/// by firing every second; the blue crew shells the area and the red crew
/// calls the rounds landing round it. Each bark lasts a tick and may be
/// said again on the next, so the red crew talks often.
fn unseen_shouter() -> Battle {
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 2 } }))
        .collect();
    let mut setup = common::scenario(
        &json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "blue", "kind": "test_supply", "position": [60, 340] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        json!(fire),
    );
    let tick = 1.0 / setup.rules.tick_hz as f64;
    setup.rules.shouts.cooldown_s = tick;
    setup.rules.shouts.talk_s = tick;
    Battle::new(&setup, 3)
}

#[test]
fn an_unseen_enemys_shout_is_published_only_as_an_anonymous_audible_cue() {
    use contract::observation::{ShoutSpeaker, SoundCategory};
    use sim::publication::reader::Reader;
    let mut battle = unseen_shouter();
    let layout = publication::layout_json(&battle);
    let mut publisher = publication::Publisher::new();
    let mut reader = Reader::new(&layout).unwrap();
    let (mut spoken, mut cues, mut own) = (0, 0, 0);
    for _ in 0..20 * battle.rules().tick_hz {
        battle.step();
        spoken += battle
            .observe(Side::Red)
            .shouts
            .iter()
            .filter(|s| s.speaker == ShoutSpeaker::Own(contract::ids::UnitId(2)))
            .count();
        let words: Vec<u32> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect();
        let o = reader.decode(&words).unwrap().unwrap();
        assert!(o.identified.is_empty(), "blue never sees the red tank");
        assert!(
            o.shouts.iter().all(|s| s.own),
            "tick {}: an unseen enemy's bark was published: {:?}",
            o.tick,
            o.shouts
        );
        own += o.shouts.len();
        // Every shout the side heard arrives as a cue, and nothing more.
        let heard = o.audible.iter().filter(|c| &*c.category == "shout").count();
        let frame = battle.observe(Side::Blue);
        let said = frame.audible.iter();
        assert_eq!(
            heard,
            said.filter(|c| c.category == SoundCategory::Shout).count()
        );
        cues += heard;
    }
    assert!(spoken > 3, "the red crew talks: {spoken}");
    assert!(cues > 0, "blue hears it shout");
    assert!(own > 0, "blue's own barks are published");
}
