//! Posed units as the frame's models: where a soldier's clip and a vehicle's
//! articulation become palettes and instances, for a battle and a showcase
//! alike.
use super::*;
use crate::units::pose::{SoldierPose, VehiclePose};
use crate::world::Detail;

/// What units are drawn from: their section, each skinned body's clip table
/// and inverse binds, and the world rows their appearances hold; and the
/// view and detail this frame draws them at.
pub(super) struct UnitArt<'a> {
    pub units: &'a UnitsSection,
    pub tables: &'a ClipTables,
    pub inverse_binds: &'a HashMap<String, Vec<Mat4>>,
    pub world: &'a World,
    pub view: &'a View,
    pub detail: Detail,
}

/// The node transforms posing reuses from model to model.
#[derive(Default)]
pub(super) struct Scratch {
    pub locals: Vec<Trs>,
    pub worlds: Vec<Mat4>,
}

/// Each skinned appearance's inverse binds, by name.
pub(super) fn inverse_binds(units: &UnitsSection) -> HashMap<String, Vec<Mat4>> {
    let mut binds = HashMap::new();
    for (name, a) in &units.appearances {
        if let Appearance::Skinned {
            joints,
            inverse_binds: given,
            ..
        } = a
        {
            binds.insert(name.clone(), skin::inverse_binds(joints, given));
        }
    }
    binds
}

/// The rows one unit appearance draws in: a body and each way it lies dead,
/// or an articulated model.
pub(super) fn register_appearance(world: &mut World, name: &str, a: &Appearance) {
    match a {
        Appearance::Skinned {
            joints,
            corpse_poses,
            ..
        } => {
            world.row(name, "", NONE, joints.len() as u32, false);
            for lies in 0..corpse_poses.len() {
                world.row(name, "", lies as u32, 0, false);
            }
        }
        Appearance::Articulated { nodes, .. } => {
            world.row(name, "", NONE, nodes.len() as u32, false);
        }
    }
}

impl Model {
    /// A unit's model: fogged as units are, unscaled, unhighlighted, untinted
    /// and unposed.
    pub(super) fn unit(row: u32, tier: i32, pass: u32, at: Vec3, yaw: f64) -> Self {
        Self {
            row,
            tier: tier as u32,
            pass,
            at,
            yaw,
            tint: [1.0; 3],
            highlight: false,
            scale: [1.0; 3],
            colour: [0.0; 4],
            palette: 0,
            scroll: [0.0; 2],
            fog: model_fog(false, false, false),
        }
    }
}

impl UnitArt<'_> {
    pub(super) fn tint(&self, side: &str) -> [f32; 3] {
        self.units
            .tints
            .get(side)
            .map_or([1.0; 3], |t| t.map(|v| v as f32))
    }

    pub(super) fn row(&self, name: &str, state: &str, lies: u32) -> u32 {
        self.world.rows[&(name.to_owned(), state.to_owned(), lies, 0, false)]
    }

    /// Living and dying soldiers, each tiered by his body's far bounds and
    /// coloured by `colour` (his side, his unit); how many are off screen.
    pub(super) fn soldiers(
        &self,
        soldiers: &[SoldierPose],
        colour: impl Fn(&str, f64) -> [f64; 4],
        worlds: &mut Vec<Mat4>,
        f: &mut Frame,
        models: &mut Vec<Model>,
    ) -> u32 {
        let units = self.units;
        let mut culled = 0;
        for s in soldiers {
            let Some(name) = units.resolve(
                &s.kind,
                &s.side,
                s.soldier,
                s.slot,
                s.operator_mount,
                s.active_mount,
            ) else {
                continue;
            };
            f.counts.soldiers += 1;
            let far = bounds(units, name, 0);
            let tier = model_detail(
                &self.detail,
                self.view,
                s.position,
                far.size,
                far.radius,
                true,
                None,
            );
            if tier == CULLED {
                culled += 1;
                continue;
            }
            let palette = if tier == IMPOSTOR {
                0
            } else {
                pose_body(
                    units,
                    self.tables,
                    self.inverse_binds,
                    worlds,
                    &mut f.palettes,
                    name,
                    &s.clip,
                    s.phase,
                    s.blend.as_ref(),
                    s.kick,
                )
            };
            models.push(Model {
                tint: self.tint(&s.side),
                colour: colour(&s.side, s.unit),
                palette,
                ..Model::unit(self.row(name, "", NONE), tier, 0, s.position, s.facing)
            });
        }
        culled
    }

    /// Vehicles and aircraft, coloured by `colour` (side, unit). `shadow`
    /// says where each one's shadow lies, if it draws while only its shadow
    /// is in view, and the pass that draws it.
    pub(super) fn vehicles(
        &self,
        vehicles: &[VehiclePose],
        shadow: impl Fn(&VehiclePose) -> (Option<Vec3>, u32),
        colour: impl Fn(&str, f64) -> [f64; 4],
        scratch: &mut Scratch,
        f: &mut Frame,
        models: &mut Vec<Model>,
    ) {
        let units = self.units;
        for v in vehicles {
            let Some(name) = units.resolve(&v.kind, &v.side, v.unit, 0, None, None) else {
                continue;
            };
            if v.airborne {
                f.counts.aircraft += 1;
            } else {
                f.counts.vehicles += 1;
            }
            let far = bounds(units, name, 0);
            let (shadow, pass) = shadow(v);
            let tier = model_detail(
                &self.detail,
                self.view,
                v.position,
                far.size,
                far.radius,
                false,
                shadow,
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = pose_vehicle(
                units,
                scratch,
                &mut f.palettes,
                name,
                &v.articulation,
                v.motion.as_ref(),
            );
            models.push(Model {
                tint: self.tint(&v.side),
                colour: colour(&v.side, v.unit),
                palette,
                scroll,
                ..Model::unit(self.row(name, "", NONE), tier, pass, v.position, v.yaw)
            });
        }
    }
}

/// Skinned appearance `name` as it holds still: lying `lies` (a
/// `corpse_poses` entry), or in its far pose for `NONE`. Each joint's
/// bind-to-model motion; none for a rigid appearance.
pub(super) fn rest_palette(
    units: &UnitsSection,
    tables: &ClipTables,
    inverse_binds: &HashMap<String, Vec<Mat4>>,
    name: &str,
    lies: u32,
) -> Option<Vec<Joint>> {
    let Some(Appearance::Skinned {
        joints,
        clips,
        table,
        far_pose,
        corpse_poses,
        ..
    }) = units.appearances.get(name)
    else {
        return None;
    };
    let pose = match lies {
        NONE => far_pose,
        lies => corpse_poses.get(lies as usize)?,
    };
    let mut worlds = Vec::new();
    let table = tables.get(table)?;
    skin::joint_worlds(
        joints,
        clips,
        table,
        &pose.clip,
        pose.phase,
        None,
        &mut worlds,
    );
    let ibs = &inverse_binds[name];
    Some(
        worlds
            .iter()
            .zip(ibs)
            .map(|(w, ib)| joint(&skin::affine(&skin::mul(w, ib))))
            .collect(),
    )
}

/// A body in a clip at a phase, crossfaded and kicked: its palette's first joint.
#[allow(clippy::too_many_arguments)]
pub(super) fn pose_body(
    units: &UnitsSection,
    tables: &ClipTables,
    inverse_binds: &HashMap<String, Vec<Mat4>>,
    worlds: &mut Vec<Mat4>,
    palettes: &mut Vec<Joint>,
    name: &str,
    clip: &str,
    phase: f64,
    blend: Option<&ClipBlend>,
    kick: Kick,
) -> u32 {
    let Some(Appearance::Skinned {
        joints,
        clips,
        table,
        ..
    }) = units.appearances.get(name)
    else {
        return 0;
    };
    let Some(table) = tables.get(table) else {
        return 0;
    };
    skin::drawn_worlds(joints, clips, table, clip, phase, blend, kick, worlds);
    let first = palettes.len() as u32;
    let ibs = &inverse_binds[name];
    palettes.extend(
        worlds
            .iter()
            .zip(ibs)
            .map(|(w, ib)| joint(&skin::affine(&skin::mul(w, ib)))),
    );
    first
}

/// A vehicle articulated, its whole model moved by `motion` (an aircraft's
/// lean, a hull's kick): its palette's first joint and its track scrolls.
pub(super) fn pose_vehicle(
    units: &UnitsSection,
    scratch: &mut Scratch,
    palettes: &mut Vec<Joint>,
    name: &str,
    articulation: &Articulation,
    motion: Option<&Mat4>,
) -> (u32, [f64; 2]) {
    let Some(Appearance::Articulated { nodes, rig, .. }) = units.appearances.get(name) else {
        return (0, [0.0; 2]);
    };
    let Scratch { locals, worlds } = scratch;
    *locals = rest_locals(nodes);
    articulate(locals, nodes, rig, articulation);
    skin::node_worlds(nodes, locals, motion, worlds);
    let first = palettes.len() as u32;
    palettes.extend(worlds.iter().map(|w| joint(&skin::affine(w))));
    (first, track_scroll(rig, articulation))
}
