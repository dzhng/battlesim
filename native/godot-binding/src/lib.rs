use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use godot::prelude::*;
use serde_json::from_str;
use sim::battle::{Battle, Replay};
use sim::publication::Publisher;

struct BattleGameExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BattleGameExtension {}

#[derive(GodotClass)]
#[class(base=Node)]
struct SimulationProbe {
    base: Base<Node>,
    battle: Option<Battle>,
    scenario: Option<ScenarioDefinition>,
    publisher: Publisher,
}

#[godot_api]
impl INode for SimulationProbe {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            battle: None,
            scenario: None,
            publisher: Publisher::new(),
        }
    }
}

#[godot_api]
impl SimulationProbe {
    #[func]
    fn binding_name(&self) -> GString {
        "rust-simulation-probe".into()
    }

    #[func]
    fn simulation_crate_version(&self) -> GString {
        env!("CARGO_PKG_VERSION").into()
    }

    #[func]
    fn load_scenario(&mut self, scenario_json: GString, seed: i64) -> bool {
        let Ok(scenario) = from_str::<ScenarioDefinition>(&scenario_json.to_string()) else {
            return false;
        };
        self.battle = Some(Battle::new(&scenario, seed as u64));
        self.scenario = Some(scenario);
        self.publisher.resync();
        true
    }

    #[func]
    fn step_fixed(&mut self) -> i64 {
        self.battle
            .as_mut()
            .map_or(-1, |battle| battle.step() as i64)
    }

    #[func]
    fn accept_command(&mut self, command_json: GString) -> bool {
        let Some(battle) = self.battle.as_mut() else {
            return false;
        };
        let Ok(command) = from_str(&command_json.to_string()) else {
            return false;
        };
        battle.accept(command).error.is_none()
    }

    #[func]
    fn replay_json(&self) -> GString {
        self.battle.as_ref().map_or_else(
            || "".into(),
            |battle| {
                GString::from(&serde_json::to_string(&battle.replay()).expect("replay serializes"))
            },
        )
    }

    #[func]
    fn load_replay(&mut self, replay_json: GString) -> bool {
        let (Some(scenario), Ok(replay)) = (
            self.scenario.as_ref(),
            from_str::<Replay>(&replay_json.to_string()),
        ) else {
            return false;
        };
        let Ok(battle) = Battle::from_replay(scenario, &replay) else {
            return false;
        };
        self.battle = Some(battle);
        self.publisher.resync();
        true
    }

    #[func]
    fn tick(&self) -> i64 {
        self.battle
            .as_ref()
            .map_or(-1, |battle| battle.tick() as i64)
    }

    #[func]
    fn digest(&self) -> GString {
        self.battle.as_ref().map_or_else(
            || "".into(),
            |battle| {
                let digest = format!("{:016x}", battle.digest());
                GString::from(&digest)
            },
        )
    }

    #[func]
    fn publish_side(&mut self, side: GString) -> PackedFloat32Array {
        let Ok(side) = from_str::<Side>(&format!("\"{side}\"")) else {
            return PackedFloat32Array::new();
        };
        let Some(battle) = self.battle.as_ref() else {
            return PackedFloat32Array::new();
        };
        self.publisher
            .publish(battle, side)
            .map_or_else(|_| PackedFloat32Array::new(), PackedFloat32Array::from)
    }
}
