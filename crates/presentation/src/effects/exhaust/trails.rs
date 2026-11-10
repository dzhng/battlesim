//! The trails jets leave, laid tick by tick where the side's publications
//! put each aircraft, as smoke behind a smoking hull is: so a trail is the
//! same however often frames are drawn, and a replay framed at any moment
//! after its publications were taken shows the trail it would have had.
//!
//! Where on the aircraft each trail starts comes from its drawn model: each
//! emitter's offset from where the model stands, in its heading's frame and
//! leaning as last drawn. A tick lays a sample per emitter there, while its
//! engines run (it was drawn running within `max_gap_s`); a jet first drawn
//! after its history was taken (a replay framed mid-flight) lays its whole
//! history at once. A gap in the publications or a jump past `break_mps`
//! starts a new stretch; a downed or vanished jet lays nothing more and its
//! trail fades out. At most `max_aircraft` aircraft are tracked, each holding no
//! more than its longest trail's life; spent tracks keep their room.
use std::collections::VecDeque;

use super::{distance, DrawnJet, Emitter, JetExhaustStyle};
use crate::effects::publication::Shooter;
use crate::world::Vec3;

/// Where an aircraft was published at a tick, and whether that follows on
/// from the tick before.
#[derive(Clone, Copy, Debug)]
struct Body {
    t: f64,
    at: Vec3,
    yaw: f64,
    joined: bool,
}

/// A point of one emitter's trail: when it was laid, where, how strong, and
/// whether it joins the point before it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Sample {
    pub t: f64,
    pub p: Vec3,
    pub strength: f64,
    pub joined: bool,
}

/// One aircraft: its published history and, once drawn, its jet.
#[derive(Clone, Debug, Default)]
pub(super) struct Track {
    pub key: u64,
    published: bool,
    bodies: VecDeque<Body>,
    /// The clock it was last drawn running at.
    pub drawn: Option<f64>,
    pub engine: String,
    cruise_mps: f64,
    pub aft: Vec3,
    load: f64,
    pub kinds: Vec<Emitter>,
    /// Each emitter's offset from where the model stands, in its heading's
    /// frame (+x along it), leaning as last drawn.
    local: Vec<Vec3>,
    /// Each emitter where it is drawn now.
    pub heads: Vec<Vec3>,
    pub rings: Vec<VecDeque<Sample>>,
}

impl Track {
    /// Its published speed over the last `window` s of unbroken history.
    fn speed(&self, window: f64) -> Option<f64> {
        let newest = self.bodies.back()?;
        let mut oldest = newest;
        for b in self.bodies.iter().rev().skip(1) {
            if newest.t - b.t > window {
                break;
            }
            oldest = b;
            if !b.joined {
                break;
            }
        }
        (newest.t > oldest.t).then(|| distance(newest.at, oldest.at) / (newest.t - oldest.t))
    }

    /// Its throttle: speed over cruise, and how far into its afterburner.
    pub fn heat(&self, style: &JetExhaustStyle) -> (f64, f64) {
        let cruise = self.cruise_mps.max(1e-6);
        let share = self.speed(style.speed_window_s).unwrap_or(cruise) / cruise;
        let [from, full] = style.reheat_share;
        (
            share.clamp(0.0, 1.0),
            ((share - from) / (full - from)).clamp(0.0, 1.0),
        )
    }

    /// Lay a sample of every emitter where `body` puts it.
    fn lay(&mut self, body: &Body, style: &JetExhaustStyle) {
        let (heat, _) = self.heat(style);
        let [from, full] = style.vapour_load;
        let vapour = super::smooth((self.load - from) / (full - from));
        let (s, c) = libm::sincos(body.yaw);
        for ((ring, kind), d) in self.rings.iter_mut().zip(&self.kinds).zip(&self.local) {
            let joined = body.joined && ring.back().is_some_and(|last| last.t < body.t);
            ring.push_back(Sample {
                t: body.t,
                p: [
                    body.at[0] + d[0] * c - d[1] * s,
                    body.at[1] + d[0] * s + d[1] * c,
                    body.at[2] + d[2],
                ],
                strength: match kind {
                    Emitter::Nozzle { .. } => heat,
                    Emitter::Wingtip => vapour,
                },
                joined,
            });
        }
    }
}

/// Every aircraft's trails, and the jets drawn now.
#[derive(Debug, Default)]
pub struct JetTrails {
    pub(super) tracks: Vec<Track>,
    spent: Vec<Track>,
}

impl JetTrails {
    /// Forget every trail (a new battle).
    pub fn reset(&mut self) {
        self.spent.append(&mut self.tracks);
    }

    /// Samples every trail holds now, and the published poses kept.
    pub fn samples(&self) -> usize {
        self.tracks
            .iter()
            .map(|t| t.bodies.len() + t.rings.iter().map(VecDeque::len).sum::<usize>())
            .sum()
    }

    /// The track keyed `key`, made if there is room.
    fn track(&mut self, key: u64, max: usize) -> Option<&mut Track> {
        let i = match self.tracks.iter().position(|t| t.key == key) {
            Some(i) => i,
            None if self.tracks.len() < max => {
                let mut t = self.spent.pop().unwrap_or_default();
                t.key = key;
                t.drawn = None;
                t.bodies.clear();
                t.kinds.clear();
                t.rings.clear();
                self.tracks.push(t);
                self.tracks.len() - 1
            }
            None => return None,
        };
        Some(&mut self.tracks[i])
    }

    /// Take a tick's publication, at presentation time `t` (`dt` after the
    /// tick before; `gap`: ticks were missed): every airborne unit's pose,
    /// and a sample of every running jet's trails there.
    pub fn note(
        &mut self,
        style: &JetExhaustStyle,
        t: f64,
        dt: f64,
        gap: bool,
        shooters: &[Shooter],
    ) {
        for track in &mut self.tracks {
            track.published = false;
        }
        for s in shooters.iter().filter(|s| s.airborne) {
            let Some(track) = self.track(s.key.to_bits(), style.max_aircraft) else {
                continue;
            };
            track.published = true;
            let joined = !gap
                && track.bodies.back().is_some_and(|b| {
                    (t - dt - b.t).abs() < dt * 0.5
                        && distance(b.at, s.position) <= style.break_mps * dt + 1.0
                });
            let body = Body {
                t,
                at: s.position,
                yaw: s.yaw,
                joined,
            };
            track.bodies.push_back(body);
            if track.drawn.is_some_and(|d| t - d <= style.max_gap_s) {
                track.lay(&body, style);
            }
        }
        let life = style.engine_trail.life_s.max(style.wingtip_trail.life_s);
        let mut kept = 0;
        for i in 0..self.tracks.len() {
            let track = &mut self.tracks[i];
            if !track.published {
                track.bodies.clear();
            }
            while track.bodies.front().is_some_and(|b| t - b.t > life) {
                track.bodies.pop_front();
            }
            for (ring, &kind) in track.rings.iter_mut().zip(&track.kinds) {
                let life = style.trail(kind).life_s;
                while ring.front().is_some_and(|s| t - s.t >= life) {
                    ring.pop_front();
                }
            }
            if track.published || track.rings.iter().any(|r| !r.is_empty()) {
                self.tracks.swap(kept, i);
                kept += 1;
            }
        }
        self.spent.extend(self.tracks.drain(kept..));
    }

    /// Take the jets drawn at `clock`: where each one's emitters are on its
    /// model; one first drawn lays the history already published.
    pub fn observe(&mut self, style: &JetExhaustStyle, clock: f64, jets: &[DrawnJet]) {
        for jet in jets {
            let Some(track) = self.track(jet.key, style.max_aircraft) else {
                continue;
            };
            if !track.kinds.iter().eq(jet.emitters.iter().map(|e| &e.0)) {
                track.kinds.clear();
                track.kinds.extend(jet.emitters.iter().map(|e| e.0));
                track.rings.clear();
                track.rings.resize_with(track.kinds.len(), VecDeque::new);
            }
            track.engine.clone_from(&jet.engine);
            track.cruise_mps = jet.cruise_mps;
            track.aft = jet.aft;
            track.load = jet.load;
            let (s, c) = libm::sincos(jet.yaw);
            track.heads.clear();
            track.local.clear();
            for &(_, p) in &jet.emitters {
                let d = [p[0] - jet.at[0], p[1] - jet.at[1], p[2] - jet.at[2]];
                track.heads.push(p);
                track
                    .local
                    .push([d[0] * c + d[1] * s, -d[0] * s + d[1] * c, d[2]]);
            }
            let fresh = track.drawn.is_none_or(|d| clock - d > style.max_gap_s);
            if fresh && track.rings.iter().all(VecDeque::is_empty) {
                for body in track.bodies.clone() {
                    track.lay(&body, style);
                }
            }
            track.drawn = Some(clock);
        }
    }
}
