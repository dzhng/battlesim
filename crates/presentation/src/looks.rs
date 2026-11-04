//! A weapon's appearance resolves once through the contract's ordinary catalog
//! inheritance. Every presentation consumer reads the same admitted look.
use crate::effects::tables::{FlashStyle, RoundScale, TracerStyle};
use crate::sound::tables::{NearFar, OneShot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponLook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracer: Option<Arc<TracerStyle>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flash: Option<Arc<FlashStyle>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<Impact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blast: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kick: Option<Kick>,
    pub sound: LookSound,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Impact {
    pub scale: f64,
    pub surfaces: BTreeMap<String, RoundScale>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Kick {
    pub hull: f64,
    pub soldier: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LookSound {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot: Option<NearFar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blast: Option<NearFar>,
    pub motor: Option<OneShot>,
    #[serde(default)]
    pub impacts: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact_scale: Option<f64>,
}

impl WeaponLook {
    pub fn check(&self, name: &str) -> Result<(), String> {
        let path = format!("presentation.weapon_looks.{name}");
        crate::settings::validate::numbers(self, &path, &[".forward_m"])?;
        if let Some(tracer) = &self.tracer {
            tracer.validate(name)?;
            if let Some(cast) = &tracer.cast {
                cast.check(&format!("{path}.tracer.cast"), false)?;
            }
        }
        if let Some(flash) = &self.flash {
            if let Some(cast) = &flash.cast {
                cast.check(&format!("{path}.flash.cast"), true)?;
            }
        }
        if let Some(impact) = &self.impact {
            for (surface, scale) in &impact.surfaces {
                if !crate::effects::tables::IMPACT_MATERIALS.contains(&surface.as_str()) {
                    return Err(format!(
                        "{path}.impact.surfaces.{surface}: unknown impact material"
                    ));
                }
                for (field, value) in [("size", scale.size), ("duration", scale.duration)] {
                    crate::settings::validate::range(
                        &format!("{path}.impact.surfaces.{surface}.{field}"),
                        value,
                        f64::MIN_POSITIVE,
                        f64::MAX,
                    )?;
                }
            }
            crate::settings::validate::range(
                &format!("{path}.impact.scale"),
                impact.scale,
                f64::MIN_POSITIVE,
                f64::MAX,
            )?;
        }
        if let Some(blast) = self.blast {
            crate::settings::validate::range(
                &format!("{path}.blast"),
                blast,
                f64::MIN_POSITIVE,
                f64::MAX,
            )?;
        }
        for (part, sound) in [("shot", &self.sound.shot), ("blast", &self.sound.blast)] {
            if let Some(sound) = sound {
                crate::settings::validate::range(
                    &format!("{path}.sound.{part}.far_m"),
                    sound.far_m,
                    f64::MIN_POSITIVE,
                    f64::MAX,
                )?;
            }
        }
        if let Some(shot) = &self.sound.shot {
            if shot.near.is_empty() || shot.far.is_empty() {
                return Err(format!(
                    "{path}.sound.shot: firing needs explicit near and far recordings"
                ));
            }
        }
        Ok(())
    }

    fn check_firing(&self, weapon: &str) -> Result<(), String> {
        for (part, missing) in [
            ("tracer", self.tracer.is_none()),
            (
                "impact",
                self.impact.is_none() || self.sound.impact_scale.is_none(),
            ),
            ("kick", self.kick.is_none()),
            ("shot", self.sound.shot.is_none()),
            ("flash", self.flash.is_none()),
        ] {
            if missing {
                return Err(format!(
                    "weapons.{weapon}.appearance: missing firing {part}"
                ));
            }
        }
        Ok(())
    }
}

/// A battle's flat weapon-to-appearance index and shared look library. This is
/// also the admitted wire form carried in the effect section.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponLooks {
    pub weapons: BTreeMap<String, String>,
    pub looks: BTreeMap<String, Arc<WeaponLook>>,
}

impl WeaponLooks {
    pub fn from_value(presentation: &Value, weapons: &Value) -> Result<Self, String> {
        let looks =
            crate::settings::read(&presentation["weapon_looks"], "presentation.weapon_looks")?;
        Self::new(looks, weapons)
    }

    pub fn new(looks: BTreeMap<String, Arc<WeaponLook>>, weapons: &Value) -> Result<Self, String> {
        let rows =
            contract::weapons::resolve_weapons(weapons).map_err(|e| format!("weapons: {e}"))?;
        let mut index = BTreeMap::new();
        for (id, row) in rows {
            let look = looks.get(&row.appearance).ok_or_else(|| {
                format!("weapons.{id}.appearance: unknown look {}", row.appearance)
            })?;
            let required = [
                (
                    "blast",
                    row.blast_radius_m > 0.0
                        && (look.blast.is_none() || look.sound.blast.is_none()),
                ),
                (
                    "motor",
                    row.ballistics.accel_mps2.is_some() && look.sound.motor.is_none(),
                ),
            ];
            for (part, missing) in required {
                if missing {
                    return Err(format!(
                        "weapons.{id}.appearance ({}): missing {part}",
                        row.appearance
                    ));
                }
            }
            index.insert(id, row.appearance);
        }
        for (name, look) in &looks {
            look.check(name)?;
        }
        Ok(Self {
            weapons: index,
            looks,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.weapons.is_empty() && self.looks.is_empty()
    }
    pub fn name(&self, weapon: &str) -> &str {
        &self.weapons[weapon]
    }
    pub fn get(&self, weapon: &str) -> &WeaponLook {
        &self.looks[self.name(weapon)]
    }

    /// Every ammunition kind a catalog unit or soldier mounts must support firing.
    pub fn check_mounts(&self, catalog: &Value) -> Result<(), String> {
        for row in catalog["units"]
            .as_array()
            .ok_or("unit catalog: no units")?
            .iter()
            .chain(
                catalog["soldiers"]
                    .as_object()
                    .into_iter()
                    .flat_map(|s| s.values()),
            )
        {
            for mount in row["mounts"].as_array().into_iter().flatten() {
                for weapon in mount["weapons"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    let name = self.weapons.get(weapon).ok_or_else(|| {
                        format!("weapons.{weapon}: mounted weapon has no appearance")
                    })?;
                    self.looks[name].check_firing(weapon)?;
                }
            }
        }
        Ok(())
    }

    pub fn with_sounds(
        mut self,
        audio: &crate::sound::tables::AudioMix,
        catalog: &crate::sound::catalog::SoundCatalog,
    ) -> Result<Self, String> {
        for (name, look) in &mut self.looks {
            let s = &mut Arc::make_mut(look).sound;
            for hit in s.impacts.keys() {
                if !audio.weapon_defaults.impacts.contains_key(hit) {
                    return Err(format!(
                        "presentation.weapon_looks.{name}.sound.impacts.{hit}: unknown surface"
                    ));
                }
            }
            if let Some(blast) = &mut s.blast {
                if blast.near.is_empty() {
                    blast.near = audio.weapon_defaults.blast.near.clone();
                }
                if blast.far.is_empty() {
                    blast.far = audio.weapon_defaults.blast.far.clone();
                }
            }
            if let Some(m) = &mut s.motor {
                if m.sound.is_empty() {
                    m.sound = audio.weapon_defaults.motor.clone();
                }
            }
            for (hit, name) in &audio.weapon_defaults.impacts {
                s.impacts.entry(hit.clone()).or_insert_with(|| name.clone());
            }
        }
        self.check_sounds(catalog)?;
        Ok(self)
    }

    pub fn check_sounds(
        &self,
        catalog: &crate::sound::catalog::SoundCatalog,
    ) -> Result<(), String> {
        for (name, look) in &self.looks {
            let at = format!("presentation.weapon_looks.{name}.sound");
            if let Some(shot) = &look.sound.shot {
                catalog.check_shot(shot, &at)?;
            }
            for (part, sound) in [
                (
                    "blast.near",
                    look.sound.blast.as_ref().map(|s| s.near.as_str()),
                ),
                (
                    "blast.far",
                    look.sound.blast.as_ref().map(|s| s.far.as_str()),
                ),
                ("motor", look.sound.motor.as_ref().map(|m| m.sound.as_str())),
            ] {
                if let Some(sound) = sound {
                    let recipe = catalog.sound(sound, &format!("{at}.{part}"))?;
                    if recipe.looping != (part == "motor") {
                        return Err(format!(
                            "{at}.{part}: recording loop does not match its part"
                        ));
                    }
                }
            }
            for (hit, sound) in &look.sound.impacts {
                if catalog
                    .sound(sound, &format!("{at}.impacts.{hit}"))?
                    .looping
                {
                    return Err(format!("{at}.impacts.{hit}: impact cannot use a loop"));
                }
            }
        }
        Ok(())
    }
}

/// Unused looks are a shipped-fixture error, never a per-battle restriction.
pub fn check_shipped(presentation: &Value, weapons: &Value) -> Result<(), String> {
    let reader = WeaponLooks::from_value(presentation, weapons)?;
    let used: BTreeSet<_> = reader.weapons.values().collect();
    for name in reader.looks.keys() {
        if !used.contains(name) {
            return Err(format!("presentation.weapon_looks.{name}: unused look"));
        }
    }
    Ok(())
}
