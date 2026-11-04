use super::scenario;
use contract::ids::UnitId;
use serde_json::json;
use sim::air::FixedWingLimits;
use sim::battle::Battle;
use sim::math::V3;

/// Small authored-like field for fixed-wing flight tests. It keeps setup in the
/// shared test catalog, so tests do not duplicate battle construction.
pub struct AirField {
    size: f64,
    units: Vec<serde_json::Value>,
}

impl AirField {
    pub fn new(size_m: f64) -> Self {
        Self {
            size: size_m,
            units: Vec::new(),
        }
    }

    pub fn aircraft(mut self, kind: &str, at: [f64; 2], heading: f64) -> Self {
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

/// The edge margin for the narrow fields of the order and reaction tests,
/// where a jet's turn circles would not fit inside the shipped margin.
const NARROW_FIELD_MARGIN_M: f64 = 50.0;
/// The roll-in for the narrow fields: the fast-turning test jet rolls into
/// its turns this quickly, so they fit there.
pub const NARROW_FIELD_ROLL_IN_S: f64 = 0.8;

/// The test rules for a narrow field, where the edge guard and the jet's
/// roll-in, not these tests' subjects, are kept out of the way.
pub fn narrow_field_rules() -> serde_json::Value {
    let mut rules = sim::fixtures::test_rules();
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(NARROW_FIELD_MARGIN_M);
    rules["air"]["fixed_wing"]["roll_in_s"] = json!(NARROW_FIELD_ROLL_IN_S);
    rules
}

/// A fixed-wing airframe's limits for driving the flight law directly.
pub fn wing_limits() -> FixedWingLimits {
    FixedWingLimits {
        cruise_mps: 100.0,
        burner_mult: 1.5,
        turn_rad_s: 1.0,
        roll_in_s: 0.8,
        climb_mps: 30.0,
        ramp_s: 2.0,
        floor_agl_m: 100.0,
        edge_margin_m: 300.0,
        cruise_agl_m: 250.0,
        nose_on_rad: 2_f64.to_radians(),
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
            let unit = battle.unit(UnitId(0)).expect("the placed aircraft");
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
