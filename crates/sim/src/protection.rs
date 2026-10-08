//! Finite defensive interception; offensive fire and sensing remain separate.
use crate::digest::Digest;
use contract::catalog::ActiveProtection;

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
    pub fn digest(&self, digest: &mut Digest) {
        digest.u64(u64::from(self.charges)).f64(self.ready_at_tick);
    }
}
