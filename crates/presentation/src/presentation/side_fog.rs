//! The side's fog of war as the presentation keeps it: what a scene asks of
//! it, the sight it holds, and preparing that sight for a renderer.
use super::*;

/// What a scene asks of fog of war (`Presentation::set_fog`).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct FogSetup {
    /// The rules' sensor numbers sight shares with the simulation.
    pub sensors: Option<Sensors>,
    /// The side's foliage (the simulation's export less what it has seen
    /// cleared); none keeps the last, at first the ground's.
    #[serde(skip)]
    pub foliage: Option<Vec<f32>>,
    /// A lab's stand-in for the publication's eyes.
    pub eyes: Option<Vec<Eye>>,
    /// Only these own units' eyes (a lab looking through one unit).
    pub units: Option<Vec<f64>>,
}

/// The side's sight over the ground, and what it was last set from.
pub(super) struct Fog {
    pub(super) sight: FogSight,
    pub(super) sensors: Sensors,
    pub(super) eyes: Option<Vec<Eye>>,
    pub(super) units: Option<Vec<f64>>,
    pub(super) shown: bool,
    /// The sight changed since the last frame (a new stand-in, fog shown).
    pub(super) dirty: bool,
    /// The publication the eyes were last taken from, and the known props.
    pub(super) from: Option<Arc<Observation>>,
    pub(super) known_from: Option<Shared<KnownProp>>,
}

impl Presentation {
    /// Draws the side's fog of war over the ground (none: everything seen):
    /// its sight from its own units' eyes at each publication, cut by the
    /// ground, the side's foliage and the occluders it knows. A lab may
    /// stand eyes in for the publication's, or look through some units'.
    pub fn set_fog(&mut self, setup: Option<FogSetup>) -> Result<(), String> {
        let Some(setup) = setup else {
            if let Some(fog) = &mut self.fog {
                fog.shown = false;
            }
            return Ok(());
        };
        let ground = self
            .ground
            .as_ref()
            .ok_or("fog of war is cut by a ground: set_ground first")?;
        let sensors = match (setup.sensors, &self.fog) {
            (Some(s), _) => s,
            (None, Some(fog)) => fog.sensors,
            (None, None) => return Err("fog: no sensor rules".into()),
        };
        let fresh = self.fog.as_ref().is_none_or(|f| {
            let a = f.sensors;
            (
                a.fog_target_height_m,
                a.foliage_full_block,
                a.min_sight_gap_m,
            ) != (
                sensors.fog_target_height_m,
                sensors.foliage_full_block,
                sensors.min_sight_gap_m,
            )
        });
        if fresh {
            let mut sight = FogSight::new(ground.tuning.fog_geometry.clone())?;
            sight.set_world(
                SightWorld::of(&ground.source, sensors),
                ground.source.foliage.clone(),
            )?;
            self.fog = Some(Fog {
                sight,
                sensors,
                eyes: None,
                units: None,
                shown: true,
                dirty: true,
                from: None,
                known_from: None,
            });
        }
        let fog = self.fog.as_mut().unwrap();
        if let Some(foliage) = setup.foliage {
            fog.sight.set_foliage(foliage);
        }
        fog.dirty |= !fog.shown || fog.eyes != setup.eyes || fog.units != setup.units;
        fog.eyes = setup.eyes;
        fog.units = setup.units;
        fog.shown = true;
        Ok(())
    }

    /// One drawn frame's fog for `camera` (none: a probe's) over a
    /// `width`×`height` target, with `probes` points to probe; none without
    /// fog. The renderer uploads it as it is and runs its passes.
    pub fn prepare_fog(
        &mut self,
        camera: Option<&crate::records::Camera>,
        width: u32,
        height: u32,
        probes: u32,
    ) -> Result<Option<FogUpload>, String> {
        let Some(fog) = &mut self.fog else {
            return Ok(None);
        };
        let latest = self.latest.clone();
        // The side sees from its stand-in eyes, else its latest publication's.
        let sighted = fog.shown && (fog.eyes.is_some() || latest.is_some());
        let new = match (&fog.from, &latest) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if !sighted {
            fog.sight.clear_sight();
        } else if fog.dirty || new {
            // Only a new publication or stand-in changes the sight: the
            // eyes and what the side knows are taken then, not each frame.
            let eyes = match (&fog.eyes, &latest) {
                (Some(eyes), _) => eyes.clone(),
                (None, Some(o)) => Eye::of_units(
                    o.own
                        .iter()
                        .filter(|u| fog.units.as_ref().is_none_or(|ids| ids.contains(&u.id)))
                        .map(|u| (u.id, &u.sight)),
                ),
                (None, None) => unreachable!("sighted"),
            };
            let group = latest.as_ref().map(|o| o.known_props.clone());
            let changed = match (&group, &fog.known_from) {
                (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                (None, None) => false,
                _ => true,
            };
            let known: Vec<KnownBox> = match (&group, changed) {
                (Some(g), true) => g.iter().map(KnownBox::published).collect(),
                _ => Vec::new(),
            };
            fog.sight
                .set_sight(&eyes, changed.then_some(known.as_slice()))?;
            fog.known_from = group;
            fog.from = latest;
            fog.dirty = false;
        }
        fog.sight.prepare(camera, width, height, probes).map(Some)
    }

    /// The renderer holds no fog (a new one): the next upload sends it all.
    pub fn resend_fog(&mut self) {
        if let Some(fog) = &mut self.fog {
            fog.sight.resend();
        }
    }

    /// Every eye's map rebuilds on the next frame (a cost probe).
    pub fn rebuild_fog(&mut self) {
        if let Some(fog) = &mut self.fog {
            fog.sight.invalidate();
        }
    }

    /// What the fog holds; none without fog.
    pub fn fog_stats(&self) -> Option<FogStats> {
        self.fog.as_ref().map(|f| f.sight.stats())
    }

    /// The eyes the fog sees from, as the last frame took them.
    pub fn fog_eyes(&self) -> Vec<Eye> {
        self.fog.as_ref().map_or_else(Vec::new, |f| f.sight.eyes())
    }

    /// The occluders the side's fog knows, in the order of the structures'
    /// seen flags.
    pub fn fog_occluders(&self) -> Vec<Occluder> {
        self.fog
            .as_ref()
            .map_or_else(Vec::new, |f| f.sight.occluders())
    }
}
