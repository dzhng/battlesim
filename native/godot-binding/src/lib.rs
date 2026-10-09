use godot::prelude::*;

struct BattleGameExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BattleGameExtension {}

#[derive(GodotClass)]
#[class(base=Node)]
struct SimulationProbe {
    base: Base<Node>,
}

#[godot_api]
impl INode for SimulationProbe {
    fn init(base: Base<Node>) -> Self {
        Self { base }
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
}

