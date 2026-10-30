//! The battle's score: its mood follows the side's own barks, its tracks
//! shuffle, a change of mood crossfades, and it dips under voice lines.
//!
//! `fixtures/music-vectors.json` was recorded from the TypeScript score this
//! module replaced (`BattleMusic` over `MusicDirector`), its chance drawn
//! from `mulberry32` with each case's seed: each step a moment, the barks
//! heard (tier and trigger) and whether a line sounds, with what the score
//! asked of its player (`play` a track with a fade, `crossfade` to one,
//! `duck`). The cases are five seeds over a 20-minute battle of fights
//! coming and going with barks that don't count, the calm span's edges and
//! every combat trigger, a mood without tracks, and two tracks a mood.
use presentation::sound::frame::{Command, Ducked};
use presentation::sound::music::{Mood, Moods, MusicRules, Score};
use presentation::sound::voices::{Request, Tier};
use serde_json::Value;

fn rules() -> MusicRules {
    let game: Value = serde_json::from_str(include_str!("../../../fixtures/audio.json")).unwrap();
    serde_json::from_value(game["music"].clone()).unwrap()
}

fn bark(tier: Tier, trigger: &str, at: f64) -> Request {
    Request {
        tier,
        speaker: "own/1".into(),
        set: "s".into(),
        trigger: trigger.into(),
        intensity: "stressed".into(),
        position: Some([0.0; 3]),
        group_key: None,
        at,
    }
}

fn moods(tension: &[&str], combat: &[&str]) -> Moods {
    let own = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect();
    Moods {
        tension: own(tension),
        combat: own(combat),
    }
}

/// The tracks `commands` start.
fn started(commands: &[Command]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::Music { track, .. } => Some(track.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn every_decision_equals_the_recorded_vectors() {
    let v: Value = serde_json::from_str(include_str!("fixtures/music-vectors.json")).unwrap();
    let rules: MusicRules = serde_json::from_value(v["rules"].clone()).unwrap();
    let (mut steps, mut changes, mut ducks) = (0, 0, 0);
    for case in v["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let moods: Moods = serde_json::from_value(case["tracks"].clone()).unwrap();
        let seed = case["seed"].as_u64().unwrap() as u32;
        let mut score = Score::new(rules.clone(), moods, seed);
        for step in case["steps"].as_array().unwrap() {
            let t = step["t"].as_f64().unwrap();
            let barks: Vec<Request> = step
                .get("barks")
                .map_or(&Vec::new(), |b| b.as_array().unwrap())
                .iter()
                .map(|b| {
                    let tier = serde_json::from_value(b["tier"].clone()).unwrap();
                    bark(tier, b["trigger"].as_str().unwrap(), t)
                })
                .collect();
            let speaking = step.get("speaking").is_some_and(|s| s.as_bool().unwrap());
            // What the TypeScript score asked of its player, as commands.
            let mut want = Vec::new();
            for call in step
                .get("calls")
                .map_or(&Vec::new(), |c| c.as_array().unwrap())
            {
                let n = |k: &str| call[k].as_f64().unwrap();
                let track = || call["track"].as_str().unwrap().to_owned();
                match call["op"].as_str().unwrap() {
                    "play" => want.push(Command::Music {
                        at: t,
                        track: track(),
                        fade: n("fade"),
                    }),
                    "crossfade" => {
                        changes += 1;
                        want.push(Command::MusicStop {
                            at: t,
                            fade: n("fade"),
                        });
                        want.push(Command::Music {
                            at: t,
                            track: track(),
                            fade: n("fade"),
                        });
                    }
                    "duck" => {
                        ducks += 1;
                        let depth = n("depth");
                        want.push(Command::Duck {
                            at: t,
                            bus: Ducked::Music,
                            depth,
                            glide: if depth > 0.0 {
                                n("attack")
                            } else {
                                n("release")
                            },
                        });
                    }
                    op => panic!("{name}: unknown call {op}"),
                }
            }
            assert_eq!(
                score.hear(t, &barks, speaking),
                &want[..],
                "{name}, t = {t}"
            );
            steps += 1;
        }
    }
    // The vectors exercise what they claim to.
    assert!(
        steps > 15_000 && changes > 50 && ducks > 1_000,
        "{steps} {changes} {ducks}"
    );
}

#[test]
fn an_own_fighting_bark_brings_combat_and_the_calm_span_brings_tension_back() {
    let r = rules();
    let (window, calm) = (r.combat_window_s, r.calm_s);
    assert!(calm > window, "leaving takes longer than entering");
    let mut score = Score::new(r, moods(&["t"], &["c"]), 1);
    assert_eq!(started(score.hear(0.0, &[], false)), ["t"]);
    // An enemy's fighting bark, an own contact call and an answer leave it.
    let others = [
        bark(Tier::OtherBark, "under_fire", 1.0),
        bark(Tier::OwnBark, "contact_infantry", 1.0),
        bark(Tier::Response, "move", 1.0),
    ];
    assert!(score.hear(1.0, &others, false).is_empty());
    assert_eq!(score.mood(), Mood::Tension);
    assert_eq!(
        started(score.hear(10.0, &[bark(Tier::OwnBark, "pinned", 10.0)], false)),
        ["c"]
    );
    // A bark keeps combat; quiet for the calm span, it holds, just past it, it leaves.
    score.hear(30.0, &[bark(Tier::OwnBark, "hit", 30.0)], false);
    assert!(score.hear(30.0 + calm, &[], false).is_empty());
    assert_eq!(score.mood(), Mood::Combat);
    assert_eq!(started(score.hear(30.0 + calm + 0.1, &[], false)), ["t"]);
    assert_eq!(score.mood(), Mood::Tension);
}

#[test]
fn a_change_of_mood_crossfades_and_the_first_track_only_fades_in() {
    let r = rules();
    let fade = r.crossfade_s;
    let mut score = Score::new(r, moods(&["t"], &["c"]), 1);
    assert_eq!(
        score.hear(0.0, &[], false),
        [Command::Music {
            at: 0.0,
            track: "t".into(),
            fade
        }]
    );
    assert_eq!(
        score.hear(5.0, &[bark(Tier::OwnBark, "under_fire", 5.0)], false),
        [
            Command::MusicStop { at: 5.0, fade },
            Command::Music {
                at: 5.0,
                track: "c".into(),
                fade
            }
        ]
    );
}

#[test]
fn every_track_once_a_shuffle_and_never_twice_in_a_row() {
    let r = rules();
    let calm = r.calm_s;
    let tracks = ["c1", "c2", "c3"];
    for seed in 0..50 {
        let mut score = Score::new(r.clone(), moods(&["t"], &tracks), seed);
        let mut played = Vec::new();
        let mut t = 0.0;
        score.hear(t, &[], false);
        for _ in 0..30 {
            // A fight, then the calm span, then the next fight.
            t += 1.0;
            played.extend(started(score.hear(
                t,
                &[bark(Tier::OwnBark, "hit", t)],
                false,
            )));
            t += calm + 1.0;
            score.hear(t, &[], false);
        }
        assert_eq!(played.len(), 30);
        for pair in played.windows(2) {
            assert_ne!(pair[0], pair[1], "seed {seed}: {played:?}");
        }
        for shuffle in played.chunks(3) {
            let mut s = shuffle.to_vec();
            s.sort();
            assert_eq!(s, tracks, "seed {seed}: {played:?}");
        }
    }
}

#[test]
fn the_seed_decides_the_order() {
    let order = |seed| {
        let mut score = Score::new(rules(), moods(&["a", "b", "c", "d"], &[]), seed);
        // Each tension → combat → tension cycle draws one tension track.
        let mut t = 0.0;
        let mut played = started(score.hear(t, &[], false));
        for _ in 0..3 {
            t += 1.0;
            score.hear(t, &[bark(Tier::OwnBark, "hit", t)], false);
            t += 100.0;
            played.extend(started(score.hear(t, &[], false)));
        }
        played
    };
    assert_eq!(order(11), order(11));
    assert!((0..20).any(|s| order(s) != order(11)));
}

#[test]
fn a_mood_without_tracks_changes_the_mood_and_starts_nothing() {
    let mut score = Score::new(rules(), moods(&["t"], &[]), 1);
    score.hear(0.0, &[], false);
    assert!(score
        .hear(1.0, &[bark(Tier::OwnBark, "hit", 1.0)], false)
        .is_empty());
    assert_eq!(score.mood(), Mood::Combat);
}

#[test]
fn the_music_dips_while_a_line_sounds_and_recovers_after() {
    let r = rules();
    let duck = r.duck;
    let mut score = Score::new(r, moods(&["t"], &["c"]), 1);
    score.hear(0.0, &[], false);
    assert_eq!(
        score.hear(0.1, &[], true),
        [Command::Duck {
            at: 0.1,
            bus: Ducked::Music,
            depth: duck.depth,
            glide: duck.attack_s
        }]
    );
    assert!(score.hear(0.2, &[], true).is_empty());
    assert_eq!(
        score.hear(1.5, &[], false),
        [Command::Duck {
            at: 1.5,
            bus: Ducked::Music,
            depth: 0.0,
            glide: duck.release_s
        }]
    );
}
