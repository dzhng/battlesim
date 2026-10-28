//! Current static populations shared by framing and renderer reconstruction.
use super::*;
use crate::static_world::batches::Population;

pub(super) fn populations<'a>(
    buildings: &'a mut Option<BuildingPopulation>,
    scenery: &'a mut Option<SceneryPopulation<'_>>,
    props: &'a mut Option<PropPopulation>,
) -> Vec<&'a mut dyn Population> {
    let mut populations: Vec<&mut dyn Population> = Vec::new();
    if let Some(buildings) = buildings {
        populations.push(buildings);
    }
    if let Some(scenery) = scenery {
        populations.push(scenery);
    }
    if let Some(props) = props.as_mut().filter(|props| !props.rows.is_empty()) {
        populations.push(props);
    }
    populations
}

impl Presentation {
    /// Materialize the current frame's held static batches for a fresh
    /// renderer without advancing its clock or repeating UI/pose decisions.
    pub fn resend_static_frame(&mut self, generation: u32) {
        let mut scenery = match (&mut self.scenery, &self.ground) {
            (Some(walk), Some(ground)) if !walk.rows.is_empty() => {
                Some(SceneryPopulation { walk, ground })
            }
            _ => None,
        };
        let populations = populations(&mut self.buildings, &mut scenery, &mut self.props);
        self.statics
            .materialize(&populations, &mut self.frame, generation);
    }
}
