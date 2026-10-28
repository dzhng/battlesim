//! Prepared populations and installed geometry, distinct from issued pass work.
use super::*;
use serde_json::json;

impl Models {
    pub fn diagnostics(&self) -> serde_json::Value {
        let mut tiers = [0u64; presentation::view::TIERS];
        let mut bodies = [0u64; presentation::view::TIERS];
        let (mut triangles, mut draws) = (0u64, 0u64);
        for run in &self.runs {
            let Some(row) = &self.rows[run.row] else {
                continue;
            };
            if row.building || run.pass == CAST_PASS {
                continue;
            }
            let Some(drawable) = row.drawable(run.tier) else {
                continue;
            };
            tiers[run.tier] += u64::from(run.count);
            if matches!(row.kind, Kind::Soldier) {
                bodies[run.tier] += u64::from(run.count);
            }
            triangles += u64::from(drawable.count / 3) * u64::from(run.count);
            draws += 1;
        }
        let mut textures = crate::memory::ResourceMemory::default();
        textures.view(&self.albedo);
        textures.view(&self.surface);
        json!({
            "installed": self.installed,
            "appearances": self.installed.len(),
            "instances": tiers.iter().sum::<u64>(),
            "triangles": triangles,
            "draws": draws,
            "tiers": tiers,
            "bodyTiers": bodies,
            "skinned": bodies.iter().sum::<u64>(),
            "cards": self.drawn.impostors,
            "corpses": self.drawn.corpses,
            "atlasLayers": self.cards.as_ref().map_or(0, |cards| cards.layers()),
            "textureLayers": self.albedo.texture().depth_or_array_layers() + self.surface.texture().depth_or_array_layers(),
            "textureBytes": textures.counts().texture_bytes,
        })
    }

    pub fn building_diagnostics(&self) -> serde_json::Value {
        let mut tiers = [0u64; presentation::view::TIERS];
        let mut triangles = [0u64; presentation::view::TIERS];
        let mut record_bytes = 0;
        for run in &self.runs {
            let Some(row) = &self.rows[run.row] else {
                continue;
            };
            if !row.building || run.pass == CAST_PASS {
                continue;
            }
            let Some(drawable) = row.drawable(run.tier) else {
                continue;
            };
            tiers[run.tier] += u64::from(run.count);
            triangles[run.tier] += u64::from(drawable.count / 3) * u64::from(run.count);
        }
        for held in self.statics.listed() {
            if self
                .rows
                .get(held.row as usize)
                .and_then(Option::as_ref)
                .is_some_and(|row| row.building)
            {
                record_bytes += held.buffer.size();
            }
        }
        let submitted = self.submissions.snapshot();
        json!({
            "tiers": tiers,
            "triangles": triangles,
            "recordBytes": record_bytes,
            "draws": submitted.color.buildings.draws,
            "casterDraws": submitted.cascades.iter().map(|p| p.buildings.draws).sum::<u64>(),
            "casterTriangles": submitted.cascades.iter().map(|p| p.buildings.triangles).sum::<u64>(),
        })
    }
}

impl Models {
    pub fn drawn_instances(&self) -> serde_json::Value {
        let mut buildings = Vec::new();
        let mut offsets = vec![None; self.rows.len()];
        for (index, row) in self.rows.iter().enumerate() {
            if row.as_ref().is_some_and(|row| row.building) {
                offsets[index] = Some(buildings.len());
                buildings.resize(buildings.len() + presentation::view::TIERS, 0u64);
            }
        }
        let mut props = std::collections::BTreeMap::<&str, [u64; presentation::view::TIERS]>::new();
        for run in self.runs.iter().filter(|run| run.pass == VIEW_PASS) {
            let Some(row) = &self.rows[run.row] else {
                continue;
            };
            if let Some(offset) = offsets[run.row] {
                if self.buildings_shown {
                    buildings[offset + run.tier] += u64::from(run.count);
                }
            } else if row.prop {
                props.entry(&self.row_names[run.row]).or_default()[run.tier] +=
                    u64::from(run.count);
            }
        }
        json!({
            "buildings": buildings,
            "props": props.into_iter().map(|(appearance, levels)| json!({"appearance": appearance, "levels": levels})).collect::<Vec<_>>(),
        })
    }
}
