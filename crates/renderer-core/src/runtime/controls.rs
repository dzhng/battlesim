//! Live rendering choices, retained across target and device reconstruction.
use super::*;
use crate::frame::FrameView;

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Controls {
    pub view: FrameView,
    pub paint: bool,
    pub grass: bool,
    pub trees: bool,
    pub buildings: bool,
    pub models: bool,
    pub effects: bool,
    pub scars: bool,
    pub road_wear: bool,
    pub field_texture: bool,
    pub cast_lights: bool,
    pub xray_coverage: bool,
    /// Albedo, normal, ORM; disabled channels retain authored material factors.
    pub texture_channels: [bool; 3],
    /// Opaque, room, cutout, blended, applied in every mesh pass.
    pub surfaces: [bool; 4],
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            view: FrameView::Final,
            paint: true,
            grass: true,
            trees: true,
            buildings: true,
            models: true,
            effects: true,
            scars: true,
            road_wear: true,
            field_texture: true,
            cast_lights: true,
            xray_coverage: true,
            texture_channels: [true; 3],
            surfaces: [true; 4],
        }
    }
}

impl Runtime {
    /// All passes read the same controls; changing a diagnostic cannot leave
    /// depth, shadows or a separately presented plane at an earlier setting.
    pub fn set_controls(
        &mut self,
        queue: &Queue,
        presentation: &Presentation,
        controls: Controls,
    ) -> Result<(), String> {
        let texture_changed =
            self.ground
                .set_field_texture(queue, presentation, controls.field_texture)?;
        let road_changed = self.ground.set_road_wear(queue, controls.road_wear);
        self.ground.material.scars.set_shown(queue, controls.scars);
        if texture_changed
            || road_changed
            || self.config.controls.grass != controls.grass
            || self.config.controls.scars != controls.scars
        {
            if let Some(grass) = &mut self.grass {
                grass.regrow();
            }
        }
        self.ground
            .set_class_view(queue, controls.view == FrameView::GroundClasses);
        self.fog_mask
            .set_mask_view(queue, controls.view.mask_view());
        if let Some(xray) = &mut self.xray {
            xray.set_coverage_enabled(queue, controls.xray_coverage);
        }
        self.apply_asset_controls(queue, controls);
        self.config.controls = controls;
        self.prepared_frame = None;
        Ok(())
    }
    pub(super) fn apply_asset_controls(&mut self, queue: &Queue, controls: Controls) {
        self.scenery.set_trees_shown(controls.trees);
        self.models.set_buildings_shown(controls.buildings);
        self.models.set_models_shown(controls.models);
        self.models
            .set_texture_channels(queue, controls.texture_channels);
        for (surface, shown) in [
            crate::models::Surface::Opaque,
            crate::models::Surface::Room,
            crate::models::Surface::Cutout,
            crate::models::Surface::Blended,
        ]
        .into_iter()
        .zip(controls.surfaces)
        {
            self.models.set_surface_shown(surface, shown);
        }
        self.models
            .set_high_apart(self.config.separate_high_air && controls.view == FrameView::Final);
    }
}
