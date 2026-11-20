//! A recorded battle replayed to a tick, then played live from there: the
//! tutorial's chapter restart, and the dev loop that starts a chapter
//! without playing the ones before it. Replay is the only proof; nothing is
//! cloned or snapshotted.
use super::*;

/// Why a battle can't be resumed at a tick.
#[derive(Debug, PartialEq, Eq)]
pub enum ResumeError {
    Replay(ReplayError),
    /// An AI whose memory lies outside the digest played before the tick, so
    /// replaying the log can't rebuild what it knew.
    AiMemory {
        player: String,
    },
    /// The battle had ended by the tick.
    Beyond {
        tick: Tick,
    },
}

impl std::fmt::Display for ResumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Replay(error) => error.fmt(f),
            Self::AiMemory { player } => {
                write!(
                    f,
                    "{player}'s AI played before the tick, and its memory isn't replayed"
                )
            }
            Self::Beyond { tick } => write!(f, "the battle had ended by tick {tick}"),
        }
    }
}

impl From<ReplayError> for ResumeError {
    fn from(error: ReplayError) -> Self {
        Self::Replay(error)
    }
}

impl Battle {
    /// `replay`'s battle replayed to `tick`, then live: from `tick` on it
    /// takes input and runs its AI seats as a played battle does. Its record
    /// goes on from the replayed prefix, so its own replay is the battle as
    /// it was played to `tick` and after.
    pub fn resume(
        setup: &ScenarioDefinition,
        replay: &Replay,
        tick: Tick,
    ) -> Result<Self, ResumeError> {
        Self::resume_with(setup, replay, tick, || Self::new(setup, replay.seed))
    }

    /// `resume`, keeping the prepared map's physical allocations.
    pub fn resume_prepared(
        setup: &ScenarioDefinition,
        replay: &Replay,
        tick: Tick,
        prepared: crate::encounter::PreparedMap,
    ) -> Result<Self, ResumeError> {
        Self::resume_with(setup, replay, tick, || {
            Self::from_prepared(setup, replay.seed, prepared)
        })
    }

    fn resume_with(
        setup: &ScenarioDefinition,
        replay: &Replay,
        tick: Tick,
        build: impl FnOnce() -> Self,
    ) -> Result<Self, ResumeError> {
        if let Some(player) = remembering_ai(setup, replay, tick) {
            return Err(ResumeError::AiMemory { player });
        }
        let cut = Replay {
            accepted: replay
                .accepted
                .iter()
                .filter(|(at, _)| *at <= tick)
                .cloned()
                .collect(),
            controller_transitions: replay
                .controller_transitions
                .iter()
                .filter(|t| t.tick <= tick)
                .cloned()
                .collect(),
            script_trace: replay
                .script_trace
                .iter()
                .filter(|t| t.tick <= tick)
                .cloned()
                .collect(),
            seat_events: replay
                .seat_events
                .iter()
                .filter(|e| e.tick() <= tick)
                .cloned()
                .collect(),
            ..replay.clone()
        };
        let mut battle = Self::load_replay(setup, &cut, build)?;
        while battle.tick < tick && !battle.finished() {
            battle.step();
        }
        if battle.finished() {
            return Err(ResumeError::Beyond { tick });
        }
        battle.replaying = None;
        // A controller change recorded at `tick` was made before the tick
        // ran, as a live one made now is.
        for transition in std::mem::take(&mut battle.replay_controller_transitions)
            .into_values()
            .flatten()
        {
            battle.apply_controller(&transition);
        }
        battle.controller_transitions = cut.controller_transitions;
        battle.script_trace = cut.script_trace;
        battle.replay_script_trace.clear();
        Ok(battle)
    }
}

/// The first AI that played before `tick` and keeps a memory the digest
/// doesn't hold: an active AI seat, or the encounter's opponent.
fn remembering_ai(setup: &ScenarioDefinition, replay: &Replay, tick: Tick) -> Option<String> {
    if tick == 0 {
        return None;
    }
    if setup.opponent.is_some() {
        return Some("the opponent".to_owned());
    }
    // Each seat's controller and the tick it took over. A record with no
    // controller changes (a battle without a skirmish, or nobody changed)
    // names its controllers once, for the whole battle.
    let mut seats: BTreeMap<&String, (&ControllerKind, Tick)> = (setup.players.iter())
        .map(|p| (&p.id, (&p.controller, 0)))
        .collect();
    if replay.controller_transitions.is_empty() {
        seats.extend(replay.controllers.iter().map(|(id, c)| (id, (c, 0))));
    }
    // An active AI played if it held a seat through a tick before `tick`; a
    // change takes effect before its tick runs.
    let played = |(controller, since): (&ControllerKind, Tick), until: Tick| {
        *controller == ControllerKind::ActiveAi && since < until
    };
    for change in (replay.controller_transitions.iter()).filter(|t| t.tick < tick) {
        if let Some(&held) = seats.get(&change.player) {
            if played(held, change.tick) {
                return Some(change.player.clone());
            }
        }
        seats.insert(&change.player, (&change.controller, change.tick));
    }
    (seats.into_iter())
        .find(|&(_, held)| played(held, tick))
        .map(|(player, _)| player.clone())
}

impl Battle {
    /// Where `chapter` of a solved scenario opens, or where its `beat` fires,
    /// for starting there (the chapters' dev loop): the battle its solution
    /// plays to that point, as a replay to resume at the returned tick, with
    /// every human-capable seat handed back to a person then. A human seat
    /// takes no director orders, so the solution's later beats leave the
    /// person's units alone.
    pub fn chapter_start(
        solved: &ScenarioDefinition,
        seed: u64,
        chapter: &str,
        beat: Option<&str>,
        limit: Tick,
    ) -> Result<(Replay, Tick), String> {
        let director = solved
            .director
            .as_ref()
            .ok_or("the scenario has no director")?;
        let index = (director.chapters.iter())
            .position(|c| c.id == chapter)
            .ok_or(format!("no chapter {chapter:?}"))?;
        let mut battle = Battle::new(solved, seed);
        let there = |battle: &Battle| {
            battle.director().is_some_and(|run| {
                run.chapter() >= index && beat.is_none_or(|beat| run.fired_at(beat).is_some())
            })
        };
        while !there(&battle) {
            if battle.finished() || battle.tick() >= limit {
                return Err(format!("chapter {chapter:?} never reached {beat:?}"));
            }
            battle.step();
        }
        // A start on the tick that decided the match resumes the tick before,
        // which decides it again: a battle resumes only before its end.
        let tick = battle.tick() - Tick::from(battle.finished());
        let mut replay = battle.replay();
        for player in solved.players.iter().filter(|p| p.human_capable) {
            replay.controller_transitions.push(ControllerTransition {
                tick,
                player: player.id.clone(),
                controller: ControllerKind::Human,
            });
        }
        Ok((replay, tick))
    }
}
