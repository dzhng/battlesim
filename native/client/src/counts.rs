//! What the last frame shows, counted as the browser's capture counts it
//! (the manifest's `modelCounts`, `effectCounts`, `instanceCounts` and
//! `frameCounts`): the models it poses by kind, the effects running by kind,
//! their instances by shape and the lights offered, and of the static world
//! in view, the building modules, the props and the scenery. All of it is
//! the presentation's, so a replay is counted without a GPU.
use std::collections::{BTreeMap, BTreeSet};

use presentation::effects::frame::{EFFECT_KINDS, SHAPE_NAMES};
use presentation::presentation::Presentation;
use presentation::records::{Counts, VIEW_PASS};
use presentation::static_world::ground::walk::Draws;
use presentation::static_world::props::{DRAWS_PROP, DRAWS_PROP_GROUND};
use presentation::view::TIERS;
use serde_json::Value;

/// The scenery's populations, in `Draws` order before the stumps.
const POPULATIONS: [&str; 5] = ["forest", "understorey", "backdrop", "dressing", "felled"];

pub struct Counted {
    /// The models posed and the effects running (`Frame.counts`).
    pub frame: Counts,
    /// The effect instances by shape (`SHAPE_NAMES`).
    pub shapes: [u32; SHAPE_NAMES.len()],
    /// The building modules in view, `row · TIERS + tier`.
    pub buildings: Vec<u32>,
    /// The props in view, by appearance, per tier.
    pub props: BTreeMap<String, [u32; TIERS + 1]>,
    /// The scenery in view per population, `kind · TIERS + tier`, the kind
    /// being the ground's `scenery_kinds` index.
    pub scenery: [Vec<u32>; 5],
    /// The stumps in view, per kind.
    pub stumps: Vec<u32>,
}

impl Counted {
    /// What the last frame `presentation` framed shows. Its batches list
    /// every static batch in view, written or named unchanged, with its
    /// instances.
    pub fn of(presentation: &Presentation, card_layers: &[Option<u32>]) -> Self {
        let frame = presentation.last_frame();
        let world = presentation.world();
        let kinds = presentation
            .ground()
            .map_or_else(Vec::new, |g| g.scenery_kinds());
        let mut shapes = [0; SHAPE_NAMES.len()];
        for e in presentation.effects() {
            if let Some(n) = shapes.get_mut(e.misc[0] as usize) {
                *n += 1;
            }
        }
        let mut out = Self {
            frame: frame.counts,
            shapes,
            buildings: Vec::new(),
            props: BTreeMap::new(),
            scenery: std::array::from_fn(|_| vec![0; kinds.len() * TIERS]),
            stumps: vec![0; kinds.len()],
        };
        for b in frame.batches.iter().filter(|b| b.pass == VIEW_PASS) {
            let Some(a) = world.appearances.get(b.appearance as usize) else {
                continue;
            };
            let name = &world.strings[a.name as usize..(a.name + a.name_len) as usize];
            let tier = (b.tier as usize).min(TIERS - 1);
            if a.draws == DRAWS_PROP || a.draws == DRAWS_PROP_GROUND {
                let tier = if b.tier == presentation::view::IMPOSTOR as u32
                    && card_layers
                        .get(b.appearance as usize)
                        .copied()
                        .flatten()
                        .is_some()
                {
                    TIERS
                } else {
                    tier
                };
                out.props.entry(name.to_owned()).or_default()[tier] += b.count;
            } else if a.draws == 0 {
                if a.placed != 0 {
                    let at = b.appearance as usize * TIERS + tier;
                    if out.buildings.len() <= at {
                        out.buildings.resize(at + 1, 0);
                    }
                    out.buildings[at] += b.count;
                }
            } else if let Some(kind) = kinds.iter().position(|k| k == name) {
                if a.draws == Draws::Stump as u32 {
                    // Only a tree leaves a stump; a shrub's stump row draws nothing.
                    if presentation.stump_mesh(b.appearance).is_some() {
                        out.stumps[kind] += b.count;
                    }
                } else if let Some(population) = out.scenery.get_mut(a.draws as usize - 1) {
                    // A felled tree is two records an instance.
                    let felled = a.draws == Draws::Felled as u32;
                    population[kind * TIERS + tier] += if felled { b.count / 2 } else { b.count };
                }
            }
        }
        out
    }

    /// Where these differ from what the browser counted of the same frame,
    /// each line led by `at`: its `models` (`modelCounts`' row), `effects`
    /// (`effectCounts`') and `drawn` (`instanceCounts`'). A row the browser
    /// did not count (null) is a difference too.
    pub fn mismatches(
        &self,
        at: &str,
        models: &Value,
        effects: &Value,
        drawn: &Value,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for (what, row) in [("model", models), ("effect", effects), ("instance", drawn)] {
            if row.is_null() {
                out.push(format!("{at}: the browser counted no {what}s"));
            }
        }
        let n = |v: &Value| v.as_u64().map_or(u32::MAX, |n| n as u32);
        let c = &self.frame;
        let mut checks: Vec<(String, u32, u32)> = Vec::new();
        if !models.is_null() {
            for (kind, got) in [
                ("soldiers", c.soldiers),
                ("vehicles", c.vehicles),
                ("aircraft", c.aircraft),
                ("corpses", c.corpses),
                ("fading", c.fading),
            ] {
                checks.push((kind.into(), got, n(&models[kind])));
            }
        }
        if !effects.is_null() {
            for (kind, got) in EFFECT_KINDS.iter().zip(c.running) {
                checks.push((format!("running {kind}"), got, n(&effects["running"][kind])));
            }
            checks.push(("lights".into(), c.lights, n(&effects["lights"])));
            for (name, got) in SHAPE_NAMES.iter().zip(self.shapes) {
                checks.push((
                    format!("{name} instances"),
                    got,
                    n(&effects["instances"][name]),
                ));
            }
        }
        out.extend(
            checks
                .into_iter()
                .filter(|(_, got, wanted)| got != wanted)
                .map(|(what, got, wanted)| format!("{at} {what}: {got} against {wanted}")),
        );
        if !drawn.is_null() {
            out.extend(self.building_mismatches(at, &drawn["buildings"]));
            out.extend(self.prop_mismatches(at, &drawn["props"]));
            out.extend(self.scenery_mismatches(at, &drawn["scenery"]));
        }
        out
    }

    fn building_mismatches(&self, at: &str, want: &Value) -> Vec<String> {
        let want = list(want);
        let got = &self.buildings;
        let of = |list: &[u32], i: usize| list.get(i).copied().unwrap_or(0);
        let total = |list: &[u32]| list.iter().sum::<u32>();
        (0..want.len().max(got.len()))
            .filter(|&i| of(got, i) != of(&want, i))
            .map(|i| {
                format!(
                    "{at} building module {} tier {}: {} against {} (all modules {} against {})",
                    i / TIERS,
                    i % TIERS,
                    of(got, i),
                    of(&want, i),
                    total(got),
                    total(&want)
                )
            })
            .collect()
    }

    fn prop_mismatches(&self, at: &str, want: &Value) -> Vec<String> {
        let want: BTreeMap<String, [u32; TIERS + 1]> = want
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| {
                let levels = list(&p["levels"]);
                let level = |t: usize| {
                    levels
                        .get(t)
                        .copied()
                        .unwrap_or(if t == TIERS { 0 } else { u32::MAX })
                };
                let name = p["appearance"].as_str().unwrap_or("").to_owned();
                (name, std::array::from_fn(level))
            })
            .collect();
        let got = &self.props;
        let total = |m: &BTreeMap<String, [u32; TIERS + 1]>| -> u32 { m.values().flatten().sum() };
        let names: BTreeSet<&String> = want.keys().chain(got.keys()).collect();
        names
            .into_iter()
            .filter(|n| want.get(*n) != got.get(*n))
            .map(|n| {
                format!(
                    "{at} props {n}: {:?} against {:?} (all props {} against {})",
                    got.get(n),
                    want.get(n),
                    total(got),
                    total(&want)
                )
            })
            .collect()
    }

    fn scenery_mismatches(&self, at: &str, want: &Value) -> Vec<String> {
        let total = |v: &[u32]| v.iter().sum::<u32>();
        POPULATIONS
            .iter()
            .zip(&self.scenery)
            .map(|(name, got)| (*name, got.clone(), list(&want[name])))
            .chain([("stumps", self.stumps.clone(), list(&want["stumps"]))])
            .filter(|(_, got, wanted)| got != wanted)
            .map(|(name, got, wanted)| {
                format!(
                    "{at} scenery {name}: {} against {} ({got:?} against {wanted:?})",
                    total(&got),
                    total(&wanted)
                )
            })
            .collect()
    }
}

/// A JSON list of counts; a count that is not one reads as `u32::MAX`.
fn list(v: &Value) -> Vec<u32> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|n| n.as_u64().map_or(u32::MAX, |n| n as u32))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prop_card_slot_cannot_be_reported_as_fallback_mesh() {
        let mut counted = Counted {
            frame: Counts::default(),
            shapes: [0; SHAPE_NAMES.len()],
            buildings: vec![],
            props: BTreeMap::new(),
            scenery: Default::default(),
            stumps: vec![],
        };
        counted.props.insert("test".into(), [0, 0, 0, 0, 3]);
        assert!(counted
            .prop_mismatches(
                "plate",
                &serde_json::json!([{"appearance":"test", "levels":[0,0,0,0,3]}])
            )
            .is_empty());
        assert!(!counted
            .prop_mismatches(
                "plate",
                &serde_json::json!([{"appearance":"test", "levels":[0,0,0,3]}])
            )
            .is_empty());
        counted.props.insert("test".into(), [0, 0, 0, 3, 0]);
        assert!(counted
            .prop_mismatches(
                "plate",
                &serde_json::json!([{"appearance":"test", "levels":[0,0,0,3]}])
            )
            .is_empty());
    }
}
