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

/// The fixture's default lighting look: mid-morning sun, stored in `light`.
pub const DAY_LIGHT_PRESET: &str = "day";

fn day_light_preset() -> String {
    DAY_LIGHT_PRESET.to_owned()
}

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
    pub camera_armory: ArmoryCamera,
    /// The selected lighting look, resolved from [`Self::light_preset`].
    pub light: LightSettings,
    /// Named lighting looks other than [`DAY_LIGHT_PRESET`], which is the
    /// authored `light` object. After admission the table also holds `day`.
    #[serde(default)]
    pub light_presets: BTreeMap<String, LightSettings>,
    /// Which look `light` holds. `"day"` is the authored default; changing
    /// this field is how golden hour becomes the shipped default.
    #[serde(default = "day_light_preset")]
    pub light_preset: String,
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
    /// Smoke screens' looks, by the name a smoke capability gives its cloud.
    pub smoke_screen_looks:
        BTreeMap<String, std::sync::Arc<crate::effects::tables::SmokeScreenLook>>,
}

impl PresentationSettings {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let mut settings: Self = read(value, "presentation")?;
        settings.install_light_presets()?;
        settings.light.check()?;
        settings.fog.selected()?;
        settings.fog_geometry.check()?;
        settings.pose.check()?;
        settings.effects.check()?;
        settings.effects.check_looks(&settings.weapon_looks)?;
        for (name, look) in &settings.smoke_screen_looks {
            look.check(name)?;
        }
        validate::presentation(&settings)?;
        Ok(settings)
    }

    /// Admit every named look and resolve [`Self::light`] to the selected one.
    /// `day` is reserved in `light_presets`: the default look lives in `light`.
    fn install_light_presets(&mut self) -> Result<(), String> {
        if self.light_presets.contains_key(DAY_LIGHT_PRESET) {
            return Err(
                "presentation.light_presets.day: reserved; the default look is presentation.light"
                    .into(),
            );
        }
        self.light.check()?;
        for (name, light) in &self.light_presets {
            light.check().map_err(|e| {
                e.replace(
                    "presentation.light",
                    &format!("presentation.light_presets.{name}"),
                )
            })?;
        }
        let mut table = std::mem::take(&mut self.light_presets);
        table.insert(DAY_LIGHT_PRESET.to_owned(), self.light.clone());
        let selected = self.light_preset.as_str();
        self.light = table
            .get(selected)
            .ok_or_else(|| format!("presentation.light_preset: unknown preset `{selected}`"))?
            .clone();
        self.light_presets = table;
        Ok(())
    }

    /// The same document under another named look.
    pub fn with_light_preset(&self, name: &str) -> Result<Self, String> {
        let light = self
            .light_presets
            .get(name)
            .cloned()
            .ok_or_else(|| format!("presentation.light_preset: unknown preset `{name}`"))?;
        let mut out = self.clone();
        out.light = light;
        out.light_preset = name.to_owned();
        Ok(out)
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
