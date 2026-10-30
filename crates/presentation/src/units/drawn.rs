//! Where parts of each posed model are drawn, read from
//! the pose frame the models are drawn from, so an effect put there sits on
//! the drawn model however the simulation's own model differs:
//! - its muzzles: a vehicle mount's muzzle node under its articulation and
//!   its whole-model motion, and a soldier's weapon `muzzle` socket in his
//!   clip and kick, found only for the muzzles asked after (a frame's live
//!   flashes) and kept for the frame;
//! - its rotors: each one's hub, axis, blades and the angle it turned since
//!   the last drawn frame, from which the effects draw its blur.
use std::collections::HashMap;
use std::sync::Arc;

use super::articulation::articulate;
use super::pose::{Articulation, PoseFrame, SoldierPose, VehiclePose};
use super::skin::{drawn_worlds, mul, node_worlds, trs, Mat4};
use crate::effects::{DrawnRotor, Mast, MuzzleSource};
use crate::seed::side_key;
use crate::world::{Appearance, Trs, UnitsSection, Vec3};

/// Each skinned body's dense clip table (raw f32 samples), by table name.
pub type ClipTables = HashMap<String, Vec<f32>>;

/// A model-space point placed at `position`, turned by `yaw` about +Z.
fn placed(position: Vec3, yaw: f64, x: f64, y: f64, z: f64) -> Vec3 {
    let (s, c) = libm::sincos(yaw);
    [
        position[0] + x * c - y * s,
        position[1] + x * s + y * c,
        position[2] + z,
    ]
}

fn normalized(v: Vec3) -> Vec3 {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length > 0.0 {
        v.map(|c| c / length)
    } else {
        v
    }
}

/// One key per (side, id): blue's ids even.
fn key_of(side: &str, id: f64) -> u64 {
    side_key(id, side == "blue").to_bits()
}

/// Where soldier `s`'s body draws the `muzzle` socket of his weapon `mount`,
/// while that weapon is in use: his clip crossfaded and kicked, posed as his
/// palette is ([`drawn_worlds`]). `worlds` is scratch.
pub fn soldier_muzzle(
    units: &UnitsSection,
    tables: &ClipTables,
    s: &SoldierPose,
    mount: usize,
    worlds: &mut Vec<Mat4>,
) -> Option<Vec3> {
    if s.active_mount != Some(mount) {
        return None;
    }
    let name = units.resolve(
        &s.kind,
        &s.side,
        s.soldier,
        s.slot,
        s.operator_mount,
        s.active_mount,
    )?;
    let Appearance::Skinned {
        joints,
        clips,
        table,
        muzzle: Some(muzzle),
        ..
    } = units.appearances.get(name)?
    else {
        return None;
    };
    drawn_worlds(
        joints,
        clips,
        tables.get(table)?,
        &s.clip,
        s.phase,
        s.blend.as_ref(),
        s.kick,
        worlds,
    );
    let o = &muzzle.offset;
    let m = mul(worlds.get(muzzle.joint)?, &trs(o.t, o.r, o.s));
    Some(placed(s.position, s.facing, m[12], m[13], m[14]))
}

/// Where vehicle `v` draws the muzzle of its mount `mount`, with that muzzle
/// node's name: its rig's muzzle node for the mount's role, articulated and
/// moved (a lean, a hull's kick) as drawn. `locals` and `worlds` are scratch.
pub fn vehicle_muzzle<'a>(
    units: &'a UnitsSection,
    v: &VehiclePose,
    mount: usize,
    locals: &mut Vec<Trs>,
    worlds: &mut Vec<Mat4>,
) -> Option<(&'a str, Vec3)> {
    let name = units.resolve(&v.kind, &v.side, v.unit, 0, None, None)?;
    let Appearance::Articulated { nodes, rig, .. } = units.appearances.get(name)? else {
        return None;
    };
    let role = units.types.get(&*v.kind)?.roles.get(mount)?;
    let &node = rig.muzzles.get(role)?;
    locals.resize(nodes.len(), nodes.first()?.bind);
    articulate(locals, nodes, rig, &v.articulation);
    node_worlds(nodes, locals, v.motion.as_ref(), worlds);
    let m = worlds.get(node)?;
    Some((
        &nodes[node].name,
        placed(v.position, v.yaw, m[12], m[13], m[14]),
    ))
}

pub struct DrawnModels {
    units: Arc<UnitsSection>,
    tables: Arc<ClipTables>,
    /// The presented side: a shooter key's even ids are its.
    side: Arc<str>,
    enemy: Arc<str>,
    vehicles: Vec<VehiclePose>,
    soldiers: Vec<SoldierPose>,
    /// Each pose's place in its list, by side key.
    vehicle_at: HashMap<u64, usize>,
    soldier_at: HashMap<u64, usize>,
    /// This frame's answers by (soldier asked, side key, mount): a muzzle
    /// point, or None where nothing is drawn.
    memo: HashMap<(bool, u64, usize), Option<Vec3>>,
    /// Each vehicle's rotor tip travel as last drawn, and how far it moved
    /// since the frame before.
    rotor_at: HashMap<u64, f64>,
    rotor_turned: HashMap<u64, f64>,
    locals: Vec<Trs>,
    worlds: Vec<Mat4>,
}

impl DrawnModels {
    pub fn new(units: Arc<UnitsSection>, tables: Arc<ClipTables>, side: &str) -> Self {
        Self {
            units,
            tables,
            side: side.into(),
            enemy: if side == "blue" { "red" } else { "blue" }.into(),
            vehicles: Vec::new(),
            soldiers: Vec::new(),
            vehicle_at: HashMap::new(),
            soldier_at: HashMap::new(),
            memo: HashMap::new(),
            rotor_at: HashMap::new(),
            rotor_turned: HashMap::new(),
            locals: Vec::new(),
            worlds: Vec::new(),
        }
    }

    /// Replace art facts without dropping observed poses or rotor travel.
    pub fn replace_catalog(&mut self, units: Arc<UnitsSection>, tables: Arc<ClipTables>) {
        self.units = units;
        self.tables = tables;
        self.memo.clear();
    }

    /// Take the pose frame the models are drawn from this frame.
    pub fn update(&mut self, frame: &PoseFrame) {
        self.memo.clear();
        self.vehicles.clone_from(&frame.vehicles);
        self.soldiers.clone_from(&frame.soldiers);
        self.vehicle_at.clear();
        self.rotor_turned.clear();
        for (i, v) in self.vehicles.iter().enumerate() {
            let key = key_of(&v.side, v.unit);
            self.vehicle_at.insert(key, i);
            let was = self.rotor_at.get(&key);
            self.rotor_turned
                .insert(key, was.map_or(0.0, |was| v.articulation.rotor - was));
        }
        self.rotor_at.clear();
        for (&key, &i) in &self.vehicle_at {
            self.rotor_at
                .insert(key, self.vehicles[i].articulation.rotor);
        }
        self.soldier_at.clear();
        for (i, s) in self.soldiers.iter().enumerate() {
            self.soldier_at.insert(key_of(&s.side, s.soldier), i);
        }
    }

    /// The drawn muzzle of the vehicle at side key `key`'s mount `mount`.
    fn vehicle_muzzle(&mut self, key: u64, mount: usize) -> Option<Vec3> {
        let v = &self.vehicles[*self.vehicle_at.get(&key)?];
        vehicle_muzzle(&self.units, v, mount, &mut self.locals, &mut self.worlds).map(|m| m.1)
    }

    /// The drawn muzzle of the soldier at side key `key`'s weapon `mount`.
    fn soldier_muzzle(&mut self, key: u64, mount: usize) -> Option<Vec3> {
        let s = &self.soldiers[*self.soldier_at.get(&key)?];
        soldier_muzzle(&self.units, &self.tables, s, mount, &mut self.worlds)
    }

    /// Each drawn rotor of this frame's aircraft, posed as the model is.
    pub fn rotors(&mut self) -> Vec<DrawnRotor> {
        let mut out = Vec::new();
        for (v, key) in self.vehicles.iter().map(|v| (v, key_of(&v.side, v.unit))) {
            if !v.airborne {
                continue;
            }
            let Some(name) = self.units.resolve(&v.kind, &v.side, v.unit, 0, None, None) else {
                continue;
            };
            let Some(Appearance::Articulated { nodes, rig, .. }) = self.units.appearances.get(name)
            else {
                continue;
            };
            if rig.rotors.is_empty() {
                continue;
            }
            // Posed with its rotors shown: the blur hides them, not their place.
            let input = Articulation {
                rotor_blur: 0.0,
                ..v.articulation
            };
            self.locals.resize(nodes.len(), nodes[0].bind);
            articulate(&mut self.locals, nodes, rig, &input);
            // Its whole-model motion carries every rotor, as it is drawn.
            node_worlds(nodes, &self.locals, v.motion.as_ref(), &mut self.worlds);
            let tip_travel = self.rotor_turned.get(&key).copied().unwrap_or(0.0);
            for rotor in &rig.rotors {
                let m = self.worlds[rotor.node];
                let (s, c) = libm::sincos(rotor.rest);
                let along = |d: f64| {
                    placed(
                        v.position,
                        v.yaw,
                        m[12] + m[8] * d,
                        m[13] + m[9] * d,
                        m[14] + m[10] * d,
                    )
                };
                out.push(DrawnRotor {
                    hub: along(0.0),
                    axis: normalized(placed([0.0; 3], v.yaw, m[8], m[9], m[10])),
                    blade: normalized(placed(
                        [0.0; 3],
                        v.yaw,
                        m[0] * c + m[4] * s,
                        m[1] * c + m[5] * s,
                        m[2] * c + m[6] * s,
                    )),
                    radius: rotor.radius,
                    blades: rotor.blades,
                    chord: rotor.chord,
                    turned: tip_travel / rotor.radius,
                    mast: (rotor.mast_radius > 0.0).then(|| Mast {
                        base: along(-rotor.mast_below),
                        top: along(rotor.mast_above),
                    }),
                    mast_radius: rotor.mast_radius,
                });
            }
        }
        out
    }
}

impl MuzzleSource for DrawnModels {
    /// `shooter` is a side key with the presented side's ids even (as the
    /// effect publication keys them).
    fn muzzle(&mut self, shooter: f64, mount: usize, soldier: Option<f64>) -> Option<Vec3> {
        let side = if shooter % 2.0 == 0.0 {
            &self.side
        } else {
            &self.enemy
        };
        let key = match soldier {
            Some(id) => key_of(side, id),
            None => key_of(side, (shooter / 2.0).floor()),
        };
        let asked = (soldier.is_some(), key, mount);
        if let Some(&answer) = self.memo.get(&asked) {
            return answer;
        }
        let answer = if soldier.is_some() {
            self.soldier_muzzle(key, mount)
        } else {
            self.vehicle_muzzle(key, mount)
        };
        self.memo.insert(asked, answer);
        answer
    }
}
