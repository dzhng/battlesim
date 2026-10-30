//! Which voice lines play, and when: the one place line limits are enforced.
//! A battle and the sound workbench's chatter preview ask it alike, and a
//! player only carries out the start commands it gives on the voices bus:
//!
//! - responses beat own barks, which beat others' barks; within a tier the
//!   request nearest the listener speaks first, and one without a position
//!   (an owner's answer, over the radio) is as near as can be;
//! - at most `lines` lines sound at once;
//! - a request that can't start within `wait_s` of its moment is dropped;
//! - a speaker never talks over itself;
//! - requests sharing a group key within `group_s` collapse to the first, so
//!   one unit answers for a group (the caller orders them: the clicked unit,
//!   else the highest value);
//! - a line whose take hasn't loaded is dropped at once.
//!
//! It is a pure function of what it is told: time is the player's clock, the
//! takes are those the player has loaded, and the take is chosen by the
//! client's own seeded randomness (the browser's, [`crate::seed`]), never the
//! simulation's.
//!
//! In a battle ([`Chatter::hear`]) it also takes the barks the battle's sound
//! heard (an own unit's shout, or an identified enemy's, in its kind's set:
//! [`Speakers::bark`]), and a bark it starts is spoken where the shout was,
//! placed as every unit sound is; an answer stays dry and centred.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::frame::{Command, Shout, Sound, Voice};
use super::tables::Bus;
use crate::seed::Key;
use crate::world::Vec3;

/// A line plays dry: no low-pass short of the top of hearing.
const OPEN_HZ: f64 = 20_000.0;
/// The takes a line may be recorded in.
const TAKES: [u32; 2] = [0, 1];

/// `audio.voices`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatterRules {
    /// Lines sounding at once.
    pub lines: usize,
    /// Seconds a request may wait to start before it is dropped.
    pub wait_s: f64,
    /// Seconds within which requests sharing a group key collapse to the first.
    pub group_s: f64,
    /// A line's level on the voices bus, before the calibration every take gets.
    pub gain: f64,
}

impl ChatterRules {
    pub fn check(&self) -> Result<(), String> {
        if self.lines >= 1 && self.wait_s >= 0.0 && self.group_s >= 0.0 && self.gain > 0.0 {
            Ok(())
        } else {
            Err(
                "audio.voices: lines a whole number from 1, wait_s and group_s ≥ 0, gain > 0"
                    .into(),
            )
        }
    }
}

/// Who hears a line first: the owner's answers, then own units' shouts, then
/// others'. Declared in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Response,
    OwnBark,
    OtherBark,
}

/// What a set's voice says: the owner's answers, battlefield shouts, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speaks {
    Responses,
    Barks,
    All,
}

/// One of a loaded set's voices.
#[derive(Clone, Debug, Deserialize)]
pub struct SetVoice {
    pub id: String,
    pub speaks: Speaks,
}

/// Something a unit wants to say.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub tier: Tier,
    /// A stable name for the unit speaking: it never overlaps itself, and it
    /// keeps its voice.
    pub speaker: String,
    /// The voice set it speaks with.
    pub set: String,
    pub trigger: String,
    pub intensity: String,
    /// Where it speaks; without one it is as near as can be.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec3>,
    /// Requests sharing this key within `group_s` are one answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    /// The moment it was asked for, on the player's clock.
    pub at: f64,
}

/// A line started: its start command's id, the voice and take chosen, and
/// when it starts and for how long, on the player's clock.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Started {
    pub id: u32,
    pub request: Request,
    pub voice: String,
    pub take: u32,
    pub at: f64,
    pub duration: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DropReason {
    Group,
    Late,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Dropped {
    pub request: Request,
    pub reason: DropReason,
    /// When it was dropped, on the player's clock.
    pub at: f64,
}

/// What one arbitration did: the commands a player carries out, the lines
/// started and dropped, and the barks it took from the battle's sound
/// ([`Chatter::hear`]; none otherwise), whether they play or not.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Lines {
    pub commands: Vec<Command>,
    pub started: Vec<Started>,
    pub dropped: Vec<Dropped>,
    pub heard: Vec<Request>,
}

/// Which voice set each unit kind speaks with (`voices.json`'s assignments,
/// which the player resolves): for the side's own faction, and for its
/// enemy's. A kind without one is silent.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Speakers {
    #[serde(default)]
    pub own: HashMap<String, String>,
    #[serde(default)]
    pub enemy: HashMap<String, String>,
}

impl Speakers {
    /// The bark `shout` asks for at player time `at`: an own unit's as an
    /// own bark, an identified enemy's as another's, in the set its kind
    /// speaks with for its side, where it was shouted. None for a kind
    /// without a set, or a tag that names no bark.
    pub fn bark(&self, shout: &Shout, at: f64) -> Option<Request> {
        let (tier, sets, side) = if shout.own {
            (Tier::OwnBark, &self.own, "own")
        } else {
            (Tier::OtherBark, &self.enemy, "seen")
        };
        Some(Request {
            tier,
            // An own unit is the speaker its answers name, so it never
            // shouts over its own answer.
            speaker: format!("{side}/{}", shout.id),
            set: sets.get(&*shout.kind)?.clone(),
            trigger: bark_name(&shout.trigger)?.to_owned(),
            intensity: shout.intensity.to_string(),
            position: Some(shout.position),
            group_key: None,
            at,
        })
    }
}

/// A bark's voice line name from the contract's tag: `b5_under_fire` is
/// `under_fire`.
fn bark_name(tag: &str) -> Option<&str> {
    let (number, name) = tag.split_once('_')?;
    number.strip_prefix('b')?.parse::<u8>().ok()?;
    Some(name)
}

struct Pending {
    request: Request,
    /// Arrival order: the last tie-break.
    order: u64,
    /// This speaker's requests for this trigger and intensity before it: keys the take.
    count: u32,
}

/// The sound a line's takes are the variants of: `set/voice/trigger/intensity`.
pub fn line_sound(set: &str, voice: &str, trigger: &str, intensity: &str) -> String {
    format!("{set}/{voice}/{trigger}/{intensity}")
}

pub struct Chatter {
    rules: ChatterRules,
    seed: f64,
    /// Each loaded set's voices.
    voices: HashMap<String, Vec<SetVoice>>,
    /// Each loaded take's length, seconds, by `line_sound` and take.
    takes: HashMap<(String, u32), f64>,
    pending: Vec<Pending>,
    /// Each sounding speaker and when its line ends.
    sounding: Vec<(String, f64)>,
    /// When the last line started ends: lines play out through a reset.
    until: f64,
    /// Each group key's first request moment.
    groups: HashMap<String, f64>,
    /// Requests so far per speaker, trigger and intensity.
    counts: HashMap<String, u32>,
    dropped: Vec<Dropped>,
    arrivals: u64,
    next_id: u32,
    out: Lines,
}

impl Chatter {
    /// Lines by `rules`, their takes chosen by `seed`.
    pub fn new(rules: ChatterRules, seed: f64) -> Self {
        Self {
            rules,
            seed,
            voices: HashMap::new(),
            takes: HashMap::new(),
            pending: Vec::new(),
            sounding: Vec::new(),
            until: f64::NEG_INFINITY,
            groups: HashMap::new(),
            counts: HashMap::new(),
            dropped: Vec::new(),
            arrivals: 0,
            next_id: 1,
            out: Lines::default(),
        }
    }

    /// Set `set`'s pack arrived: these are its voices.
    pub fn set_voices(&mut self, set: &str, voices: Vec<SetVoice>) {
        self.voices.insert(set.to_owned(), voices);
    }

    /// A take is loaded and lasts `duration` seconds at rate 1.
    pub fn loaded(
        &mut self,
        set: &str,
        voice: &str,
        trigger: &str,
        intensity: &str,
        take: u32,
        duration: f64,
    ) {
        self.takes
            .insert((line_sound(set, voice, trigger, intensity), take), duration);
    }

    /// Queues a request at player time `now`; the next `update` decides
    /// whether and when it plays.
    pub fn request(&mut self, request: Request, now: f64) {
        if let Some(key) = &request.group_key {
            if let Some(first) = self.groups.get(key) {
                if request.at - first <= self.rules.group_s {
                    self.dropped.push(Dropped {
                        request,
                        reason: DropReason::Group,
                        at: now,
                    });
                    return;
                }
            }
            self.groups.insert(key.clone(), request.at);
        }
        let key = format!(
            "{}/{}/{}",
            request.speaker, request.trigger, request.intensity
        );
        let count = self.counts.entry(key).or_insert(0);
        let pending = Pending {
            request,
            order: self.arrivals,
            count: *count,
        };
        *count += 1;
        self.arrivals += 1;
        self.pending.push(pending);
    }

    /// Starts what may start at player time `now`, nearest `listener` first
    /// within a tier, and drops what is too late.
    pub fn update(&mut self, now: f64, listener: Vec3) -> &Lines {
        self.out = Lines {
            dropped: std::mem::take(&mut self.dropped),
            ..Lines::default()
        };
        self.sounding.retain(|(_, end)| *end > now);
        let group_s = self.rules.group_s;
        self.groups.retain(|_, first| now - *first <= group_s);
        let wait_s = self.rules.wait_s;
        let out = &mut self.out;
        self.pending.retain(|item| {
            if now - item.request.at <= wait_s {
                return true;
            }
            out.dropped.push(Dropped {
                request: item.request.clone(),
                reason: DropReason::Late,
                at: now,
            });
            false
        });

        // An owner's answer has no position: it is as near as can be.
        let range_sq = |r: &Request| {
            r.position.map_or(0.0, |p| {
                let (x, y, z) = (p[0] - listener[0], p[1] - listener[1], p[2] - listener[2]);
                x * x + y * y + z * z
            })
        };
        let mut ready: Vec<(usize, f64)> = (0..self.pending.len())
            .filter(|&i| self.pending[i].request.at <= now)
            .map(|i| (i, range_sq(&self.pending[i].request)))
            .collect();
        let float = |a: f64, b: f64| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
        ready.sort_by(|&(a, ra), &(b, rb)| {
            let (a, b) = (&self.pending[a], &self.pending[b]);
            a.request
                .tier
                .cmp(&b.request.tier)
                .then(float(ra, rb))
                .then(float(a.request.at, b.request.at))
                .then(a.order.cmp(&b.order))
        });

        let mut done = vec![false; self.pending.len()];
        for (i, _) in ready {
            if self.sounding.len() >= self.rules.lines {
                break;
            }
            let item = &self.pending[i];
            if self
                .sounding
                .iter()
                .any(|(s, _)| *s == item.request.speaker)
            {
                continue;
            }
            done[i] = true;
            let Some((voice, take, duration)) = self.line(item) else {
                self.out.dropped.push(Dropped {
                    request: item.request.clone(),
                    reason: DropReason::Missing,
                    at: now,
                });
                continue;
            };
            let r = &item.request;
            let id = self.next_id;
            self.next_id += 1;
            self.out.commands.push(Command::Start {
                id,
                voice: Voice {
                    sound: line_sound(&r.set, &voice, &r.trigger, &r.intensity),
                    variant: take,
                    bus: Bus::Voices,
                    at: now,
                    gain: self.rules.gain,
                    rate: 1.0,
                    lowpass: OPEN_HZ,
                    wet: 0.0,
                    attack: 0.0,
                    position: None,
                    pan: None,
                    looping: false,
                },
            });
            self.sounding.push((r.speaker.clone(), now + duration));
            self.until = self.until.max(now + duration);
            self.out.started.push(Started {
                id,
                request: r.clone(),
                voice,
                take,
                at: now,
                duration,
            });
        }
        let mut i = 0;
        self.pending.retain(|_| {
            i += 1;
            !done[i - 1]
        });
        &self.out
    }

    /// A battle's arbitration at player time `now`: takes the barks `sound`
    /// heard, then starts what may start heard from where `sound` last
    /// listened, each bark placed where it was shouted ([`Sound::place`]).
    pub fn hear(&mut self, sound: &mut Sound, now: f64) -> &Lines {
        let barks = sound.take_barks();
        for bark in &barks {
            self.request(bark.clone(), now);
        }
        self.update(now, sound.listener().position);
        let out = &mut self.out;
        for (command, line) in out.commands.iter_mut().zip(&out.started) {
            if let (Command::Start { voice, .. }, Some(at)) = (command, line.request.position) {
                sound.place(voice, at);
            }
        }
        out.heard = barks;
        &self.out
    }

    /// A line is sounding at player time `now` (the score dips under it).
    pub fn speaking(&self, now: f64) -> bool {
        now < self.until
    }

    /// Forgets everything waiting and sounding, as for a new battle.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.sounding.clear();
        self.groups.clear();
        self.counts.clear();
        self.dropped.clear();
    }

    /// The voice, take and length `item` speaks with, or none when no take is loaded.
    fn line(&self, item: &Pending) -> Option<(String, u32, f64)> {
        let r = &item.request;
        let voice = self.voice(r)?;
        let sound = line_sound(&r.set, &voice, &r.trigger, &r.intensity);
        let loaded: Vec<(u32, f64)> = TAKES
            .iter()
            .filter_map(|&t| self.takes.get(&(sound.clone(), t)).map(|&d| (t, d)))
            .collect();
        if loaded.is_empty() {
            return None;
        }
        let mut rng = Key::new()
            .num(self.seed)
            .text(&r.speaker)
            .text(&r.trigger)
            .text(&r.intensity)
            .num(f64::from(item.count))
            .rng();
        // `random.choice`: the math package's pick from a list.
        let n = loaded.len();
        let (take, duration) = loaded[(rng.sample() * n as f64).floor() as usize % n];
        Some((voice, take, duration))
    }

    /// Answers come from the set's answering voice; a speaker barks in one
    /// voice, by its name.
    fn voice(&self, r: &Request) -> Option<String> {
        let speaks = if r.tier == Tier::Response {
            Speaks::Responses
        } else {
            Speaks::Barks
        };
        let voices: Vec<&SetVoice> = self
            .voices
            .get(&r.set)?
            .iter()
            .filter(|v| v.speaks == speaks || v.speaks == Speaks::All)
            .collect();
        if voices.is_empty() {
            return None;
        }
        let pick = if r.tier == Tier::Response {
            0
        } else {
            Key::new().text(&r.speaker).seed() as usize % voices.len()
        };
        Some(voices[pick].id.clone())
    }
}
