//! Continuous battlefield voices: reconcile the loudest loops with the player.
use std::collections::BTreeSet;

use super::{
    clamp01, lerp, pick, profile, vehicle_loops, Bus, Command, LoopKey, LoopWant, Motion, Mover,
    Params, Pending, Sound, Sounding, Voice, FADE_LOOP_S,
};

impl Sound {
    /// Every loop wanted now: the bed, engines and running gear, turrets,
    /// reverse, fires and flying rounds; the loudest within the loop budget sound.
    pub(super) fn update_loops(&mut self, clock: f64, now: f64, motion: &Motion, step: f64) {
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
                distance: p.distance[profile::WEAPONS],
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
            for mut layer in vehicle_loops(row, load, traverse, v.reverse) {
                if layer.slot == "engine" {
                    if let Some(speed) = v.rotor_speed {
                        layer.gain *= speed;
                        layer.rate *= speed;
                    }
                }
                wants.push(LoopWant {
                    key: LoopKey::Vehicle(key, layer.slot),
                    sound: layer.sound.to_owned(),
                    bus: Bus::Units,
                    gain: layer.gain,
                    rate: layer.rate,
                    position: Some(v.position),
                    priority: 0.0,
                    distance: p.distance[layer.distance],
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
                distance: p.distance[profile::BURNING],
            });
        }
        self.flight_wants(clock, &mut wants);
        for w in &mut wants {
            w.priority = self.level(w.gain, w.position.as_ref(), w.distance);
        }
        let mut kept: Vec<LoopWant> = wants
            .into_iter()
            .filter(|w| w.priority >= w.distance.cull)
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
            distance: p.distance[profile::WEAPONS],
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
