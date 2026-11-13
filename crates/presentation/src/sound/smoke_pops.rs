//! A smoke screen's pop, heard at its cloud by a side that sees the cloud
//! while it blooms (`specs/done/smoke-screen` D16). A side that first sees it
//! later never hears it, and a pop is never a hearing cue: an unseen cloud
//! is silent.
use std::collections::HashMap;

use contract::smoke::{state, SmokeCloud};

use super::tables::SoundTables;
use crate::seed::Key;
use crate::world::Vec3;

/// The clouds a side has heard pop, by id, each kept while it still blooms.
#[derive(Default)]
pub(super) struct SmokePops {
    voiced: HashMap<u32, SmokeCloud>,
}

/// Whether `c` is still blooming at `tick`: the law's phases before full size.
fn blooming(c: &SmokeCloud, tick: u64, tick_hz: u32) -> bool {
    state(c, tick, tick_hz).is_some_and(|s| s.phase < c.spec.bloom_steps)
}

impl SmokePops {
    /// The pops `clouds` (the side's, at frame `tick`, `dt` seconds a tick)
    /// sound now: each cloud seen while it blooms and not heard before, as
    /// its look's sound, at its centre, keyed by its id.
    pub(super) fn heard(
        &mut self,
        clouds: &[SmokeCloud],
        tick: f64,
        dt: f64,
        tables: &SoundTables,
    ) -> Vec<(String, Vec3, Key)> {
        let (tick, tick_hz) = (tick as u64, (1.0 / dt).round() as u32);
        self.voiced.retain(|_, c| blooming(c, tick, tick_hz));
        let mut pops = Vec::new();
        for c in clouds {
            if !blooming(c, tick, tick_hz) || self.voiced.contains_key(&c.id) {
                continue;
            }
            self.voiced.insert(c.id, c.clone());
            let key = Key::new().text("smoke").num(f64::from(c.id));
            pops.push((tables.smoke_pops[&c.spec.look].clone(), c.centre, key));
        }
        pops
    }

    /// Forgets every cloud (a new battle).
    pub(super) fn clear(&mut self) {
        self.voiced.clear();
    }
}
