//! Finite defensive interception; offensive fire and sensing remain separate.
use crate::digest::Digest;
use contract::catalog::ActiveProtection;
use contract::observation::ProtectionReadiness;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub charges: u32,
    pub ready_at_tick: f64,
}
impl State {
    pub fn new(capability: &ActiveProtection) -> Self {
        Self {
            charges: capability.capacity,
            ready_at_tick: 0.0,
        }
    }
    pub fn intercept(&mut self, capability: &ActiveProtection, time: f64, hz: u32) -> bool {
        if self.charges == 0 || time < self.ready_at_tick {
            return false;
        }
        self.charges -= 1;
        self.ready_at_tick = time + capability.cooldown_s * f64::from(hz);
        true
    }
    /// What its owner sees at `tick`: the charges left, and how far the
    /// cooldown has run while one runs.
    pub fn readiness(
        &self,
        capability: &ActiveProtection,
        tick: f64,
        hz: u32,
    ) -> ProtectionReadiness {
        let remaining = (self.ready_at_tick - tick).max(0.0);
        ProtectionReadiness {
            charges: self.charges,
            cooldown: (remaining > 0.0).then(|| {
                (1.0 - remaining / (capability.cooldown_s * f64::from(hz))).clamp(0.0, 1.0)
            }),
        }
    }
    pub fn digest(&self, digest: &mut Digest) {
        digest.u64(u64::from(self.charges)).f64(self.ready_at_tick);
    }
}
