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
    pub tracer: Arc<TracerStyle>,
    pub flash: Arc<FlashStyle>,
    pub impact: Impact,
    pub blast: f64,
    pub kick: Kick,
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
    pub shot: NearFar,
    pub blast: NearFar,
    pub motor: Option<OneShot>,
    #[serde(default)]
    pub impacts: BTreeMap<String, String>,
    pub impact_scale: f64,
}

impl WeaponLook {
    pub fn check(&self, name: &str) -> Result<(), String> {
        let path = format!("presentation.weapon_looks.{name}");
        crate::settings::validate::numbers(self, &path, &[".forward_m"])?;
        self.tracer.validate(name)?;
        for (field, value) in [
            ("blast", self.blast),
            ("impact.scale", self.impact.scale),
            ("sound.shot.far_m", self.sound.shot.far_m),
            ("sound.blast.far_m", self.sound.blast.far_m),
        ] {
            crate::settings::validate::range(
                &format!("{path}.{field}"),
                value,
                f64::MIN_POSITIVE,
                f64::MAX,
            )?;
        }
        if self.sound.shot.near.is_empty() || self.sound.shot.far.is_empty() {
            return Err(format!(
                "{path}.sound.shot: firing needs explicit near and far recordings"
            ));
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
        for (name, look) in &looks {
            look.check(name)?;
        }
        let rows =
            contract::weapons::resolve_weapons(weapons).map_err(|e| format!("weapons: {e}"))?;
        let mut index = BTreeMap::new();
        for (id, row) in rows {
            if !looks.contains_key(&row.appearance) {
                return Err(format!(
                    "weapons.{id}.appearance: unknown look {}",
                    row.appearance
                ));
            }
            index.insert(id, row.appearance);
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
            if s.blast.near.is_empty() {
                s.blast.near = audio.weapon_defaults.blast.near.clone();
            }
            if s.blast.far.is_empty() {
                s.blast.far = audio.weapon_defaults.blast.far.clone();
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
            catalog.check_shot(&look.sound.shot, &at)?;
            for (part, sound) in [
                ("blast.near", Some(look.sound.blast.near.as_str())),
                ("blast.far", Some(look.sound.blast.far.as_str())),
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
