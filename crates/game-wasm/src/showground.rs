//! A squad standing at rest, for pages that show a unit outside a battle. The
//! scatter (`sim::arrangement`) and the first claim on each weapon
//! (`sim::weapons::operators::original_carriers`) stay the simulation's; this only asks
//! them about flat ground.
use contract::map::MoverClass;
use contract::scenario::Rules;
use serde::{Deserialize, Serialize};
use sim::arrangement;
use sim::math::v2;
use sim::weapons::operators::original_carriers;
use sim::weapons::Arsenal;
use sim::world::{Prop, WorldGeometry};
use wasm_bindgen::prelude::*;

use crate::js_error;

/// The flat ground's side: room for any squad's spread about its middle.
const GROUND_M: f64 = 400.0;

/// One squad as a fresh battle fields it, about the origin.
#[derive(Debug, Serialize, Deserialize)]
pub struct SquadAtRest {
    /// One soldier a slot, in slot order.
    pub soldiers: Vec<SoldierAtRest>,
    /// Each mount's operator, a soldier id; none for a gun the whole squad
    /// carries, or a single gun with no carrier free.
    pub operators: Vec<Option<u32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoldierAtRest {
    pub id: u32,
    pub slot: usize,
    /// Metres from the squad's centre: `[x, y]`, z up.
    pub at: [f64; 2],
}

/// Admitted rules over bare level ground.
#[wasm_bindgen]
pub struct Showground {
    world: WorldGeometry,
    arsenal: Arsenal,
    rules: Rules,
}

#[wasm_bindgen]
impl Showground {
    /// `rules_json` is a scenario's rules (the fixture with its catalog),
    /// admitted once.
    #[wasm_bindgen(constructor)]
    pub fn new(rules_json: &str) -> Result<Showground, JsError> {
        Self::admit(rules_json).map_err(js_error)
    }

    /// The squad of `kind` at rest about (0, 0), as a fresh battle squad of
    /// battle seed `seed` stands: `SquadAtRest` JSON. A kind with a hull is
    /// refused.
    pub fn squad(&self, kind: &str, seed: u32) -> Result<String, JsError> {
        let squad = self.at_rest(kind, seed).map_err(js_error)?;
        serde_json::to_string(&squad).map_err(js_error)
    }
}

impl Showground {
    pub fn admit(rules_json: &str) -> Result<Showground, String> {
        let rules: Rules = serde_json::from_str(rules_json).map_err(|e| e.to_string())?;
        sim::admit(&rules).map_err(|e| e.to_string())?;
        Ok(Showground {
            world: WorldGeometry::flat([GROUND_M; 2], &rules),
            arsenal: Arsenal::new(&rules),
            rules,
        })
    }

    /// [`Self::squad`] before it crosses the boundary.
    pub fn at_rest(&self, kind: &str, seed: u32) -> Result<SquadAtRest, String> {
        let index = self
            .rules
            .catalog
            .index(kind)
            .ok_or_else(|| format!("no unit type {kind:?}"))?;
        let unit = self.rules.catalog.get(index);
        if unit.hull().is_some() {
            return Err(format!("{kind} has a hull, not a squad"));
        }
        let count = unit.slots().map_or(0, <[String]>::len);
        let centre = v2(GROUND_M / 2.0, GROUND_M / 2.0);
        // Spawn's scatter: the battle's own rng and solids for unit 0.
        let solid = |p: &Prop| p.blocks(MoverClass::Infantry);
        let spots = arrangement::squad_spots(
            &self.world,
            centre,
            count,
            &self.rules.infantry_movement,
            self.rules.physics.soldier_radius_m,
            &solid,
            &mut arrangement::rng(u64::from(seed), 0, 0),
        );
        let soldiers: Vec<_> = spots
            .into_iter()
            .enumerate()
            .map(|(slot, p)| {
                let at = p - centre;
                SoldierAtRest {
                    id: slot as u32 + 1,
                    slot,
                    at: [at.x, at.y],
                }
            })
            .collect();
        let specs = self.arsenal.specs(index);
        let guns: Vec<_> = specs.iter().map(Some).collect();
        let members: Vec<_> = soldiers.iter().map(|s| Some(s.slot)).collect();
        let operators = original_carriers(&guns, &members)
            .into_iter()
            .map(|k| k.map(|k| soldiers[k].id))
            .collect();
        Ok(SquadAtRest {
            soldiers,
            operators,
        })
    }
}
