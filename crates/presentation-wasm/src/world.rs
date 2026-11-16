//! World asset admission and record publication for browser presentation.
use super::{error, set, Presentation};
use presentation::records::WorldHeader;
use presentation::static_world::buildings::{Placed, PlacedBuildings};
use presentation::static_world::library::{Bounds, Library};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    /// The map's buildings, before `load_world`: the template library's
    /// file, inflated; per library module its mesh bounds (min, max), NaN
    /// where its kit is not installed; per building its template (an index
    /// of `templates`, a JSON list of ids), frame (x, y, z, yaw) and owner
    /// prop; each part prop (`part_props`) and its building
    /// (`part_buildings`); and the prop kinds a gutted building's parts
    /// become (`shells`, a JSON list). `load_world`'s `buildings` style
    /// places them.
    #[allow(clippy::too_many_arguments)]
    pub fn add_buildings(
        &mut self,
        library: &[u8],
        bounds: &[f32],
        templates: &str,
        template: &[u16],
        frames: &[f64],
        owners: &[u32],
        part_props: &[u32],
        part_buildings: &[u32],
        shells: &str,
    ) -> Result<(), JsError> {
        let ids: Vec<String> = serde_json::from_str(templates).map_err(error)?;
        let shells: Vec<String> = serde_json::from_str(shells).map_err(error)?;
        let bounds: Vec<Option<Bounds>> = bounds
            .as_chunks::<6>()
            .0
            .iter()
            .map(|b| (!b[0].is_nan()).then_some(*b))
            .collect();
        let placed = (0..owners.len())
            .map(|b| Placed {
                owner: owners[b],
                template: ids[template[b] as usize].clone(),
                frame: frames[b * 4..b * 4 + 4].try_into().unwrap(),
            })
            .collect();
        self.buildings = Some(PlacedBuildings {
            library: Library::decode(library).map_err(error)?,
            bounds,
            placed,
            parts: part_props
                .iter()
                .zip(part_buildings)
                .map(|(&p, &b)| (p, b as usize))
                .collect(),
            shells,
        });
        Ok(())
    }

    /// Stages the loaded battle fielded again, borrowing its sections and clip
    /// tables from the result and reading only shell-owned world settings from
    /// JSON, for the next appearance admission, which draws it
    /// without loading the world anew; false when it is another battle,
    /// which only `load_world` loads.
    pub fn stage_battle(
        &mut self,
        json: &str,
        fielded: &crate::fielded::Fielded,
    ) -> Result<bool, JsError> {
        let input = world_input(json, Some(fielded))?;
        let tables = fielded.inner.tables.clone();
        self.inner.stage_battle(input, tables).map_err(error)
    }

    /// The appearances a staged battle draws: its admission brings their art.
    pub fn staged_appearances(&self) -> Vec<String> {
        self.inner.staged_appearances()
    }

    /// Loads what the world draws: shell-owned settings as JSON, battle sections
    /// and clip tables from its result, and the buildings added; returns the
    /// `WorldHeader`'s address.
    pub fn load_world(
        &mut self,
        json: &str,
        fielded: Option<crate::fielded::Fielded>,
    ) -> Result<u32, JsError> {
        let input = world_input(json, fielded.as_ref())?;
        self.inner
            .load_world(
                input,
                fielded
                    .as_ref()
                    .map_or_else(Default::default, |f| f.inner.tables.clone()),
                self.buildings.take(),
            )
            .map_err(error)?;
        Ok(self.refresh_world_header())
    }
}

impl Presentation {
    pub fn refresh_world_header(&mut self) -> u32 {
        let world = self.inner.world();
        self.world_header = WorldHeader {
            appearances: set(&world.appearances),
            strings: set(world.strings.as_bytes()),
        };
        &self.world_header as *const WorldHeader as u32
    }
}

/// The shell owns the light and observer settings; the result owns the sections.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldSettings {
    battle: Option<BattleSettings>,
    sun_azimuth: f64,
    sun_elevation: f64,
    buildings: Option<presentation::world::BuildingStyle>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BattleSettings {
    side: String,
    tick_hz: f64,
    soldier: presentation::world::SoldierBody,
    xray: presentation::world::Xray,
    blocked: [f64; 4],
    stand_in: Option<presentation::world::StandIn>,
}
fn world_input(
    json: &str,
    fielded: Option<&crate::fielded::Fielded>,
) -> Result<presentation::world::WorldInput, JsError> {
    let input: WorldSettings = serde_json::from_str(json).map_err(error)?;
    let battle = input
        .battle
        .map(|b| {
            let f = &fielded
                .ok_or_else(|| error("battle has no fielding result"))?
                .inner;
            Ok::<_, JsError>(presentation::world::BattleInput {
                units: f.units.clone(),
                effects: f.effects.clone(),
                side: b.side,
                tick_hz: b.tick_hz,
                soldier: b.soldier,
                xray: b.xray,
                blocked: b.blocked,
                stand_in: b.stand_in,
            })
        })
        .transpose()?;
    Ok(presentation::world::WorldInput {
        battle,
        sun_azimuth: input.sun_azimuth,
        sun_elevation: input.sun_elevation,
        buildings: input.buildings,
    })
}
