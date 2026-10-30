//! World asset admission and record publication for browser presentation.
use super::{error, set, Presentation};
use presentation::records::WorldHeader;
use presentation::static_world::buildings::{Placed, PlacedBuildings};
use presentation::static_world::library::{Bounds, Library};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    /// One skinned body's dense clip table, before `load_world`.
    pub fn add_clip_table(&mut self, name: String, floats: Vec<f32>) {
        self.tables.insert(name, floats);
    }

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

    /// Loads what the world draws (`WorldInput`
    /// JSON) with the clip tables and buildings added; returns the
    /// `WorldHeader`'s address.
    pub fn load_world(&mut self, json: &str) -> Result<u32, JsError> {
        let input = serde_json::from_str(json).map_err(error)?;
        self.inner
            .load_world(
                input,
                std::mem::take(&mut self.tables),
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
