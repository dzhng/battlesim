//! A smoke screen's pop, heard at its cloud by a side that sees the cloud
//! while it blooms (`specs/smoke-screen` D16). A side that first sees it
//! later never hears it, and a pop is never a hearing cue: an unseen cloud
//! is silent.
use std::collections::HashMap;

use contract::smoke::SmokeCloud;

use super::tables::SoundTables;
use crate::seed::Key;
use crate::world::Vec3;

/// The clouds a side has heard pop, by id, each kept until its bloom is over
/// (a frame tick), when it can no longer pop.
#[derive(Default)]
pub(super) struct SmokePops {
    voiced: HashMap<u32, f64>,
}

impl SmokePops {
    /// The pops `clouds` (the side's, at frame `tick`, `dt` seconds a tick)
    /// sound now: each cloud seen within its bloom of its pop and not heard
    /// before, as its look's sound, at its centre, keyed by its id.
    pub(super) fn heard(
        &mut self,
        clouds: &[SmokeCloud],
        tick: f64,
        dt: f64,
        tables: &SoundTables,
    ) -> Vec<(String, Vec3, Key)> {
        self.voiced.retain(|_, bloomed| tick <= *bloomed);
        let mut pops = Vec::new();
        for c in clouds {
            let bloomed = c.popped as f64 + c.spec.bloom_s / dt;
            if tick > bloomed || self.voiced.contains_key(&c.id) {
                continue;
            }
            self.voiced.insert(c.id, bloomed);
            if let Some(sound) = tables.smoke_pops.get(&c.spec.look) {
                let key = Key::new().text("smoke").num(f64::from(c.id));
                pops.push((sound.clone(), c.centre, key));
            }
        }
        pops
    }

    /// Forgets every cloud (a new battle).
    pub(super) fn clear(&mut self) {
        self.voiced.clear();
    }
}
