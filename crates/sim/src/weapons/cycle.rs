//! A physical gun owns its magazine and timing independently of targeting.
use super::{MountSpec, Weapon};
use crate::digest::Digest;
use contract::random::Rng;
use contract::weapons::{AmmoCapacity, WeaponDefinition};

/// Magazine, aim and timing of one physical weapon; the mount selects its target.
#[derive(Clone, Debug)]
pub struct Cycle {
    pub owner: Option<u32>,
    /// Retained magazine, including one being topped up during a lull.
    pub loaded: Option<usize>,
    pub reload: Option<(usize, f64)>,
    /// Aim earned by this physical gun on its mount's current target.
    pub aim: f64,
    pub(super) rounds: u32,
    pub(super) cooldown: f64,
    pub(super) started: bool,
}

impl Cycle {
    pub(super) fn ready(&self) -> Option<usize> {
        self.loaded.filter(|_| self.reload.is_none())
    }

    pub fn new(owner: Option<u32>, spec: &MountSpec, weapons: &[Weapon]) -> Self {
        let loaded = spec
            .kinds
            .iter()
            .position(|&k| !matches!(weapons[k].def.ammo, AmmoCapacity::Rounds(0)));
        Self {
            owner,
            loaded,
            reload: None,
            aim: 0.0,
            rounds: loaded.map_or(0, |k| {
                weapons[spec.kinds[k]].def.magazine.map_or(1, |m| m.rounds)
            }),
            cooldown: 0.0,
            started: false,
        }
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.owner.map_or(u64::MAX, u64::from))
            .u64(self.loaded.map_or(u64::MAX, |k| k as u64))
            .u64(self.rounds as u64)
            .f64(self.aim)
            .f64(self.cooldown)
            .u64(self.started as u64)
            .u64(self.reload.is_some() as u64);
        if let Some((k, p)) = self.reload {
            d.u64(k as u64).f64(p);
        }
    }
    /// Putting a gun away preserves its magazine and reload work.
    pub fn pause(&mut self, dt: f64) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.aim = 0.0;
        self.started = false;
    }

    /// Spend one round of `def` from the magazine; whether it ended a firing
    /// cycle: emptied the magazine (read before a reload-free weapon tops it
    /// straight back up), closed a burst, or was a single shot. The gap to
    /// the next round is a burst's seeded pause or the shot interval.
    pub(super) fn spend_round(&mut self, def: &WeaponDefinition, rng: &mut Rng) -> bool {
        self.rounds -= 1;
        let closes_burst = |m: contract::weapons::Magazine| {
            m.burst
                .is_some_and(|b| (m.rounds - self.rounds).is_multiple_of(b.rounds))
        };
        let ended = self.rounds == 0 || def.magazine.is_some_and(closes_burst);
        if self.rounds == 0 && def.reload_s == 0.0 {
            if let Some(magazine) = def.magazine {
                self.rounds = magazine.rounds;
            }
        }
        if self.rounds == 0 {
            self.loaded = None;
            if def.magazine.is_some_and(|m| m.burst.is_some()) {
                self.started = false;
            }
        } else if let Some(magazine) = def.magazine {
            self.cooldown = magazine
                .burst
                .filter(|b| (magazine.rounds - self.rounds).is_multiple_of(b.rounds))
                .map_or(magazine.shot_interval_s, |b| {
                    (rng.unit() * b.aim_max_s).max(magazine.shot_interval_s)
                });
        }
        ended
    }

    pub fn needs_reload(&self, weapons: &[Weapon], ammo: &[Option<u32>], spec: &MountSpec) -> bool {
        self.reload.is_some()
            || match self.loaded {
                None => ammo.iter().any(|a| a.is_none_or(|n| n > 0)),
                Some(k) => weapons[spec.kinds[k]].def.magazine.is_some_and(|m| {
                    self.rounds < m.rounds && ammo[k].is_none_or(|n| n > self.rounds)
                }),
            }
    }
    /// Idle guns top up partial magazines without discarding their rounds;
    /// engagement cancels that top-up, keeping the retained rounds ready.
    /// A different kind sets the retained magazine aside without spending it
    /// and restarts the reload from zero (W05).
    pub fn advance(
        &mut self,
        weapons: &[Weapon],
        ammo: &[Option<u32>],
        spec: &MountSpec,
        want: Option<usize>,
        engaging: bool,
        dt: f64,
    ) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        if let (Some(l), Some(p)) = (self.loaded, want) {
            if l != p {
                self.loaded = None;
                self.reload = None;
            }
        }
        let want = if let Some(k) = self.loaded {
            if engaging {
                self.reload = None;
                return;
            }
            let magazine = weapons[spec.kinds[k]].def.magazine;
            if magazine.is_none_or(|m| self.rounds == m.rounds)
                || ammo[k].is_some_and(|n| n <= self.rounds)
            {
                return;
            }
            Some(k)
        } else {
            want.or(self.reload.map(|(k, _)| k))
                .or_else(|| ammo.iter().position(|n| n.is_none_or(|n| n > 0)))
        };
        self.reload = match want {
            Some(k) if ammo[k].is_none_or(|n| n > 0) => {
                let progress = match self.reload {
                    Some((rk, p)) if rk == k => p,
                    _ => 0.0,
                } + dt;
                let def = &weapons[spec.kinds[k]].def;
                if progress >= def.reload_s {
                    self.loaded = Some(k);
                    self.rounds = def.magazine.map_or(1, |m| m.rounds);
                    None
                } else {
                    Some((k, progress))
                }
            }
            _ => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract::weapons::{Burst, Magazine};

    /// A weapon row of the given magazine shape and reload time.
    fn shaped(magazine: Option<Magazine>, reload_s: f64) -> WeaponDefinition {
        let rules: contract::scenario::Rules =
            serde_json::from_value(crate::fixtures::test_rules()).expect("rules");
        let mut def = rules.weapons.into_values().next().expect("a weapon row");
        def.magazine = magazine;
        def.reload_s = reload_s;
        def
    }

    fn magazine(rounds: u32, burst: Option<u32>) -> Option<Magazine> {
        Some(Magazine {
            rounds,
            shot_interval_s: 0.1,
            burst: burst.map(|rounds| Burst {
                rounds,
                aim_max_s: 0.5,
            }),
        })
    }

    /// Which of `shots` rounds, fired back to back, ended a cycle (1-based),
    /// reloading the gun whenever its magazine empties.
    fn ends(def: &WeaponDefinition, shots: u32) -> Vec<u32> {
        let full = def.magazine.map_or(1, |m| m.rounds);
        let mut cycle = Cycle {
            owner: None,
            loaded: Some(0),
            reload: None,
            aim: 0.0,
            rounds: full,
            cooldown: 0.0,
            started: true,
        };
        let mut rng = Rng::new(1);
        (1..=shots)
            .filter(|_| {
                let ended = cycle.spend_round(def, &mut rng);
                if cycle.rounds == 0 {
                    (cycle.rounds, cycle.loaded) = (full, Some(0));
                }
                ended
            })
            .collect()
    }

    #[test]
    fn a_burst_that_divides_the_magazine_ends_a_cycle_each_burst() {
        assert_eq!(ends(&shaped(magazine(9, Some(3)), 2.0), 9), [3, 6, 9]);
    }

    #[test]
    fn the_short_last_burst_ends_on_the_empty_magazine() {
        assert_eq!(ends(&shaped(magazine(8, Some(3)), 2.0), 8), [3, 6, 8]);
    }

    #[test]
    fn a_reload_free_magazine_ends_a_cycle_each_time_it_empties() {
        assert_eq!(ends(&shaped(magazine(5, None), 0.0), 10), [5, 10]);
    }

    #[test]
    fn a_magazine_without_bursts_is_one_cycle() {
        assert_eq!(ends(&shaped(magazine(6, None), 9.0), 6), [6]);
    }

    #[test]
    fn a_single_shot_weapon_ends_a_cycle_every_shot() {
        assert_eq!(ends(&shaped(None, 5.0), 3), [1, 2, 3]);
    }
}
