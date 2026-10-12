//! The buildings' walk for native (`static_world::walk`, the browser's own):
//! glue only. It converts native's view into sim space and each record the
//! walk draws into a MultiMesh row, casters bucketed by region.
use crate::static_layers::{pack, region};
use godot::prelude::*;
use presentation::static_world::{files, walk};
use presentation::view::{Plane, View};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct BuildingWalk {
    walk: Option<walk::BuildingWalk>,
}

/// A walk record as native's MultiMesh row (`model_batches.gd` `pack`).
fn row(r: &walk::Record) -> Vec<f32> {
    pack(
        [r[0], r[1], r[2]],
        r[3],
        [r[12], r[13], r[14]],
        [r[8], r[9], r[10]],
    )
}

#[godot_api]
impl BuildingWalk {
    /// Saved map `map`'s buildings from the repository at `root`, under
    /// `fixtures/game.json`'s tiers. Returns the modules the counts are by
    /// (`{kit, module}`) and the rows over every state, or
    /// `{error}`.
    #[func]
    fn load(&mut self, root: GString, map: GString) -> VarDictionary {
        let mut out = VarDictionary::new();
        match files::buildings(Path::new(&root.to_string()), &map.to_string()) {
            Err(e) => out.set("error", &GString::from(e.as_str())),
            Ok((buildings, tiers)) => {
                let modules: VarArray = (0..buildings.modules.len())
                    .map(|m| {
                        let (kit, module) = buildings.module(m);
                        vdict! {"kit" => kit, "module" => module}.to_variant()
                    })
                    .collect();
                let rows: i32 = buildings
                    .records
                    .iter()
                    .flat_map(|r| r.rows)
                    .filter(|&n| n > 0)
                    .sum();
                out.set("modules", &modules);
                out.set("rows", rows as i64);
                self.walk = Some(walk::BuildingWalk::new(
                    buildings,
                    tiers.lod_px_per_m,
                    tiers.chunk_m,
                    tiers.pool_records,
                ));
            }
        }
        out
    }

    /// The buildings the side knows fallen, each in `states` (1 ruin, 2
    /// gutted). Returns those whose template has no such rows.
    #[func]
    fn set_fallen(
        &mut self,
        buildings: PackedInt32Array,
        states: PackedInt32Array,
    ) -> PackedInt32Array {
        let fallen: Vec<(usize, usize)> = buildings
            .as_slice()
            .iter()
            .zip(states.as_slice())
            .map(|(&b, &s)| (b as usize, s as usize))
            .collect();
        self.walk
            .as_mut()
            .unwrap()
            .set_fallen(&fallen)
            .into_iter()
            .map(|b| b as i32)
            .collect()
    }

    /// Walks native's `view` (`detail_view.gd` `packed`, Godot space); returns
    /// the keys (`module · 4 + tier`) whose drawn or cast instances changed.
    #[func]
    fn select(&mut self, view: PackedFloat64Array) -> PackedInt32Array {
        let v = view.as_slice();
        // Godot (x, y, z) is sim (x, −z, y); native's sides face out.
        let side = |i: usize| {
            let p = &v[8 + i * 4..12 + i * 4];
            Plane {
                normal: [-p[0], p[2], -p[1]],
                constant: p[3],
            }
        };
        let view = View::for_detail([v[0], -v[2], v[1]], v[6], std::array::from_fn(side));
        let shadow = walk::Shadow {
            fall: [v[3], -v[5]],
            reach: v[7],
        };
        let walk = self.walk.as_mut().unwrap();
        walk.select(&view, Some(&shadow))
            .into_iter()
            .map(|k| k as i32)
            .collect()
    }

    /// What the last walk draws at `key`.
    #[func]
    fn view(&self, key: i64) -> PackedFloat32Array {
        let list = &self.walk.as_ref().unwrap().lists.view[key as usize];
        list.iter().flat_map(row).collect()
    }

    /// What the last walk casts at `key`, by caster region.
    #[func]
    fn cast(&self, key: i64) -> VarDictionary {
        let mut regions: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
        for r in &self.walk.as_ref().unwrap().lists.cast[key as usize] {
            let pack = row(r);
            regions
                .entry(region(pack[3], pack[11]))
                .or_default()
                .extend(pack);
        }
        let mut out = VarDictionary::new();
        for (region, packs) in regions {
            out.set(region as i64, &PackedFloat32Array::from(packs.as_slice()));
        }
        out
    }

    /// What the last walk drew, per module and tier.
    #[func]
    fn counts(&self) -> PackedInt32Array {
        self.walk
            .as_ref()
            .unwrap()
            .counts
            .iter()
            .map(|&n| n as i32)
            .collect()
    }
}
