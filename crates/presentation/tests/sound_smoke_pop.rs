//! A smoke screen's pop, as a side hears it (`specs/smoke-screen`, slice 10):
//! a side that sees a cloud while it blooms hears its pop once, at the cloud;
//! a side that first sees it later never hears it; and a side always hears
//! its own unit's pop. A look naming no pop sound, or one the library lacks,
//! is refused before a battle. Every number is the test catalog's and the
//! shipped mix's.
use contract::smoke::SmokeCloud;
use presentation::effects::publication::{EffectFeed, EffectPublication};
use presentation::sound::frame::{Command, Heard, Listener, Motion, Sound, Voice};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::{reader::Reader, Publisher};

#[path = "../../sim/tests/common/mod.rs"]
mod common;

fn fixture(name: &str) -> Value {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/");
    serde_json::from_str(&std::fs::read_to_string(format!("{root}{name}")).unwrap()).unwrap()
}

/// The game's sound as a player sets it up, every battle sound two seconds long.
fn sound() -> Sound {
    let game = fixture("rules.json");
    let setup = json!({
        "audio": fixture("audio.json"),
        "presentation": fixture("presentation.json"),
        "catalog": fixture("sounds.json"),
        "weapons": game["weapons"],
        "unit_catalog": {"units": [], "soldiers": {}},
        "tick_hz": game["tick_hz"],
    });
    let mut s = Sound::from_json(&setup.to_string()).unwrap();
    let durations = (s.tables().battle_sounds().into_iter())
        .map(|n| (n, vec![2.0; 8]))
        .collect();
    s.set_durations(durations).unwrap();
    s
}

/// The sound a cloud's look pops with.
fn pop_sound(cloud: &SmokeCloud) -> String {
    let look = &fixture("presentation.json")["smoke_screen_looks"][&cloud.spec.look];
    look["sound"].as_str().unwrap().into()
}

/// The cloud the test smoke tank pops: cloud `id`, at `centre`, on `tick`.
fn cloud(id: u32, centre: [f64; 3], tick: u64) -> SmokeCloud {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let smoke = rules
        .catalog
        .by_id("test_smoke_tank")
        .capabilities
        .smoke
        .clone();
    SmokeCloud {
        id,
        centre,
        popped: tick,
        spec: smoke.expect("a smoke carrier").cloud,
    }
}

/// Tick `tick`'s publication, the side seeing `clouds`.
fn publication(tick: u64, clouds: &[SmokeCloud]) -> EffectPublication {
    serde_json::from_value(json!({
        "tick": tick as f64, "segments": [], "blasts": [], "shooters": [], "smokes": [],
        "smokeClouds": clouds,
    }))
    .unwrap()
}

/// Hears `effects` at its tick, listening at `at`: the one-shots it starts.
fn hear(s: &mut Sound, effects: &EffectPublication, hz: f64, at: [f64; 3]) -> Vec<Voice> {
    let now = effects.tick / hz;
    let heard = Heard {
        effects,
        launches: &[],
        audible: Vec::new(),
        shouts: Vec::new(),
    };
    s.note(&heard, now);
    let listener = Listener {
        position: at,
        forward: [0.0, 1.0, 0.0],
    };
    let commands = s.update(now, now, now, &Motion::default(), listener);
    (commands.iter())
        .filter_map(|c| match c {
            Command::Start { voice, .. } if !voice.looping => Some(voice.clone()),
            _ => None,
        })
        .collect()
}

fn tick_hz() -> f64 {
    fixture("rules.json")["tick_hz"].as_f64().unwrap()
}

/// The first tick past `blooms` of `c`'s bloom times after its pop.
fn past(c: &SmokeCloud, blooms: f64, hz: f64) -> u64 {
    c.popped + (blooms * c.spec.bloom_s * hz).ceil() as u64 + 1
}

#[test]
fn a_side_that_sees_a_cloud_at_its_pop_hears_the_pop_once_at_the_cloud() {
    let hz = tick_hz();
    let mut s = sound();
    let c = cloud(4, [40.0, 10.0, 0.0], 30);
    let mut pops = Vec::new();
    for tick in 1..past(&c, 2.0, hz) {
        let seen = if tick >= c.popped {
            vec![c.clone()]
        } else {
            vec![]
        };
        pops.extend(hear(&mut s, &publication(tick, &seen), hz, [0.0; 3]));
    }
    let sound = pop_sound(&c);
    let pops: Vec<_> = pops.iter().filter(|v| v.sound == sound).collect();
    assert_eq!(pops.len(), 1, "one pop for one cloud");
    assert_eq!(pops[0].position, Some(c.centre), "heard at the cloud");
}

/// The pops a side hears of `c` when it first sees it at `first_seen` and
/// keeps seeing it for two seconds.
fn heard_from(c: &SmokeCloud, first_seen: u64, hz: f64) -> Vec<Voice> {
    let mut s = sound();
    let mut voices = Vec::new();
    for tick in 1..first_seen + 2 * hz as u64 {
        let seen = if tick >= first_seen {
            vec![c.clone()]
        } else {
            vec![]
        };
        voices.extend(hear(&mut s, &publication(tick, &seen), hz, [0.0; 3]));
    }
    voices
}

#[test]
fn a_side_that_first_sees_a_cloud_after_its_bloom_never_hears_the_pop() {
    let hz = tick_hz();
    let c = cloud(4, [40.0, 10.0, 0.0], 30);
    let voices = heard_from(&c, past(&c, 1.0, hz), hz);
    assert!(
        voices.is_empty(),
        "a cloud seen only after its bloom is silent: {voices:?}"
    );
}

#[test]
fn a_cloud_first_seen_on_the_tick_it_reaches_full_size_is_silent() {
    let hz = tick_hz();
    let c = cloud(4, [40.0, 10.0, 0.0], 30);
    let full = (c.popped..)
        .find(|&t| {
            let s = contract::smoke::state(&c, t, hz as u32).unwrap();
            s.phase == c.spec.bloom_steps
        })
        .unwrap();
    let sound = pop_sound(&c);
    let pops = |first_seen| {
        (heard_from(&c, first_seen, hz).into_iter())
            .filter(|v| v.sound == sound)
            .count()
    };
    assert_eq!(pops(full - 1), 1, "seen on its last blooming tick, it pops");
    assert_eq!(pops(full), 0, "seen first at full size, it is silent");
}

#[test]
fn a_side_always_hears_its_own_units_pop() {
    let mut scenario = common::scenario(
        r#"{"size":[500,500],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([{"side":"blue","kind":"test_smoke_tank","position":[100,100],
                "engagement":"return_fire_only"}]),
        json!([]),
    );
    scenario.scripts.clear();
    let mut battle = Battle::new(&scenario, 1);
    let hz = f64::from(battle.rules().tick_hz);
    let runtime =
        presentation::assets::Runtime::open(&sim::fixtures::dir().join("../assets/runtime"))
            .unwrap();
    let rules = serde_json::to_value(battle.rules()).unwrap();
    let catalog = battle.rules().catalog.view();
    let fielded = presentation::fielded::Fielding {
        presentation: &fixture("presentation.json"),
        rules: &rules,
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &["test_smoke_tank".into()],
        factions: None,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap();
    let mut reader = Reader::new(&sim::publication::layout_json(&battle)).unwrap();
    let mut publisher = Publisher::new();
    let mut feed = EffectFeed::new("blue");
    let mut s = sound();
    let mut heard = Vec::new();
    let mut clouds = Vec::new();
    for tick in 0..(3.0 * hz) as u64 {
        if tick == 5 {
            let ack = battle.accept(contract::command::CommandEnvelope {
                side: contract::ids::Side::Blue,
                player: None,
                seq: 1,
                order: contract::command::Order::PopSmoke {
                    units: vec![contract::ids::UnitId(0)],
                },
                queued: false,
            });
            assert_eq!(ack.error, None);
        }
        battle.step();
        let words: Vec<u32> = (publisher.publish(&battle, contract::ids::Side::Blue))
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect();
        let observation = reader.decode(&words).unwrap().unwrap();
        let effects = feed.publication(&observation, &fielded.units, &fielded.effects, None);
        clouds.extend(effects.smoke_clouds.iter().cloned());
        heard.extend(hear(&mut s, &effects, hz, [100.0, 100.0, 0.0]));
    }
    let own = clouds.first().expect("the side sees its own cloud").clone();
    let sound = pop_sound(&own);
    let pops: Vec<_> = heard.iter().filter(|v| v.sound == sound).collect();
    assert_eq!(pops.len(), 1, "its own pop, once");
    assert_eq!(pops[0].position, Some(own.centre), "heard at its cloud");
}

/// The sound tables' refusal of the shipped presentation with its first
/// smoke look's row changed by `edit`, and that look's name.
fn refusal(edit: impl Fn(&mut Value)) -> (String, String) {
    let game = fixture("rules.json");
    let mut presentation = fixture("presentation.json");
    let looks = presentation["smoke_screen_looks"].as_object_mut().unwrap();
    let look = looks.keys().next().unwrap().clone();
    edit(&mut looks[&look]);
    let error = presentation::sound::tables::SoundTables::from_parts(
        fixture("audio.json"),
        fixture("sounds.json"),
        &presentation,
        &game["weapons"],
    )
    .expect_err("the look's pop is refused");
    (error, look)
}

#[test]
fn a_pop_naming_a_sound_the_library_lacks_is_refused_before_a_battle() {
    let (error, look) = refusal(|row| row["sound"] = json!("no-such-sound"));
    assert!(
        error.contains(&format!("smoke_screen_looks.{look}.sound")),
        "{error}"
    );
}

#[test]
fn a_look_without_a_pop_sound_is_refused_before_a_battle() {
    let (error, _) = refusal(|row| {
        row.as_object_mut().unwrap().remove("sound");
    });
    assert!(error.contains("sound"), "{error}");
}
