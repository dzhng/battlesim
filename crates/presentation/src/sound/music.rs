//! The battle's score, decided once for both clients: which mood it plays
//! and which track comes next. A player only streams what it is told:
//!
//! - it opens on `tension` and moves to `combat` when one of the side's own
//!   units barks a fighting trigger (`combat_triggers`, B5–B9), and back
//!   after `calm_s` without one. Entering and leaving use different spans,
//!   so it never flickers;
//! - each mood draws its tracks from a shuffled bag: every track once per
//!   shuffle, never the same one twice in a row, the shuffle from the
//!   score's own seeded chance ([`crate::seed`]), never the simulation's;
//! - the first track fades in, and a change of mood crossfades
//!   ([`Command::MusicStop`] then [`Command::Music`], over `crossfade_s`);
//! - while a voice line sounds, the music dips by the duck depth
//!   ([`Command::Duck`]).
//!
//! Tracks are the sound catalog's logical ids (`music.moods`); which file a
//! track is, its format and where it lives, is the player's.
use serde::Deserialize;

use super::frame::{Command, Ducked};
use super::tables::Duck;
use super::voices::{Request, Tier};
use crate::seed::Mulberry32;

/// `audio.music`.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicRules {
    /// How far the music dips under a voice line.
    pub duck: Duck,
    /// A qualifying bark at most this old switches tension to combat.
    pub combat_window_s: f64,
    /// Combat returns to tension after this long without one.
    pub calm_s: f64,
    /// The crossfade between moods, and the first track's fade in.
    pub crossfade_s: f64,
    /// The own barks that mean the side is in a fight, by line name.
    pub combat_triggers: Vec<String>,
}

/// The sound catalog's battle moods (`music.moods`), each its track ids.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Moods {
    pub tension: Vec<String>,
    pub combat: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Tension,
    Combat,
}

pub struct Score {
    rules: MusicRules,
    /// Each mood's tracks, tension then combat.
    tracks: [Vec<String>; 2],
    random: Mulberry32,
    mood: Mood,
    /// The mood whose track plays; none before the first.
    playing: Option<Mood>,
    last_bark: f64,
    /// Each mood's tracks left in this shuffle, drawn from the end.
    bags: [Vec<String>; 2],
    /// Each mood's track drawn last.
    last: [Option<String>; 2],
    ducked: bool,
    out: Vec<Command>,
}

impl Score {
    /// A battle's score by `rules` over `moods`' tracks, shuffled by `seed`.
    pub fn new(rules: MusicRules, moods: Moods, seed: u32) -> Self {
        Self {
            rules,
            tracks: [moods.tension, moods.combat],
            random: Mulberry32::new(seed),
            mood: Mood::Tension,
            playing: None,
            last_bark: f64::NEG_INFINITY,
            bags: Default::default(),
            last: Default::default(),
            ducked: false,
            out: Vec::new(),
        }
    }

    /// The mood it plays.
    pub fn mood(&self) -> Mood {
        self.mood
    }

    /// At player time `now`: the barks the voice lines heard since the last
    /// call (only the side's own move the mood) and whether a line sounds.
    /// The commands the music bus carries out, in order.
    pub fn hear(&mut self, now: f64, barks: &[Request], speaking: bool) -> &[Command] {
        self.out.clear();
        let fighting = barks
            .iter()
            .any(|b| b.tier == Tier::OwnBark && self.rules.combat_triggers.contains(&b.trigger));
        if fighting {
            self.last_bark = self.last_bark.max(now);
        }
        let since = now - self.last_bark;
        if self.mood == Mood::Tension && since <= self.rules.combat_window_s {
            self.mood = Mood::Combat;
        } else if self.mood == Mood::Combat && since > self.rules.calm_s {
            self.mood = Mood::Tension;
        }
        if self.playing != Some(self.mood) {
            let fade = self.rules.crossfade_s;
            if let Some(track) = self.next(self.mood) {
                if self.playing.is_some() {
                    self.out.push(Command::MusicStop { at: now, fade });
                }
                self.out.push(Command::Music {
                    at: now,
                    track,
                    fade,
                });
            }
            self.playing = Some(self.mood);
        }
        if speaking != self.ducked {
            let duck = self.rules.duck;
            let depth = if speaking {
                duck.depth.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let glide = if depth > 0.0 {
                duck.attack_s
            } else {
                duck.release_s
            };
            self.out.push(Command::Duck {
                at: now,
                bus: Ducked::Music,
                depth,
                glide,
            });
            self.ducked = speaking;
        }
        &self.out
    }

    /// `mood`'s next track: each plays once per shuffle, never the same one
    /// twice in a row; none for a mood without tracks.
    fn next(&mut self, mood: Mood) -> Option<String> {
        let m = mood as usize;
        let bag = &mut self.bags[m];
        if bag.is_empty() {
            bag.extend(self.tracks[m].iter().cloned());
            // Fisher–Yates, from the end.
            for i in (1..bag.len()).rev() {
                let j = (self.random.sample() * (i + 1) as f64).floor() as usize;
                bag.swap(i, j);
            }
            // The bag is drawn from its end; keep the track just played away from it.
            let n = bag.len();
            if n > 1 && Some(&bag[n - 1]) == self.last[m].as_ref() {
                bag.swap(0, n - 1);
            }
        }
        let track = bag.pop();
        self.last[m].clone_from(&track);
        track
    }
}
