//! Live material and surface controls share the same owner in every pass.
use super::*;
impl Models {
    /// Albedo off keeps the texture's mean because vertex colors are relative
    /// to it; normal/ORM off fall back to material factors. Coverage stays live.
    pub fn set_texture_channels(&self, queue: &Queue, channels: [bool; 3]) {
        let mut rows = self.authored_materials.clone();
        for row in rows.as_chunks_mut::<20>().0 {
            for (channel, on) in channels.into_iter().enumerate() {
                let layer = row[8 + channel];
                if !on && layer >= 0. {
                    row[8 + channel] = if channel == 0 { -2. - layer } else { -1. };
                }
            }
        }
        queue.write_buffer(&self.materials, 0, bytemuck::cast_slice(&rows));
    }
    pub fn set_surface_shown(&mut self, surface: Surface, on: bool) {
        match surface {
            Surface::Solid => {
                self.surfaces[OPAQUE] = on;
                self.surfaces[ROOM] = on;
            }
            Surface::Cutout => self.surfaces[CUTOUT] = on,
            Surface::Opaque => self.surfaces[OPAQUE] = on,
            Surface::Room => self.surfaces[ROOM] = on,
            Surface::Blended => self.surfaces[BLENDED] = on,
        }
    }
    pub fn set_models_shown(&mut self, on: bool) {
        self.models_shown = on;
    }
    pub fn set_buildings_shown(&mut self, on: bool) {
        self.buildings_shown = on;
    }
}

pub(super) fn visible_parts(parts: &[Part; 4], requested: Part, shown: &[bool; 4]) -> [Part; 4] {
    if shown.iter().all(|shown| *shown) {
        return [requested, Part::default(), Part::default(), Part::default()];
    }
    std::array::from_fn(|class| {
        let part = parts[class];
        let first = part.first.max(requested.first);
        let end = (part.first + part.count).min(requested.first + requested.count);
        Part {
            first,
            count: if shown[class] {
                end.saturating_sub(first)
            } else {
                0
            },
        }
    })
}
