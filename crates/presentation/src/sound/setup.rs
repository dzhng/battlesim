//! Document admission for a new battle observer.
use super::{
    frame::{SmokeTimes, Sound, Timing},
    tables::SoundTables,
};
use crate::effects::cook_off::CookOffFeel;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
/// A battle's sound as a player builds it, JSON: `audio.json`, the
/// sound catalog, the game's weapon rows, the presentation document and the
/// tick rate, with the resolved unit catalog's mounts. Smoke and cook-off
/// timings come from `presentation.effects`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    audio: serde_json::Value,
    catalog: serde_json::Value,
    weapons: serde_json::Value,
    unit_catalog: serde_json::Value,
    presentation: serde_json::Value,
    tick_hz: f64,
}

#[derive(Deserialize)]
struct Timings {
    smoke: HashMap<String, SmokeTimes>,
    cook_off: CookOffFeel,
}

impl Sound {
    /// A battle's sound from its setup JSON (`Setup`).
    pub fn from_json(json: &str) -> Result<Self, String> {
        let setup: Setup = serde_json::from_str(json).map_err(|e| format!("sound: {e}"))?;
        let tables = SoundTables::from_parts(
            setup.audio,
            setup.catalog,
            &setup.presentation,
            &setup.weapons,
        )?;
        tables.looks.check_mounts(&setup.unit_catalog)?;
        let effects: Timings =
            crate::settings::read(&setup.presentation["effects"], "presentation.effects")?;
        effects.cook_off.validate()?;
        Ok(Self::new(
            Arc::new(tables),
            Timing {
                tick_hz: setup.tick_hz,
                smoke: effects.smoke,
                cook_off: effects.cook_off,
            },
        ))
    }
}
