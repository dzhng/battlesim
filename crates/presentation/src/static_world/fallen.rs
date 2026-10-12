//! The state a side knows each building in, from the props it has learned:
//! what the simulation published decides, never a rule worked out here. A
//! part's place taken by a prop of a kind in the gutted-shell set (the kinds
//! a gutted building's parts become, from the catalog's building rows) stands
//! the building gutted; taken by anything else, or destroyed with nothing
//! left, it is a ruin. A destruction the side has not seen leaves the
//! building intact, however long ago it was.
//!
//! The simulation destroys a building whole, by one rule, and a side that
//! sees any part of it learns every part, so one known part says what the
//! building is. Were a side ever to know parts in both states, the building
//! is a ruin: a shell is drawn only where nothing known says a part came down.
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

/// `STATES` indices a known building can be drawn in.
pub const RUIN: usize = 1;
pub const GUTTED: usize = 2;

/// A known prop as the rule reads it: the map prop whose place it took (none
/// for a prop that is no map prop's, a wreck), its kind, and whether it was
/// destroyed with nothing left. The publication's `KnownProp`, or a lab's
/// stand-in for one, keyed as the page's `KnownProp` is.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Known {
    #[serde(default)]
    pub authored_prop: Option<f64>,
    pub kind: String,
    #[serde(default)]
    pub destroyed: bool,
}

/// How a side's knowledge stands a map's buildings: which building each part
/// prop belongs to, and the prop kinds a gutted building's parts become.
#[derive(Default)]
pub struct Knowledge {
    part_building: HashMap<u32, usize>,
    shells: HashSet<String>,
}

impl Knowledge {
    /// `parts` each a part's prop and its building; `shells` the gutted
    /// buildings' part kinds.
    pub fn new(
        parts: impl IntoIterator<Item = (u32, usize)>,
        shells: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            part_building: parts.into_iter().collect(),
            shells: shells.into_iter().collect(),
        }
    }

    /// The buildings `known` says are no longer intact, by building, each
    /// with its state (`RUIN` or `GUTTED`). Each known prop is the map prop
    /// it replaced, its kind and whether it was destroyed.
    pub fn fallen<'a>(
        &self,
        known: impl IntoIterator<Item = (Option<f64>, &'a str, bool)>,
    ) -> Vec<(usize, usize)> {
        let mut states = BTreeMap::new();
        for (authored, kind, destroyed) in known {
            let Some(&b) = authored.and_then(|p| self.part_building.get(&(p as u32))) else {
                continue;
            };
            let stands = !destroyed && self.shells.contains(kind);
            let state = states.entry(b).or_insert(GUTTED);
            if !stands {
                *state = RUIN;
            }
        }
        states.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A house of parts 0 and 1, a tower of part 10, and a shed of part 20;
    /// a gutted building's parts become a burnt shell.
    fn town() -> Knowledge {
        Knowledge::new(
            [(0, 0), (1, 0), (10, 1), (20, 2)],
            ["burnt_shell".to_owned()],
        )
    }

    fn seen(known: &[(Option<f64>, &str, bool)]) -> Vec<(usize, usize)> {
        town().fallen(known.iter().copied())
    }

    #[test]
    fn nothing_seen_stands_every_building_and_a_wreck_is_no_buildings() {
        assert_eq!(seen(&[]), vec![]);
        assert_eq!(seen(&[(None, "heavy_wreck", false)]), vec![]);
        // A map prop that is no building's part is none of theirs.
        assert_eq!(seen(&[(Some(5.0), "ruin", false)]), vec![]);
    }

    #[test]
    fn remains_are_a_ruin_and_one_known_part_is_the_whole_buildings_state() {
        let remains = [(Some(0.0), "ruin", false), (Some(1.0), "ruin", false)];
        assert_eq!(seen(&remains), vec![(0, RUIN)]);
        assert_eq!(seen(&remains[1..]), vec![(0, RUIN)]);
        // A part seen destroyed with nothing left is a ruin's too, even a shell's.
        assert_eq!(seen(&[(Some(0.0), "burnt_shell", true)]), vec![(0, RUIN)]);
    }

    #[test]
    fn a_standing_shell_is_gutted_whatever_its_height() {
        assert_eq!(
            seen(&[(Some(10.0), "burnt_shell", false)]),
            vec![(1, GUTTED)]
        );
        // The published kind decides: a low shed's shell is gutted, its
        // remains a ruin.
        assert_eq!(
            seen(&[(Some(20.0), "burnt_shell", false)]),
            vec![(2, GUTTED)]
        );
        assert_eq!(seen(&[(Some(20.0), "ruin", false)]), vec![(2, RUIN)]);
    }

    #[test]
    fn parts_known_both_ways_are_a_ruin_in_either_order() {
        let mixed = [
            (Some(0.0), "burnt_shell", false),
            (Some(1.0), "ruin", false),
        ];
        assert_eq!(seen(&mixed), vec![(0, RUIN)]);
        let reversed = [mixed[1], mixed[0]];
        assert_eq!(seen(&reversed), vec![(0, RUIN)]);
    }

    #[test]
    fn each_building_by_its_own_knowledge_in_the_maps_order() {
        let known = [
            (Some(10.0), "burnt_shell", false),
            (None, "heavy_wreck", false),
            (Some(0.0), "ruin", false),
            (Some(1.0), "ruin", false),
        ];
        assert_eq!(seen(&known), vec![(0, RUIN), (1, GUTTED)]);
    }
}
