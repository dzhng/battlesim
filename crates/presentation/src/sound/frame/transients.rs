//! Positional reports, footsteps, and the transient voice budget.
use super::{
    profile, sector_pan, Bus, Command, Distance, Key, Motion, Pending, Sound, Sounding, Stride,
    Vec3, Voice, FADE_STEAL_S, LOOKAHEAD_S,
};

impl Sound {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn queue(
        &mut self,
        t: f64,
        sound: String,
        far: Option<String>,
        far_m: f64,
        bus: Bus,
        distance: Distance,
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
            distance,
        });
    }

    /// A footstep each `stride_m` a soldier walks; a jump past `max_step_m`
    /// (seen again elsewhere) starts his count afresh.
    pub(super) fn footsteps(&mut self, motion: &Motion, clock: f64, step: f64) {
        let tables = self.tables.clone();
        let f = &tables.audio.footsteps;
        let distance = tables.audio.distance[profile::SMALL_DETAILS];
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
            if self.level(f.gain, Some(&s.position), distance) < distance.cull {
                continue;
            }
            let variety = Key::new().num(s.id).num(clock);
            self.queue(
                clock,
                tables.effect(&f.sound).to_owned(),
                None,
                0.0,
                Bus::Units,
                distance,
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
    pub(super) fn flush(&mut self, clock: f64, now: f64) {
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
        let mut ranked: Vec<(Pending, f64)> = due
            .into_iter()
            .map(|e| {
                let priority = self.level(e.gain, e.position.as_ref(), e.distance);
                (e, priority)
            })
            .filter(|(e, priority)| *priority >= e.distance.cull)
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
}
