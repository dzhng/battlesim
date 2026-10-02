//! A physical gun owns its magazine and timing independently of targeting.
use super::{MountSpec, Weapon};
use crate::digest::Digest;
use contract::weapons::AmmoCapacity;

/// Magazine and timing of one physical weapon, independent of its target lock.
#[derive(Clone, Debug)]
pub struct Cycle {
    pub owner: Option<u32>,
    /// Retained magazine, including one being topped up during a lull.
    pub loaded: Option<usize>,
    pub reload: Option<(usize, f64)>,
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
            .f64(self.cooldown)
            .u64(self.started as u64)
            .u64(self.reload.is_some() as u64);
        if let Some((k, p)) = self.reload {
            d.u64(k as u64).f64(p);
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
