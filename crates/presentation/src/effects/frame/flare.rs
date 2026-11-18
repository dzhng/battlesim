//! A visible decoy carries a shower of burning pellets. Only observed paths
//! extend it: losing sight stops emission, while its short trails fade out.
use std::collections::BTreeMap;

use super::*;
use crate::effects::tables::FlareStyle;

#[derive(Default)]
pub(super) struct Flares {
    /// Endpoint continuity joins the anonymous, visibility-clipped paths.
    tracks: BTreeMap<[u64; 3], Shower>,
}

struct Shower {
    born: f64,
    seed: u32,
}

impl Flares {
    pub(super) fn clear(&mut self) {
        self.tracks.clear();
    }

    pub(super) fn note(
        &mut self,
        style: &FlareStyle,
        publication: &EffectPublication,
        t0: f64,
        gap: bool,
        pool: &mut Pool,
    ) {
        let mut previous = std::mem::take(&mut self.tracks);
        if gap {
            previous.clear();
        }
        let dt = pool.dt;
        for flare in &publication.flares {
            let shower = previous
                .remove(&flare.from.map(f64::to_bits))
                .unwrap_or_else(|| Shower {
                    born: t0,
                    seed: Key::new().text("flare").nums(&flare.from).num(t0).seed(),
                });
            // Keep the missile's physical decoy visible among the shower.
            let mut smoke_rng = Key::new().num(t0).nums(&flare.from).rng();
            pool.tracer_style(
                style.tracer.clone(),
                "flare".into(),
                t0,
                &[flare.from, flare.to],
                &mut smoke_rng,
                true,
            );
            let mut rng = Mulberry32::new(shower.seed);
            for _ in 0..style.pellets {
                let angle = rng.float(0.0, 2.0 * PI);
                let reach = style.spread_m * rng.float(0.35, 1.0);
                let offset = [
                    libm::cos(angle) * reach,
                    libm::sin(angle) * reach,
                    -reach * rng.float(0.0, 0.25),
                ];
                let point = |center: Vec3, age: f64| {
                    let spread = 1.0 - libm::exp(-age / style.spread_s);
                    std::array::from_fn(|k| center[k] + offset[k] * spread)
                };
                let age = t0 - shower.born;
                // Every pellet shares the observed decoy's motion. Only its
                // bounded spread is visual; no second trajectory can run away.
                let path = [point(flare.from, age), point(flare.to, age + dt)];
                if path[1][2] > 0.0 {
                    pool.tracer_style(
                        style.tracer.clone(),
                        "flare".into(),
                        t0,
                        &path,
                        &mut smoke_rng,
                        true,
                    );
                }
            }
            self.tracks.insert(flare.to.map(f64::to_bits), shower);
        }
    }
}
