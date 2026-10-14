use super::scenario;
use contract::ids::UnitId;
use serde_json::json;
use sim::battle::Battle;
use sim::math::V3;

/// Small authored-like field for fixed-wing flight tests. It keeps setup in the
/// shared test catalog, so tests do not duplicate battle construction.
pub struct JetField {
    size: f64,
    units: Vec<serde_json::Value>,
}

impl JetField {
    pub fn new(size_m: f64) -> Self {
        Self {
            size: size_m,
            units: Vec::new(),
        }
    }

    pub fn jet(mut self, kind: &str, at: [f64; 2], heading: f64) -> Self {
        self.units.push(json!({
            "side": "blue", "kind": kind, "position": at, "yaw": heading,
            "engagement": "return_fire_only"
        }));
        self
    }

    pub fn battle(self) -> Battle {
        let size = self.size;
        let map = json!({
            "size": [size, size], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35
        });
        let setup = scenario(
            &serde_json::to_string(&map).unwrap(),
            serde_json::Value::Array(self.units),
            json!([]),
        );
        Battle::new(&setup, 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub pos: V3,
    pub vel: V3,
    pub agl: f64,
    pub tick: u64,
}

pub struct Track;

impl Track {
    pub fn record(battle: &mut Battle, ticks: u64) -> Vec<Sample> {
        let mut samples = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            battle.step();
            let unit = battle.unit(UnitId(0)).expect("jet unit");
            let pos = unit.position;
            let ground = battle.world().height_at(pos.x, pos.y).unwrap_or(0.0);
            let vel = unit.air.map(|air| air.velocity).unwrap_or_default();
            samples.push(Sample {
                pos,
                vel,
                agl: pos.z - ground,
                tick: battle.tick(),
            });
        }
        samples
    }
}
