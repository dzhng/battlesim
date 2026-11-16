//! Replay records and admission to the exact engine and content.
use super::*;

/// Everything needed to reproduce a battle in the same build: the setup
/// identity, the seed and every sequenced command from both sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub engine_build: String,
    pub scenario_digest: String,
    pub config_digest: String,
    pub seed: u64,
    /// `(applied_tick, command)` in acceptance order.
    pub accepted: Vec<(Tick, CommandEnvelope)>,
    /// Authored victory/error terminal state captured after the last tick.
    #[serde(default)]
    pub script_victory: Option<bool>,
    #[serde(default)]
    pub script_error: Option<String>,
    #[serde(default)]
    pub script_terminal_tick: Option<Tick>,
    /// Controller choices used for the recorded session. Replays restore the
    /// same passive/active policy before consuming accepted commands.
    #[serde(default)]
    pub controllers: Vec<(String, ControllerKind)>,
    #[serde(default)]
    pub controller_transitions: Vec<ControllerTransition>,
    #[serde(default)]
    pub script_events: Vec<contract::script::ScriptEvent>,
    /// Authored script outputs at the tick where they became authoritative.
    #[serde(default)]
    pub script_trace: Vec<ScriptTrace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seat_events: Vec<SeatEvent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerTransition {
    pub tick: Tick,
    pub player: String,
    pub controller: ControllerKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptTrace {
    pub tick: Tick,
    #[serde(default)]
    pub events: Vec<contract::script::ScriptEvent>,
    #[serde(default)]
    pub victory: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    BuildMismatch,
    ScenarioMismatch,
    ConfigMismatch,
}

/// The shared native/Wasm engine fingerprint generated from simulation build inputs.
pub const ENGINE_BUILD_ID: &str = env!("SIM_ENGINE_BUILD_ID");

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BuildMismatch => "replay was recorded by a different simulation build",
            Self::ScenarioMismatch => "replay does not match this scenario",
            Self::ConfigMismatch => "replay does not match these simulation rules",
        })
    }
}

impl Battle {
    /// A battle that re-applies `replay`'s commands at their recorded ticks.
    /// Player input is refused for its whole life.
    pub fn from_replay(setup: &ScenarioDefinition, replay: &Replay) -> Result<Self, ReplayError> {
        Self::load_replay(setup, replay, || Self::new(setup, replay.seed))
    }

    /// Restore a replay while retaining the prepared map's physical allocations.
    pub fn from_prepared_replay(
        setup: &ScenarioDefinition,
        replay: &Replay,
        prepared: crate::encounter::PreparedMap,
    ) -> Result<Self, ReplayError> {
        Self::load_replay(setup, replay, || {
            Self::from_prepared(setup, replay.seed, prepared)
        })
    }

    fn load_replay(
        setup: &ScenarioDefinition,
        replay: &Replay,
        build: impl FnOnce() -> Self,
    ) -> Result<Self, ReplayError> {
        if replay.engine_build != ENGINE_BUILD_ID {
            return Err(ReplayError::BuildMismatch);
        }
        if format!("{:016x}", scenario_digest(setup)) != replay.scenario_digest {
            return Err(ReplayError::ScenarioMismatch);
        }
        if format!("{:016x}", config_digest(setup)) != replay.config_digest {
            return Err(ReplayError::ConfigMismatch);
        }
        let mut battle = build();
        for transition in &replay.controller_transitions {
            if transition.tick == 0 {
                battle.apply_controller(transition);
            } else {
                battle
                    .replay_controller_transitions
                    .entry(transition.tick)
                    .or_default()
                    .push(transition.clone());
            }
        }
        if replay.controller_transitions.is_empty() {
            for (player, controller) in &replay.controllers {
                battle.apply_controller(&ControllerTransition {
                    tick: 0,
                    player: player.clone(),
                    controller: controller.clone(),
                });
            }
        }
        battle.replay_script_trace = replay
            .script_trace
            .iter()
            .cloned()
            .map(|trace| (trace.tick, trace))
            .collect();
        battle.pending_seat_events = replay.seat_events.iter().cloned().collect();
        battle.seat_events = replay.seat_events.clone();
        battle.replaying = Some(replay.accepted.iter().cloned().collect());
        Ok(battle)
    }

    pub fn replay(&self) -> Replay {
        Replay {
            engine_build: ENGINE_BUILD_ID.to_owned(),
            scenario_digest: format!("{:016x}", self.scenario_digest),
            config_digest: format!("{:016x}", self.config_digest),
            seed: self.seed,
            accepted: self.accepted.clone(),
            script_victory: self.script_victory,
            script_error: self.script_error.clone(),
            script_terminal_tick: self.script_terminal_tick,
            controllers: self.controller_list(),
            controller_transitions: self.controller_transitions.clone(),
            script_events: self.script_events.clone(),
            script_trace: self.script_trace.clone(),
            seat_events: self.seat_events.clone(),
        }
    }
}
