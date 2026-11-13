//! A publication as a script or a capture gives the battle's sound: the
//! sound lab, the workbench and the recorded sound vectors.
use std::sync::Arc;

use serde::Deserialize;

use super::frame::{Cue, Heard, Shout};
use crate::effects::launches::Launch;
use crate::effects::publication::EffectPublication;

/// A publication as a script or a capture gives it (the browser's
/// `SoundPublication`): what the effects read, the launches by where they
/// leave, and the side's hearing cues.
#[derive(Deserialize)]
pub struct Scripted {
    effects: EffectPublication,
    launches: Vec<ScriptedShot>,
    #[serde(default)]
    audible: Vec<Cue>,
    #[serde(default)]
    shouts: Vec<Shout>,
}

#[derive(Deserialize)]
struct ScriptedShot {
    shooter: f64,
    mount: usize,
    soldier: Option<f64>,
    x: f64,
    y: f64,
    z: f64,
    kind: Arc<str>,
}

impl Scripted {
    /// Its launches, as the effects derive them.
    pub fn launches(&self) -> Vec<Launch> {
        self.launches
            .iter()
            .map(|l| Launch {
                shooter: l.shooter,
                mount: l.mount,
                soldier: l.soldier,
                at: [l.x, l.y, l.z],
                dir: [0.0; 3],
                kind: l.kind.clone(),
                hull: l.soldier.is_none(),
            })
            .collect()
    }

    /// The publication as sound hears it, with its `launches`.
    pub fn heard<'a>(&'a self, launches: &'a [Launch]) -> Heard<'a> {
        Heard {
            effects: &self.effects,
            launches,
            audible: self.audible.clone(),
            shouts: self.shouts.clone(),
        }
    }
}
