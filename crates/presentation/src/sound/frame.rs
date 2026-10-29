//! The battle's sound: the one owner of what is heard, and when. It reads
//! exactly the feed the visuals read:
//!
//! - the publication as the effects take it ([`Heard`]): gunfire at each
//!   launch (the muzzle flashes' own), near or far by distance, a recorded
//!   burst sounding the gun's next rounds on its cadence; a ricochet whine
//!   at each glance; an impact by hit kind; a blast; a hull cooking off,
//!   booming with each fireball and clanging where its turret lands; a motor
//!   on a flying guided round; fire on every smoke source the side knows
//!   (a wreck), burning then smouldering; and positional flare releases;
//! - the frame's emitters ([`Motion`]): each drawn vehicle's engine (idle to
//!   load), running gear, turret traverse and back-up beeper, and each drawn
//!   soldier's footsteps by the distance he walks;
//! - the observation's hearing cues: an unseen enemy is heard only as a
//!   vague sound from the cue's direction sector, louder in the near band and
//!   muffled in the far one, never at a position; a shout as a muffled bark
//!   from the enemy faction's generic pool ([`Sound::set_shouts`]), never
//!   the speaker's own voice;
//! - the observation's shouts: an own unit's, or an enemy's the side
//!   identifies, asks the voice lines ([`super::voices::Chatter`]) for a
//!   bark in its kind's set ([`Sound::set_speakers`]), spoken where the
//!   shout was and placed as every unit sound is ([`Sound::place`]); incoming
//!   missile shouts also raise the positional warning cue.
//!
//! So sound keeps fog of war as the picture does: every positional sound has
//! a cause in the side's observation.
//!
//! Timing: an event starts when the presentation clock reaches the tick it
//! happened in, so what is heard lines up with what is drawn. A clock that
//! stands still (a pause) is a hold: transients fall silent and none start,
//! loops hold. A clock that jumps drops what is too late to play. A
//! publication earlier than the last, or the clock running back, is a new
//! battle.
//!
//! Voices: a budget with loops capped apart; each voice's priority is its
//! level at the listener, so a louder new sound takes the quietest voice's
//! place and a quieter one is dropped. A sound loud at the listener (a near
//! cannon, a blast) dips the ambience bus, which recovers once the dip is
//! reached ([`Command::Duck`], `audio.ambience`), so the bed
//! makes room for it as a film mix does; distant fire never reaches it.
//!
//! The player only carries out the [`Command`]s: it starts, glides and
//! stops voices, and places them by direction (distance is decided here).
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sim::publication::reader::{Shout as PublishedShout, SoundCue};

use super::tables::{pick, Bus, SoundTables, VehicleSound};
use super::voices::{Request, Speakers};
use crate::effects::cook_off::CookOffFeel;
use crate::effects::launches::Launch;
use crate::effects::publication::EffectPublication;
use crate::records::{Emitter, NONE};
use crate::seed::Key;
use crate::world::Vec3;

/// Pending transients held at most; past it the oldest go.
const PENDING_CAP: usize = 1024;
/// How far ahead of the clock a transient is handed to the player, seconds.
const LOOKAHEAD_S: f64 = 0.05;
/// Seconds a voice fades out over when stopped: a hold, a steal, a lost loop.
const FADE_HOLD_S: f64 = 0.03;
const FADE_STEAL_S: f64 = 0.02;
const FADE_LOOP_S: f64 = 0.15;

/// One publication as sound hears it: what the effects read of it, the
/// launches it shows, the side's hearing cues and the shouts it hears in
/// the open.
pub struct Heard<'a> {
    pub effects: &'a EffectPublication,
    pub launches: &'a [Launch],
    pub audible: Vec<Cue>,
    pub shouts: Vec<Shout>,
}

/// A shout the side hears in the open: its own unit's, or an enemy's it
/// identifies (an unseen enemy's reaches it only as a hearing cue). Who
/// said it (`own`, and the unit id or identification handle), the
/// speaker's unit kind, where, the bark (the contract's tag,
/// `b5_under_fire`) and how stressed.
#[derive(Clone, Debug, Deserialize)]
pub struct Shout {
    pub own: bool,
    pub id: f64,
    pub kind: Arc<str>,
    pub position: Vec3,
    pub trigger: Arc<str>,
    pub intensity: Arc<str>,
}

impl From<&PublishedShout> for Shout {
    fn from(s: &PublishedShout) -> Self {
        Self {
            own: s.own,
            id: s.id,
            kind: s.kind.clone(),
            position: s.position,
            trigger: s.trigger.clone(),
            intensity: s.intensity.clone(),
        }
    }
}

/// A hearing cue: an unseen source's category, its direction sector from the
/// listening unit (0 east, counter-clockwise in 45° steps), its band
/// (`near` or `far`), and whether it moved.
#[derive(Clone, Debug, Deserialize)]
pub struct Cue {
    pub category: Arc<str>,
    pub sector: f64,
    pub band: Arc<str>,
    pub moving: bool,
}

impl From<&SoundCue> for Cue {
    fn from(c: &SoundCue) -> Self {
        Self {
            category: c.category.clone(),
            sector: c.sector,
            band: c.band.clone(),
            moving: c.moving,
        }
    }
}

/// A drawn vehicle, as posed this frame.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundVehicle {
    /// Unique across both sides (the effects' shooter key).
    pub key: f64,
    /// Its `audio.vehicles` row.
    pub vehicle_class: String,
    pub position: Vec3,
    /// Metres the left and right running gear have rolled.
    pub travel_l: f64,
    pub travel_r: f64,
    /// Turret heading relative to the hull, radians.
    pub turret: f64,
    #[serde(default)]
    pub reverse: bool,
}

/// A drawn soldier, as posed this frame.
#[derive(Clone, Debug, Deserialize)]
pub struct SoundSoldier {
    pub id: f64,
    pub position: Vec3,
}

/// What the side draws moving this frame.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Motion {
    pub vehicles: Vec<SoundVehicle>,
    pub soldiers: Vec<SoundSoldier>,
}

impl Motion {
    /// The frame's emitters, each vehicle as its class (`class(emitter.class)`,
    /// `default` when it has none), keyed apart by side as the effects key them.
    pub fn of<'a>(emitters: &[Emitter], class: impl Fn(u32) -> Option<&'a str>) -> Self {
        let mut motion = Self::default();
        for e in emitters {
            let position = [0, 1, 2].map(|i| f64::from(e.position[i]));
            if e.member != NONE {
                motion.soldiers.push(SoundSoldier {
                    id: f64::from(e.member),
                    position,
                });
                continue;
            }
            motion.vehicles.push(SoundVehicle {
                key: f64::from(e.unit) * 2.0 + f64::from(e.flags & 1),
                vehicle_class: class(e.class).unwrap_or("default").to_owned(),
                position,
                travel_l: f64::from(e.motion[0]),
                travel_r: f64::from(e.motion[1]),
                turret: f64::from(e.motion[2]),
                reverse: e.flags & 2 != 0,
            });
        }
        motion
    }
}

/// The listener: where it stands and which way it looks (level).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listener {
    pub position: Vec3,
    pub forward: Vec3,
}

impl Listener {
    /// Where the listener stands for a camera at `eye` looking at `target`
    /// with `up`: `share` of the way from the target to the eye, facing the
    /// way the camera looks; looking straight down, the way its top points.
    pub fn of_camera(eye: Vec3, target: Vec3, up: [f32; 3], share: f64) -> Self {
        let position = [0, 1, 2].map(|i| target[i] + share * (eye[i] - target[i]));
        let mut forward = [target[0] - eye[0], target[1] - eye[1], 0.0];
        if (forward[0] * forward[0] + forward[1] * forward[1]).sqrt() < 1e-6 {
            forward = [f64::from(up[0]), f64::from(up[1]), 0.0];
        }
        let squared = forward[0] * forward[0] + forward[1] * forward[1];
        if squared > 0.0 {
            let inverse = 1.0 / squared.sqrt();
            forward = [forward[0] * inverse, forward[1] * inverse, 0.0];
        }
        Self { position, forward }
    }
}

/// A voice to start. `position` none is not positional: `pan` places it (a
/// cue's direction), or it plays centred (the bed).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Voice {
    pub sound: String,
    /// Stable event-derived variation: which of the recipe's alternatives.
    pub variant: u32,
    pub bus: Bus,
    /// Player time to start at, seconds.
    pub at: f64,
    pub gain: f64,
    pub rate: f64,
    /// Low-pass cutoff, Hz.
    pub lowpass: f64,
    /// Reverb send, a share of the dry level.
    pub wet: f64,
    /// Seconds the onset ramps up over (0: as recorded).
    pub attack: f64,
    pub position: Option<Vec3>,
    pub pan: Option<f64>,
    #[serde(rename = "loop")]
    pub looping: bool,
}

/// A running loop's live parameters.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Params {
    pub gain: f64,
    pub rate: f64,
    pub lowpass: f64,
    pub wet: f64,
    pub position: Option<Vec3>,
}

/// What the player does, in order.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Command {
    Start {
        id: u32,
        #[serde(flatten)]
        voice: Voice,
    },
    /// Glide loop `id` to `params` from `at`.
    Set {
        id: u32,
        at: f64,
        #[serde(flatten)]
        params: Params,
    },
    /// Fade voice `id` out over `fade` seconds from `at`, then free it.
    Stop { id: u32, at: f64, fade: f64 },
    /// Hear from `listener` from `at`.
    Listen {
        at: f64,
        #[serde(flatten)]
        listener: Listener,
    },
    /// Stream music track `track` (a catalog id, whose file the player
    /// resolves) on the music bus from `at`, looping, fading in over `fade`
    /// seconds. A crossfade is a [`Command::MusicStop`] then this, over the
    /// same span.
    Music { at: f64, track: String, fade: f64 },
    /// Fade every music track sounding out over `fade` seconds from `at`,
    /// then free it.
    #[serde(rename = "music_stop")]
    MusicStop { at: f64, fade: f64 },
    /// Dip `bus` by `depth`, a share of its level (0 restores it), from
    /// `at`, gliding there with time constant `glide` seconds.
    Duck {
        at: f64,
        bus: Ducked,
        depth: f64,
        glide: f64,
    },
}

/// A bus a [`Command::Duck`] dips: the score's under a voice line
/// ([`super::music`]), or the ambience under a loud sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ducked {
    Music,
    Ambience,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Stats {
    /// Voices sounding now.
    pub transients: usize,
    pub loops: usize,
    /// Transients waiting for the clock.
    pub pending: usize,
    /// Started, dropped for the budget, and stolen by louder ones, since the last reset.
    pub started: usize,
    pub dropped: usize,
    pub stolen: usize,
    /// Positional transients and cue sounds started since the last reset.
    pub positional: usize,
    pub cues: usize,
    /// The clock stands still.
    pub held: bool,
    /// The last publication taken.
    pub tick: f64,
}

/// A transient waiting for the clock.
struct Pending {
    t: f64,
    sound: String,
    variant: u32,
    /// The far sound, and the distance it takes over at.
    far: Option<String>,
    far_m: f64,
    bus: Bus,
    gain: f64,
    rate: f64,
    position: Option<Vec3>,
    /// A cue's direction sector, its low-pass, reverb send and far share
    /// (its onset's softening).
    sector: f64,
    lowpass: f64,
    wet: f64,
    far_share: f64,
}

#[derive(Clone, Copy)]
struct Sounding {
    id: u32,
    priority: f64,
    /// A transient's end on the player's clock; infinite for a loop.
    end: f64,
}

/// What a loop sounds for: one of a vehicle's layers (by its key's bits
/// and slot), a fire (by its smoke source), a flying round's motor (by where
/// its stretch ends), or the bed.
#[derive(Clone, PartialEq)]
enum LoopKey {
    Vehicle(u64, &'static str),
    Fire(String),
    Motor([u64; 3]),
    Ambience,
}

struct LoopWant {
    key: LoopKey,
    sound: String,
    bus: Bus,
    gain: f64,
    rate: f64,
    position: Option<Vec3>,
    priority: f64,
}

struct Mover {
    travel: f64,
    turret: f64,
    speed: f64,
    traverse: f64,
}

struct Stride {
    x: f64,
    y: f64,
    carry: f64,
}

struct Fire {
    at: Vec3,
    kind: String,
    start: f64,
}

/// A gun's sounding burst recording: when it started, and the rounds it covers.
struct BurstVoice {
    start: f64,
    shots: usize,
    interval: f64,
    heard: usize,
}

struct Motor {
    kind: Arc<str>,
    from: Vec3,
    to: Vec3,
    t0: f64,
}

/// A burning source's times (`presentation.effects.smoke`).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SmokeTimes {
    pub burn_s: f64,
    pub smoulder_s: f64,
}

/// What, besides its tables, the sound reads of the battle: its tick rate,
/// and when the effects burn and cook off (`presentation.effects`).
#[derive(Clone, Debug, Deserialize)]
pub struct Timing {
    pub tick_hz: f64,
    /// How long each smoke kind burns and smoulders.
    pub smoke: HashMap<String, SmokeTimes>,
    /// When a cook-off's fireballs go and its turret lands.
    pub cook_off: CookOffFeel,
}

/// One take of the enemy's shout pool: the sound a player plays it as, and
/// its length at rate 1, seconds.
#[derive(Clone, Debug, Deserialize)]
pub struct ShoutTake {
    pub sound: String,
    pub duration: f64,
}

/// A battle's sound as a player builds it, JSON: `audio.json`, the
/// sound catalog, the game's weapon rows, the tick rate and
/// `presentation.effects`' smoke and cook-off.
#[derive(Deserialize)]
struct Setup {
    audio: serde_json::Value,
    catalog: serde_json::Value,
    weapons: serde_json::Value,
    tick_hz: f64,
    effects: Timings,
}

#[derive(Deserialize)]
struct Timings {
    smoke: HashMap<String, SmokeTimes>,
    cook_off: CookOffFeel,
}

/// A publication as a script or a capture gives it (the browser's
/// `SoundPublication`): what the effects read, the launches by where they
/// leave, and the side's hearing cues.
#[derive(Deserialize)]
pub struct Scripted {
    effects: EffectPublication,
    launches: Vec<ScriptedShot>,
    #[serde(default)]
    audible: Vec<Cue>,
    #[serde(default)]
    shouts: Vec<Shout>,
}

#[derive(Deserialize)]
struct ScriptedShot {
    shooter: f64,
    mount: usize,
    soldier: Option<f64>,
    x: f64,
    y: f64,
    z: f64,
    kind: Arc<str>,
}

impl Scripted {
    /// Its launches, as the effects derive them.
    pub fn launches(&self) -> Vec<Launch> {
        self.launches
            .iter()
            .map(|l| Launch {
                shooter: l.shooter,
                mount: l.mount,
                soldier: l.soldier,
                at: [l.x, l.y, l.z],
                dir: [0.0; 3],
                kind: l.kind.clone(),
                hull: l.soldier.is_none(),
            })
            .collect()
    }

    /// The publication as sound hears it, with its `launches`.
    pub fn heard<'a>(&'a self, launches: &'a [Launch]) -> Heard<'a> {
        Heard {
            effects: &self.effects,
            launches,
            audible: self.audible.clone(),
            shouts: self.shouts.clone(),
        }
    }
}

/// An insertion-ordered map: the order loops and fires were first wanted
/// is the order they are stopped and ranked in.
struct Ordered<K, T>(Vec<(K, T)>);

impl<K: PartialEq, T> Ordered<K, T> {
    fn get(&self, key: &K) -> Option<&T> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    fn get_mut(&mut self, key: &K) -> Option<&mut T> {
        self.0.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    fn remove(&mut self, key: &K) -> Option<T> {
        let i = self.0.iter().position(|(k, _)| k == key)?;
        Some(self.0.remove(i).1)
    }
    /// Sets `key`, keeping its place if it was already there.
    fn set(&mut self, key: K, value: T) {
        match self.get_mut(&key) {
            Some(v) => *v = value,
            None => self.0.push((key, value)),
        }
    }
}

impl<K, T> Default for Ordered<K, T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct Sound {
    tables: Arc<SoundTables>,
    timing: Timing,
    dt: f64,
    /// Each prepared sound's alternatives' lengths at rate 1, seconds.
    durations: HashMap<String, Vec<f64>>,
    pending: Vec<Pending>,
    transients: Vec<Sounding>,
    loops: Ordered<LoopKey, Sounding>,
    movers: HashMap<u64, Mover>,
    strides: HashMap<u64, Stride>,
    fires: Ordered<String, Fire>,
    motors: Ordered<[u64; 3], Motor>,
    bursts: HashMap<(u64, usize, Option<u64>), BurstVoice>,
    cue_heard: HashMap<(String, u64, Arc<str>), f64>,
    /// The enemy faction's shout pool, as far as the player has loaded it.
    shouts: Vec<ShoutTake>,
    /// Which set each unit kind speaks with, own and enemy.
    speakers: Speakers,
    /// Barks the publications asked for since the voice lines last took them.
    barks: Vec<Request>,
    next_id: u32,
    last_tick: f64,
    clock: Option<f64>,
    advanced_at: f64,
    held: bool,
    listener: Listener,
    stats: Stats,
    out: Vec<Command>,
    /// HighAir ids currently present; the transition from empty to non-empty
    /// is one air-alarm episode for this side.
    high_air: BTreeSet<u64>,
    /// While the ambience is dipped: the player time its recovery starts.
    ambience_dipped: Option<f64>,
}

/// Where a flying round's stretch ends, as a loop's key.
fn end_key(p: &Vec3) -> [u64; 3] {
    p.map(f64::to_bits)
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Stereo position of a direction sector for a listener looking along
/// `forward`: 0 east, counter-clockwise in 45° steps.
pub fn sector_pan(sector: f64, forward: Vec3) -> f64 {
    let a = sector * std::f64::consts::PI / 4.0;
    let len = libm::hypot(forward[0], forward[1]);
    let len = if len == 0.0 { 1.0 } else { len };
    // Screen-right: forward × up.
    let (rx, ry) = (forward[1] / len, -forward[0] / len);
    (libm::cos(a) * rx + libm::sin(a) * ry).clamp(-1.0, 1.0)
}

/// One of a vehicle's sounding loops.
#[derive(Serialize)]
pub struct VehicleLoop<'a> {
    pub slot: &'static str,
    pub sound: &'a str,
    pub gain: f64,
    pub rate: f64,
}

/// The loops a vehicle of `row` sounds: its engine from idle to full `load`
/// (a share of `full_speed_mps`), running gear rising with it, a turret's
/// whine by `traverse` (a share of `full_traverse_rps`) and, `reversing`, the
/// back-up beeper, steady whatever the speed, as an alarm is. A layer below
/// audibility is left out.
pub fn vehicle_loops(
    row: &VehicleSound,
    load: f64,
    traverse: f64,
    reversing: bool,
) -> Vec<VehicleLoop<'_>> {
    let mut loops = vec![VehicleLoop {
        slot: "engine",
        sound: &row.engine,
        gain: lerp(row.idle_gain, row.load_gain, load),
        rate: lerp(row.idle_rate, row.load_rate, load),
    }];
    if let Some(running) = row.running.as_deref().filter(|_| load > 0.02) {
        loops.push(VehicleLoop {
            slot: "running",
            sound: running,
            gain: row.running_gain * load.sqrt(),
            rate: 0.5 + load,
        });
    }
    if let Some(turret) = row.turret.as_deref().filter(|_| traverse > 0.05) {
        loops.push(VehicleLoop {
            slot: "turret",
            sound: turret,
            gain: row.turret_gain * traverse,
            rate: 0.9 + 0.2 * traverse,
        });
    }
    if let Some(reverse) = row.reverse.as_deref().filter(|_| reversing && load > 0.02) {
        loops.push(VehicleLoop {
            slot: "reverse",
            sound: reverse,
            gain: row.reverse_gain,
            rate: 1.0,
        });
    }
    loops
}

/// The share of a sound's gain heard `d` metres off: full within `ref_m`,
/// then inverse distance easing down to `floor`, and nothing past `max_m`.
pub fn distance_gain(tables: &SoundTables, d: f64) -> f64 {
    let p = tables.audio.distance;
    if d > p.max_m {
        return 0.0;
    }
    let inverse = p.ref_m / (p.ref_m + p.rolloff * (d - p.ref_m).max(0.0));
    p.floor + (1.0 - p.floor) * inverse
}

impl Sound {
    /// A battle's sound from its setup JSON (`Setup`).
    pub fn from_json(json: &str) -> Result<Self, String> {
        let setup: Setup = serde_json::from_str(json).map_err(|e| format!("sound: {e}"))?;
        let tables = SoundTables::from_parts(setup.audio, setup.catalog, &setup.weapons)?;
        setup.effects.cook_off.validate()?;
        Ok(Self::new(
            Arc::new(tables),
            Timing {
                tick_hz: setup.tick_hz,
                smoke: setup.effects.smoke,
                cook_off: setup.effects.cook_off,
            },
        ))
    }

    pub fn new(tables: Arc<SoundTables>, timing: Timing) -> Self {
        let dt = 1.0 / timing.tick_hz;
        Self {
            tables,
            timing,
            dt,
            durations: HashMap::new(),
            pending: Vec::new(),
            transients: Vec::new(),
            loops: Ordered::default(),
            movers: HashMap::new(),
            strides: HashMap::new(),
            fires: Ordered::default(),
            motors: Ordered::default(),
            bursts: HashMap::new(),
            cue_heard: HashMap::new(),
            shouts: Vec::new(),
            speakers: Speakers::default(),
            barks: Vec::new(),
            next_id: 1,
            last_tick: -1.0,
            clock: None,
            advanced_at: 0.0,
            held: false,
            listener: Listener {
                position: [0.0; 3],
                forward: [0.0, 1.0, 0.0],
            },
            stats: Stats::default(),
            out: Vec::new(),
            high_air: BTreeSet::new(),
            ambience_dipped: None,
        }
    }

    pub fn tables(&self) -> &SoundTables {
        &self.tables
    }

    /// The player's prepared sounds' lengths at rate 1 (seconds), by sound,
    /// one per alternative; errs naming any sound a battle plays that is missing.
    pub fn set_durations(&mut self, durations: HashMap<String, Vec<f64>>) -> Result<(), String> {
        let missing: Vec<_> = self
            .tables
            .battle_sounds()
            .into_iter()
            .filter(|s| durations.get(s).is_none_or(Vec::is_empty))
            .collect();
        if !missing.is_empty() {
            return Err(format!("sound: no length for {}", missing.join(", ")));
        }
        self.durations = durations;
        Ok(())
    }

    /// The takes an unseen enemy's shout is drawn from: stressed barks of
    /// the listener's enemy faction, whichever the player has loaded (none:
    /// shouts are silent). A battle keeps its pool through a reset.
    pub fn set_shouts(&mut self, takes: Vec<ShoutTake>) {
        self.shouts = takes;
    }

    /// Which set each unit kind speaks with, for the side's own faction and
    /// its enemy's: a shout of a kind without one is silent. A battle keeps
    /// them through a reset.
    pub fn set_speakers(&mut self, speakers: Speakers) {
        self.speakers = speakers;
    }

    /// The barks heard since the last call, for the voice lines to arbitrate.
    pub fn take_barks(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.barks)
    }

    /// Places a voice line spoken at `position` as every unit sound is
    /// placed: its level by distance, and the air's low-pass, reverb send
    /// and softened onset by how far off it is.
    pub fn place(&self, voice: &mut Voice, position: Vec3) {
        let air = self.tables.audio.air;
        let u = self.far_share(&position);
        voice.gain = self.level(voice.gain, Some(&position));
        voice.lowpass = self.air_lowpass(u);
        voice.wet = air.wet * u;
        voice.attack = air.attack_s * u;
        voice.position = Some(position);
        voice.pan = None;
    }

    fn duration(&self, sound: &str, variant: u32) -> f64 {
        match self.durations.get(sound) {
            Some(d) => d[variant as usize % d.len()],
            None => self
                .shouts
                .iter()
                .find(|t| t.sound == sound)
                .map_or(0.0, |t| t.duration),
        }
    }

    /// Where the last update heard from (the origin, looking north, before any).
    pub fn listener(&self) -> Listener {
        self.listener
    }

    pub fn stats(&self) -> Stats {
        Stats {
            transients: self.transients.len(),
            loops: self.loops.0.len(),
            pending: self.pending.len(),
            held: self.held,
            tick: self.last_tick,
            ..self.stats
        }
    }

    /// Silences and forgets everything (a new battle), from player time `now`.
    pub fn reset(&mut self, now: f64) -> &[Command] {
        self.out.clear();
        self.clear(now);
        &self.out
    }

    fn clear(&mut self, now: f64) {
        self.recover_ambience(now, true);
        for v in self
            .transients
            .iter()
            .chain(self.loops.0.iter().map(|(_, v)| v))
        {
            self.out.push(Command::Stop {
                id: v.id,
                at: now,
                fade: FADE_HOLD_S,
            });
        }
        self.transients.clear();
        self.loops.0.clear();
        self.pending.clear();
        self.movers.clear();
        self.strides.clear();
        self.fires.0.clear();
        self.motors.0.clear();
        self.bursts.clear();
        self.cue_heard.clear();
        self.barks.clear();
        self.high_air.clear();
        self.last_tick = -1.0;
        self.clock = None;
        self.held = false;
        self.stats = Stats::default();
    }

    /// Takes one publication: what it shows happened becomes transients at
    /// their presentation times, and its smoke sources and flying rounds the
    /// fire and motor loops. The same tick again is ignored.
    pub fn note(&mut self, heard: &Heard, now: f64) -> &[Command] {
        self.out.clear();
        let fx = heard.effects;
        if fx.tick == self.last_tick {
            return &self.out;
        }
        if fx.tick < self.last_tick {
            self.clear(now);
        }
        self.last_tick = fx.tick;
        let t0 = (fx.tick - 1.0) * self.dt;
        let t1 = fx.tick * self.dt;
        let tables = self.tables.clone();
        let p = &tables.audio;

        self.bursts
            .retain(|_, b| t0 <= b.start + b.shots as f64 * b.interval);
        for l in heard.launches {
            let s = tables.shot(&l.kind);
            // A burst recording sounds the gun's next rounds too, while they
            // land on its shots; a round off its cadence, or past its last,
            // starts another.
            let gun = (l.shooter.to_bits(), l.mount, l.soldier.map(f64::to_bits));
            if let Some(b) = self.bursts.get_mut(&gun) {
                if b.heard < b.shots
                    && (t0 - b.start - b.heard as f64 * b.interval).abs() <= b.interval / 2.0
                {
                    b.heard += 1;
                    continue;
                }
            }
            let burst = tables.cadence(&s.near);
            if burst.shots > 1 {
                self.bursts.insert(
                    gun,
                    BurstVoice {
                        start: t0,
                        shots: burst.shots,
                        interval: burst.interval_s,
                        heard: 1,
                    },
                );
            } else {
                self.bursts.remove(&gun);
            }
            let variety = Key::new().num(l.shooter).text(&l.kind);
            let key = Key::new()
                .num(fx.tick)
                .num(l.shooter)
                .num(l.mount as f64)
                .num(l.soldier.unwrap_or(-1.0))
                .text(&l.kind);
            self.queue(
                t0,
                s.near,
                Some(s.far),
                s.far_m,
                Bus::Effects,
                s.gain,
                l.at,
                variety,
                key,
            );
        }
        for (i, point) in fx.flare_deploys.iter().enumerate() {
            let flare = &p.flare_deploy;
            let key = Key::new().text("flare").num(i as f64).nums(point);
            self.queue(
                t0,
                flare.sound.clone(),
                None,
                0.0,
                Bus::Effects,
                flare.gain,
                *point,
                key,
                key,
            );
        }
        let mut motors = Ordered::default();
        for s in &fx.segments {
            if s.path.len() < 2 {
                continue;
            }
            // Ricochets at their place along the tick's stretch.
            if !s.ricochets.is_empty() {
                let mut total = 0.0;
                let mut cum = vec![0.0];
                for w in s.path.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    total += crate::world::length([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
                    cum.push(total);
                }
                for r in &s.ricochets {
                    let at = t0 + (self.dt * cum[r.point]) / total.max(1e-6);
                    let pt = s.path[r.point];
                    let sound = tables
                        .contact(super::GLANCE, &s.kind, &p.ricochet.sound)
                        .to_owned();
                    let gain = p.ricochet.gain * pick(&p.impact_scale, &s.kind);
                    let key = Key::new().nums(&pt);
                    self.queue(at, sound, None, 0.0, Bus::Effects, gain, pt, key, key);
                }
            }
            let end = *s.path.last().expect("two points");
            if &*s.hit != "none" {
                let hit = pick(&p.impacts, &s.hit);
                let scale = pick(&p.impact_scale, &s.kind);
                let sound = tables.contact(&s.hit, &s.kind, &hit.sound).to_owned();
                let key = Key::new().nums(&end);
                self.queue(
                    t1,
                    sound,
                    None,
                    0.0,
                    Bus::Effects,
                    hit.gain * scale,
                    end,
                    key,
                    key,
                );
            } else if p.motors.contains_key(&*s.kind) {
                // A flying round with a motor: one loop along its chain of
                // stretches, keyed by where its latest stretch ends; a stretch
                // starting where one ended continues it, and its loop follows.
                motors.set(
                    end_key(&end),
                    Motor {
                        kind: s.kind.clone(),
                        from: s.path[0],
                        to: end,
                        t0,
                    },
                );
                let old = LoopKey::Motor(end_key(&s.path[0]));
                if let Some(v) = self.loops.remove(&old) {
                    self.loops.set(LoopKey::Motor(end_key(&end)), v);
                }
            }
        }
        self.motors = motors;
        for b in &fx.blasts {
            let s = pick(&p.blasts, &b.kind);
            let key = Key::new().nums(&b.point);
            self.queue(
                t1,
                tables.effect(&s.near).to_owned(),
                Some(tables.effect(&s.far).to_owned()),
                s.far_m,
                Bus::Effects,
                s.gain,
                b.point,
                key,
                key,
            );
        }
        // A cook-off: a boom with each fireball, the biggest loudest, and the
        // clang of its turret striking the deck, as the effects time them.
        let cook = &self.timing.cook_off;
        let turret_lands_s = cook.impact_after();
        let biggest = cook
            .fireballs
            .iter()
            .fold(f64::NEG_INFINITY, |m, f| m.max(f.radius_m));
        let booms: Vec<(f64, f64)> = cook
            .fireballs
            .iter()
            .map(|f| (cook.delay_s + f.after_s, f.radius_m / biggest))
            .collect();
        let blast = &p.cook_off.blast;
        let landing = &p.cook_off.landing;
        for c in &fx.cook_offs {
            let ring = [c.center[0], c.center[1], c.center[2] + c.height];
            for (i, (after, share)) in booms.iter().enumerate() {
                let key = Key::new().text("cook_off").num(i as f64).nums(&ring);
                self.queue(
                    t0 + after,
                    tables.effect(&blast.near).to_owned(),
                    Some(tables.effect(&blast.far).to_owned()),
                    blast.far_m,
                    Bus::Effects,
                    blast.gain * share,
                    ring,
                    key,
                    key,
                );
            }
            if let Some(at) = c.landing {
                let baseline = &pick(&p.impacts, &landing.hit).sound;
                let sound = tables.contact(&landing.hit, &landing.round, baseline);
                let key = Key::new().text("cook_off_landing").nums(&at);
                self.queue(
                    t0 + turret_lands_s,
                    sound.to_owned(),
                    None,
                    0.0,
                    Bus::Effects,
                    landing.gain,
                    at,
                    key,
                    key,
                );
            }
        }
        // Fires: every smoke source the side knows, from when it was first known.
        for k in &fx.smokes {
            if self.fires.get(&k.key).is_none() {
                self.fires.0.push((
                    k.key.clone(),
                    Fire {
                        at: [k.center[0], k.center[1], k.center[2] + k.half[2]],
                        kind: k.kind.clone(),
                        start: t0,
                    },
                ));
            }
        }
        self.fires
            .0
            .retain(|(key, _)| fx.smokes.iter().any(|k| &k.key == key));

        // Unseen enemies: one vague sound per distinct cue, not repeated
        // within `repeat_s`, placed by direction and never at a position. A
        // shout is a bark from the enemy's pool, muffled in either band; the
        // cue says nothing of who shouted, so neither does the take.
        let cues = &p.cues;
        for c in &heard.audible {
            let category = &*c.category;
            let shout = category == "shout";
            if shout && self.shouts.is_empty() {
                continue;
            }
            let name = format!("{category}{}", if c.moving { "_moving" } else { "" });
            let sound = if shout {
                None
            } else {
                match cues.sounds.get(&name).or_else(|| cues.sounds.get(category)) {
                    Some(sound) => Some(sound),
                    None => continue,
                }
            };
            let key = (name.clone(), c.sector.to_bits(), c.band.clone());
            if let Some(&last) = self.cue_heard.get(&key) {
                if t0 - last < cues.repeat_s && t0 >= last {
                    continue;
                }
            }
            let near = &*c.band == "near";
            let band_gain = if near { cues.near_gain } else { cues.far_gain };
            let cue = match sound {
                Some(sound) => Pending {
                    t: t0,
                    sound: tables.effect(sound).to_owned(),
                    variant: 0,
                    far: None,
                    far_m: 0.0,
                    bus: Bus::Effects,
                    gain: band_gain,
                    rate: 1.0,
                    position: None,
                    sector: c.sector,
                    lowpass: if near {
                        p.air.near_hz
                    } else {
                        cues.far_lowpass_hz
                    },
                    wet: if near { 0.0 } else { p.air.wet },
                    far_share: if near { 0.0 } else { 1.0 },
                },
                None => {
                    // Which take: seeded by the cue and its time, so the
                    // same battle hears the same takes.
                    let pick = Key::new()
                        .text(&key.0)
                        .num(c.sector)
                        .text(&c.band)
                        .num(t0)
                        .unit()
                        * self.shouts.len() as f64;
                    Pending {
                        t: t0,
                        sound: self.shouts[pick as usize].sound.clone(),
                        variant: 0,
                        far: None,
                        far_m: 0.0,
                        bus: Bus::Voices,
                        gain: cues.shout.gain * band_gain,
                        rate: 1.0,
                        position: None,
                        sector: c.sector,
                        lowpass: cues.shout.lowpass_hz,
                        wet: cues.shout.wet,
                        far_share: 1.0,
                    }
                }
            };
            self.cue_heard.insert(key, t0);
            self.pending.push(cue);
        }
        // Shouts heard in the open: each a bark asked of the voice lines,
        // for when the clock reaches its tick, as the transients wait; one
        // too far off to be heard is never asked.
        let at = now + self.clock.map_or(0.0, |c| (t0 - c).max(0.0));
        for s in &heard.shouts {
            if &*s.trigger == "b15_incoming_missile" {
                let warning = &p.missile_warning;
                let key = Key::new().text("missile_warning").num(s.id);
                self.queue(
                    t0,
                    warning.sound.clone(),
                    None,
                    0.0,
                    Bus::Effects,
                    warning.gain,
                    s.position,
                    key,
                    key,
                );
            }
            let Some(bark) = self.speakers.bark(s, at) else {
                continue;
            };
            if self.level(p.voices.gain, Some(&s.position)) >= p.distance.cull {
                self.barks.push(bark);
            }
        }
        if self.pending.len() > PENDING_CAP {
            self.pending.drain(..self.pending.len() - PENDING_CAP);
        }
        &self.out
    }

    #[allow(clippy::too_many_arguments)]
    fn queue(
        &mut self,
        t: f64,
        sound: String,
        far: Option<String>,
        far_m: f64,
        bus: Bus,
        gain: f64,
        position: Vec3,
        variety: Key,
        variant: Key,
    ) {
        let variant = variant.num(t).nums(&position).seed();
        self.pending.push(Pending {
            t,
            sound,
            variant,
            far,
            far_m,
            bus,
            gain,
            rate: 0.94 + 0.12 * variety.unit(),
            position: Some(position),
            sector: -1.0,
            lowpass: 0.0,
            wet: 0.0,
            far_share: 0.0,
        });
    }

    fn distance(&self, position: &Vec3) -> f64 {
        let l = self.listener.position;
        crate::world::length([position[0] - l[0], position[1] - l[1], position[2] - l[2]])
    }

    /// Level at the listener for a sound of `gain` at `position`.
    fn level(&self, gain: f64, position: Option<&Vec3>) -> f64 {
        position.map_or(gain, |p| {
            gain * distance_gain(&self.tables, self.distance(p))
        })
    }

    /// How far `position` sounds: 0 at the listener, 1 at `air.far_m` and beyond.
    fn far_share(&self, position: &Vec3) -> f64 {
        clamp01(self.distance(position) / self.tables.audio.air.far_m)
    }

    /// The air's low-pass cutoff at far share `u`, log-spaced.
    fn air_lowpass(&self, u: f64) -> f64 {
        let air = self.tables.audio.air;
        air.near_hz * libm::pow(air.far_hz / air.near_hz, u)
    }

    /// Advances to presentation time `clock` at wall time `wall` and player
    /// time `now` (seconds): starts what is due, footsteps and loops from
    /// `motion`, heard at `listener`.
    pub fn update(
        &mut self,
        clock: f64,
        wall: f64,
        now: f64,
        motion: &Motion,
        listener: Listener,
    ) -> &[Command] {
        self.out.clear();
        self.listener = listener;
        self.out.push(Command::Listen { at: now, listener });
        // The clock running backwards is a new battle.
        if self.clock.is_some_and(|c| clock < c - 1e-9) {
            self.clear(now);
        }
        if self.clock != Some(clock) {
            self.advanced_at = wall;
        }
        let step = self.clock.map_or(0.0, |c| (clock - c).max(0.0));
        self.clock = Some(clock);
        let held = wall - self.advanced_at > self.tables.audio.hold_s;
        if held && !self.held {
            for v in self.transients.drain(..) {
                self.out.push(Command::Stop {
                    id: v.id,
                    at: now,
                    fade: FADE_HOLD_S,
                });
            }
        }
        self.held = held;
        self.transients.retain(|v| v.end > now);
        self.recover_ambience(now, held);
        if held {
            return &self.out; // loops hold as they are
        }
        self.footsteps(motion, clock, step);
        self.flush(clock, now);
        self.update_loops(clock, now, motion, step);
        &self.out
    }

    /// Dips the ambience under a sound starting at `at`, `level` loud at
    /// the listener on `bus`, if it is loud enough; a dip under way is held
    /// until this one is reached too. A sound never dips its own bus.
    fn duck_ambience(&mut self, at: f64, level: f64, bus: Bus) {
        let a = &self.tables.audio.ambience;
        if bus == Bus::Ambience || level < a.duck_over {
            return;
        }
        let recover = at + a.duck.attack_s;
        if self.ambience_dipped.is_none() {
            self.out.push(Command::Duck {
                at,
                bus: Ducked::Ambience,
                depth: a.duck.depth.clamp(0.0, 1.0),
                glide: a.duck.attack_s,
            });
        }
        self.ambience_dipped = Some(self.ambience_dipped.map_or(recover, |t| t.max(recover)));
    }

    /// Lets a dipped ambience recover from `now` once its dip is reached, or
    /// at once when `now_anyway` (a hold, a new battle).
    fn recover_ambience(&mut self, now: f64, now_anyway: bool) {
        let Some(t) = self.ambience_dipped else {
            return;
        };
        if now < t && !now_anyway {
            return;
        }
        self.ambience_dipped = None;
        self.out.push(Command::Duck {
            at: now,
            bus: Ducked::Ambience,
            depth: 0.0,
            glide: self.tables.audio.ambience.duck.release_s,
        });
    }

    /// A footstep each `stride_m` a soldier walks; a jump past `max_step_m`
    /// (seen again elsewhere) starts his count afresh.
    fn footsteps(&mut self, motion: &Motion, clock: f64, step: f64) {
        let tables = self.tables.clone();
        let f = &tables.audio.footsteps;
        let mut seen = Vec::with_capacity(motion.soldiers.len());
        for s in &motion.soldiers {
            let id = s.id.to_bits();
            seen.push(id);
            let [x, y, _] = s.position;
            let Some(st) = self.strides.get_mut(&id) else {
                let carry = Key::new().text("step").num(s.id).unit() * f.stride_m;
                self.strides.insert(id, Stride { x, y, carry });
                continue;
            };
            let d = libm::hypot(x - st.x, y - st.y);
            st.x = x;
            st.y = y;
            if step <= 0.0 || d > f.max_step_m {
                continue;
            }
            st.carry += d;
            if st.carry < f.stride_m {
                continue;
            }
            st.carry %= f.stride_m;
            if self.level(f.gain, Some(&s.position)) < tables.audio.distance.cull {
                continue;
            }
            let variety = Key::new().num(s.id).num(clock);
            self.queue(
                clock,
                tables.effect(&f.sound).to_owned(),
                None,
                0.0,
                Bus::Units,
                f.gain,
                s.position,
                variety,
                variety,
            );
        }
        seen.sort_unstable();
        self.strides.retain(|id, _| seen.binary_search(id).is_ok());
    }

    /// Starts every pending transient due by `clock`, loudest first, within the budget.
    fn flush(&mut self, clock: f64, now: f64) {
        let late_s = self.tables.audio.late_s;
        let (later, due): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending)
            .into_iter()
            .filter(|e| e.t > clock + LOOKAHEAD_S || e.t >= clock - late_s)
            .partition(|e| e.t > clock + LOOKAHEAD_S);
        self.pending = later;
        if due.is_empty() {
            return;
        }
        let budget = self.tables.audio.budget;
        let cap = budget.voices - budget.loops;
        let cull = self.tables.audio.distance.cull;
        let mut ranked: Vec<(Pending, f64)> = due
            .into_iter()
            .map(|e| {
                let priority = self.level(e.gain, e.position.as_ref());
                (e, priority)
            })
            .filter(|(_, priority)| *priority >= cull)
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (e, priority) in ranked {
            if self.transients.len() >= cap {
                let mut low = 0;
                for i in 1..self.transients.len() {
                    if self.transients[i].priority < self.transients[low].priority {
                        low = i;
                    }
                }
                if self.transients[low].priority >= priority {
                    self.stats.dropped += 1;
                    continue;
                }
                let stolen = self.transients.remove(low);
                self.out.push(Command::Stop {
                    id: stolen.id,
                    at: now,
                    fade: FADE_STEAL_S,
                });
                self.stats.stolen += 1;
            }
            let far =
                matches!((&e.far, &e.position), (Some(_), Some(p)) if self.distance(p) > e.far_m);
            let sound = if far { e.far.expect("far") } else { e.sound };
            let at = now + (e.t - clock).max(0.0);
            let id = self.next_id;
            self.next_id += 1;
            let u = e
                .position
                .as_ref()
                .map_or(e.far_share, |p| self.far_share(p));
            let air = self.tables.audio.air;
            let end = at + self.duration(&sound, e.variant) / e.rate;
            self.duck_ambience(at, priority, e.bus);
            self.out.push(Command::Start {
                id,
                voice: Voice {
                    sound,
                    variant: e.variant,
                    bus: e.bus,
                    at,
                    gain: if e.position.is_some() {
                        priority
                    } else {
                        e.gain
                    },
                    rate: e.rate,
                    lowpass: if e.position.is_some() {
                        self.air_lowpass(u)
                    } else {
                        e.lowpass
                    },
                    wet: if e.position.is_some() {
                        air.wet * u
                    } else {
                        e.wet
                    },
                    attack: air.attack_s * u,
                    position: e.position,
                    pan: e
                        .position
                        .is_none()
                        .then(|| sector_pan(e.sector, self.listener.forward)),
                    looping: false,
                },
            });
            self.transients.push(Sounding { id, priority, end });
            self.stats.started += 1;
            if e.position.is_some() {
                self.stats.positional += 1;
            } else {
                self.stats.cues += 1;
            }
        }
    }

    /// Every loop wanted now: the bed, engines and running gear, turrets,
    /// reverse, fires and motors; the loudest within the loop budget sound.
    fn update_loops(&mut self, clock: f64, now: f64, motion: &Motion, step: f64) {
        let tables = self.tables.clone();
        let p = &tables.audio;
        let mut wants: Vec<LoopWant> = Vec::new();
        // The catalog derives a vehicle class from the type's altitude layer,
        // so `high_air_` names the layer (`contract::catalog::UnitType::vehicle_class`).
        let current_high_air: BTreeSet<u64> = motion
            .vehicles
            .iter()
            .filter(|v| v.vehicle_class.starts_with("high_air_"))
            .map(|v| v.key.to_bits())
            .collect();
        if self.high_air.is_empty() && !current_high_air.is_empty() {
            let alarm = &p.air_alarm;
            self.pending.push(Pending {
                t: clock,
                sound: tables.effect(&alarm.sound).to_owned(),
                variant: 0,
                far: None,
                far_m: 0.0,
                bus: Bus::Ambience,
                gain: alarm.gain,
                rate: 1.0,
                position: None,
                sector: -1.0,
                lowpass: p.air.near_hz,
                wet: 0.0,
                far_share: 0.0,
            });
        }
        self.high_air = current_high_air;
        let mut seen = Vec::with_capacity(motion.vehicles.len());
        for v in &motion.vehicles {
            let key = v.key.to_bits();
            seen.push(key);
            let row = pick(&p.vehicles, &v.vehicle_class);
            let travel = (v.travel_l.abs() + v.travel_r.abs()) / 2.0;
            let m = self.movers.entry(key).or_insert(Mover {
                travel,
                turret: v.turret,
                speed: 0.0,
                traverse: 0.0,
            });
            if step > 0.0 {
                // Rolled distance and traverse over the frame, eased (about 0.25 s).
                let speed = (travel - m.travel).abs() / step;
                let turn = (v.turret - m.turret).abs() / step;
                let k = (step / 0.25).min(1.0);
                m.speed += (speed.min(60.0) - m.speed) * k;
                m.traverse += (turn.min(10.0) - m.traverse) * k;
            }
            m.travel = travel;
            m.turret = v.turret;
            let load = clamp01(m.speed / row.full_speed_mps);
            let traverse = clamp01(m.traverse / row.full_traverse_rps);
            for layer in vehicle_loops(row, load, traverse, v.reverse) {
                wants.push(LoopWant {
                    key: LoopKey::Vehicle(key, layer.slot),
                    sound: layer.sound.to_owned(),
                    bus: Bus::Units,
                    gain: layer.gain,
                    rate: layer.rate,
                    position: Some(v.position),
                    priority: 0.0,
                });
            }
        }
        seen.sort_unstable();
        self.movers.retain(|key, _| seen.binary_search(key).is_ok());
        for (key, f) in &self.fires.0 {
            let Some(times) = self
                .timing
                .smoke
                .get(&f.kind)
                .or_else(|| self.timing.smoke.get("default"))
            else {
                continue;
            };
            let row = pick(&p.fires, &f.kind);
            let age = clock - f.start;
            if age > times.burn_s + times.smoulder_s {
                continue;
            }
            // Full while it burns, easing to the smoulder over its last tenth.
            let burn = clamp01((times.burn_s - age) / (times.burn_s * 0.1));
            wants.push(LoopWant {
                key: LoopKey::Fire(key.clone()),
                sound: row.sound.clone(),
                bus: Bus::Ambience,
                gain: lerp(row.smoulder_gain, row.gain, burn),
                rate: 1.0,
                position: Some(f.at),
                priority: 0.0,
            });
        }
        for (key, m) in &self.motors.0 {
            let u = clamp01((clock - m.t0) / self.dt);
            let at = [0, 1, 2].map(|i| m.from[i] + u * (m.to[i] - m.from[i]));
            let row = &p.motors[&*m.kind];
            wants.push(LoopWant {
                key: LoopKey::Motor(*key),
                sound: row.sound.clone(),
                bus: Bus::Effects,
                gain: row.gain,
                rate: 1.0,
                position: Some(at),
                priority: 0.0,
            });
        }
        for w in &mut wants {
            w.priority = self.level(w.gain, w.position.as_ref());
        }
        let mut kept: Vec<LoopWant> = wants
            .into_iter()
            .filter(|w| w.priority >= p.distance.cull)
            .collect();
        kept.sort_by(|a, b| b.priority.total_cmp(&a.priority));
        kept.truncate(p.budget.loops);
        // The bed is always on and outside the budget.
        kept.push(LoopWant {
            key: LoopKey::Ambience,
            sound: p.ambience.sound.clone(),
            bus: Bus::Ambience,
            gain: p.ambience.gain,
            rate: 1.0,
            position: None,
            priority: f64::INFINITY,
        });
        let loops = std::mem::take(&mut self.loops.0);
        for (key, v) in loops {
            if kept.iter().any(|w| w.key == key) {
                self.loops.0.push((key, v));
            } else {
                self.out.push(Command::Stop {
                    id: v.id,
                    at: now,
                    fade: FADE_LOOP_S,
                });
            }
        }
        for w in kept {
            let u = w.position.as_ref().map_or(0.0, |p| self.far_share(p));
            let params = Params {
                gain: if w.position.is_some() {
                    w.priority
                } else {
                    w.gain
                },
                rate: w.rate,
                lowpass: self.air_lowpass(u),
                wet: p.air.wet * u,
                position: w.position,
            };
            if let Some(v) = self.loops.get_mut(&w.key) {
                v.priority = w.priority;
                let id = v.id;
                self.out.push(Command::Set {
                    id,
                    at: now,
                    params,
                });
                continue;
            }
            let id = self.next_id;
            self.next_id += 1;
            self.out.push(Command::Start {
                id,
                voice: Voice {
                    sound: tables.effect(&w.sound).to_owned(),
                    variant: 0,
                    bus: w.bus,
                    at: now,
                    gain: params.gain,
                    rate: params.rate,
                    lowpass: params.lowpass,
                    wet: params.wet,
                    attack: 0.0,
                    position: params.position,
                    pan: None,
                    looping: true,
                },
            });
            self.loops.0.push((
                w.key,
                Sounding {
                    id,
                    priority: w.priority,
                    end: f64::INFINITY,
                },
            ));
        }
    }
}
