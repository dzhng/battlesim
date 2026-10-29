//! Admission of authored presentation documents. Each section has one typed
//! reader, shared by the native filesystem and browser bindings.
mod ui;
pub(crate) mod validate;
pub use ui::*;

use crate::{
    effects::{tables::EffectPresentation, transit::TransitFeel},
    fog::{sight::Geometry, FogPresentation},
    light::LightSettings,
    static_world::ground::{tuning::TerrainTuning, Tuning},
    units::feel::PoseFeel,
    world::{BuildingStyle, Detail},
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Deserialize without losing the path to a bad nested key or value.
pub fn read<T: DeserializeOwned>(value: &Value, path: &str) -> Result<T, String> {
    serde_path_to_error::deserialize(value).map_err(|e| {
        let at = e.path().to_string();
        format!(
            "{path}{}: {}",
            if at.is_empty() || at == "." {
                String::new()
            } else {
                format!(".{at}")
            },
            e.inner()
        )
    })
}

/// The whole `fixtures/presentation.json`. No battle rules or mix settings
/// enter this document; section readers also admit their isolated inputs.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSettings {
    pub terrain: TerrainTuning,
    pub ground: Ground,
    pub camera: Camera,
    pub light: LightSettings,
    pub fog_geometry: Geometry,
    pub fog: FogPresentation,
    pub map_border: MapBorder,
    pub overlay: Overlay,
    pub hud: Hud,
    pub contacts: Contacts,
    pub models: Detail,
    pub buildings: BuildingStyle,
    pub pose: PoseFeel,
    pub transit: TransitFeel,
    pub effects: EffectPresentation,
    pub weapon_looks: BTreeMap<String, std::sync::Arc<crate::looks::WeaponLook>>,
}

impl PresentationSettings {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let settings: Self = read(value, "presentation")?;
        settings.light.check()?;
        settings.fog.selected()?;
        settings.fog_geometry.check()?;
        settings.pose.check()?;
        settings.effects.check()?;
        for (name, look) in &settings.weapon_looks {
            look.check(name)?;
        }
        validate::presentation(&settings)?;
        Ok(settings)
    }

    pub fn tuning(&self) -> Tuning {
        Tuning {
            terrain: self.terrain.clone(),
            fog_geometry: self.fog_geometry.clone(),
        }
    }

    /// Display-encoded UI colours, with the overlay's own alpha.
    pub fn ally_xray(&self) -> [f64; 4] {
        let [r, g, b] = self.hud.ally;
        [r, g, b, self.overlay.xray.ally_alpha]
    }
}
