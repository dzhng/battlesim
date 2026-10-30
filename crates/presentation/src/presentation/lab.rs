//! External tooling models composed into the ordinary presentation frame.
use super::*;
use crate::assets::{Container, BUNDLE};
use crate::records::FOG_UNITS;
use crate::static_world::props::{DRAWS_PROP, DRAWS_PROP_GROUND};
use crate::world::{Corners, Detail};
use std::collections::BTreeMap;

/// Geometry facts and authored poses, independent of a battle roster.
pub enum Asset {
    Animated {
        appearance: Box<Appearance>,
        table: Vec<f32>,
    },
    Static(Vec<State>),
}

pub struct State {
    name: String,
    tiers: u32,
    bounds: Bounds,
    corners: Corners,
}

impl Asset {
    /// Reads the same transported bundles as the renderer. Textures remain
    /// with the GPU asset owner; only geometry/pose facts are retained here.
    pub fn from_bundle(bytes: &[u8], skeleton: Option<&[u8]>) -> Result<Self, String> {
        let bundle = Container::<serde_json::Value>::parse(bytes, BUNDLE, "lab model")?;
        let (mut value, table) = match bundle.content["kind"].as_str() {
            Some("skinned") => {
                let skeleton = Container::parse(
                    skeleton.ok_or("lab model: missing skeleton")?,
                    BUNDLE,
                    "lab skeleton",
                )?;
                crate::appearance::skinned(&bundle, &skeleton)?
            }
            Some("articulated") => (crate::appearance::articulated(&bundle)?, Vec::new()),
            _ => {
                let states = bundle.content["states"]
                    .as_array()
                    .ok_or("lab model: no states")?;
                return states
                    .iter()
                    .map(|s| {
                        let corners: Corners = serde_json::from_value(s["bounds"].clone())
                            .map_err(|e| e.to_string())?;
                        Ok(State {
                            name: s["name"].as_str().ok_or("lab model: unnamed state")?.into(),
                            tiers: s["tiers"].as_array().ok_or("lab model: no tiers")?.len() as u32,
                            bounds: corners.bounds(),
                            corners,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()
                    .map(Self::Static);
            }
        };
        value["table"] = serde_json::json!("lab");
        Ok(Self::Animated {
            appearance: Box::new(serde_json::from_value(value).map_err(|e| e.to_string())?),
            table,
        })
    }

    pub(super) fn state_corners(&self, state: &str) -> Option<Corners> {
        let Self::Static(states) = self else {
            return None;
        };
        states.iter().find(|s| s.name == state).map(|s| s.corners)
    }
    pub(super) fn first_corners(&self) -> Option<Corners> {
        let Self::Static(states) = self else {
            return None;
        };
        states.first().map(|s| s.corners)
    }

    pub(super) fn palette(&self, pose: &Pose) -> Option<(Vec<Joint>, [f64; 2])> {
        match pose {
            Pose::Posed { palette, scroll } => Some((
                palette
                    .as_chunks::<12>()
                    .0
                    .iter()
                    .map(|j| Joint {
                        x: j[..4].try_into().unwrap(),
                        y: j[4..8].try_into().unwrap(),
                        z: j[8..].try_into().unwrap(),
                    })
                    .collect(),
                *scroll,
            )),
            Pose::Static { motion, .. } => Some((
                motion
                    .map(|m| vec![joint(&skin::affine(&m))])
                    .unwrap_or_default(),
                [0.0; 2],
            )),
            Pose::Corpse { .. } => Some((Vec::new(), [0.0; 2])),
            _ => {
                let Self::Animated { appearance, table } = self else {
                    return None;
                };
                match appearance.as_ref() {
                    Appearance::Skinned {
                        joints,
                        inverse_binds,
                        clips,
                        far_pose,
                        ..
                    } => {
                        let (clip, phase) = match pose {
                            Pose::Clip { clip, phase } => (clip, *phase),
                            Pose::Rest => (&far_pose.clip, far_pose.phase),
                            _ => return None,
                        };
                        if !clips.contains_key(clip) {
                            return None;
                        }
                        let mut worlds = Vec::new();
                        skin::joint_worlds(joints, clips, table, clip, phase, None, &mut worlds);
                        let inverse = skin::inverse_binds(joints, inverse_binds);
                        Some((
                            worlds
                                .iter()
                                .zip(inverse)
                                .map(|(w, i)| joint(&skin::affine(&skin::mul(w, &i))))
                                .collect(),
                            [0.0; 2],
                        ))
                    }
                    Appearance::Articulated { nodes, rig, .. } => {
                        let articulation = match pose {
                            Pose::Articulated { articulation } => *articulation,
                            Pose::Rest => Articulation::default(),
                            _ => return None,
                        };
                        let mut locals = rest_locals(nodes);
                        articulate(&mut locals, nodes, rig, &articulation);
                        let mut worlds = Vec::new();
                        skin::node_worlds(nodes, &locals, None, &mut worlds);
                        Some((
                            worlds.iter().map(|w| joint(&skin::affine(w))).collect(),
                            track_scroll(rig, &articulation),
                        ))
                    }
                }
            }
        }
    }

    fn info(&self, pose: &Pose) -> Option<(Bounds, u32, bool)> {
        match self {
            Self::Static(states) => {
                let Pose::Static { state, .. } = pose else {
                    return None;
                };
                let s = states.iter().find(|s| s.name == *state)?;
                Some((s.bounds, s.tiers, false))
            }
            Self::Animated { appearance, .. } => match appearance.as_ref() {
                Appearance::Skinned {
                    bounds,
                    tiers,
                    card,
                    corpse_poses,
                    ..
                } => {
                    let at = match pose {
                        Pose::Corpse { lies } => {
                            corpse_poses.get(*lies as usize)?;
                            *lies as usize + 1
                        }
                        _ => 0,
                    };
                    Some((*bounds.get(at)?, *tiers, *card))
                }
                Appearance::Articulated { bounds, tiers, .. } => {
                    Some((*bounds.first()?, *tiers, false))
                }
            },
        }
    }
}

/// Page requests name installed art and supply a transform and pose. No
/// simulation identity, shader, device or renderer handle crosses this seam.
#[derive(Deserialize)]
pub struct ModelInput {
    pub appearance: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    #[serde(default = "unit_scale")]
    pub scale: [f32; 3],
    #[serde(default = "unit_scale")]
    pub tint: [f32; 3],
    pub pose: Pose,
    pub tier: Option<u32>,
    /// Mesh-only selection cue. Highlighted models cannot select a card.
    #[serde(default)]
    pub highlight: bool,
    pub xray: Option<[f64; 4]>,
    pub ghost: Option<[f64; 4]>,
}
fn unit_scale() -> [f32; 3] {
    [1.0; 3]
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Pose {
    Posed {
        palette: Vec<f32>,
        #[serde(default)]
        scroll: [f64; 2],
    },
    Static {
        state: String,
        motion: Option<Mat4>,
        #[serde(default)]
        ground: bool,
    },
    Corpse {
        lies: u32,
    },
    Rest,
    Clip {
        clip: String,
        phase: f64,
    },
    Articulated {
        articulation: Articulation,
    },
}

#[derive(Default)]
pub(super) struct Lab {
    pub(super) assets: BTreeMap<String, Asset>,
    pub(super) detail: Option<Detail>,
    pub(super) models: Vec<ModelInput>,
    /// Canonical battle/static rows and strings precede replaceable tooling rows.
    pub(super) frontier: Option<(usize, usize)>,
}

impl Presentation {
    /// Replaces the tooling catalog as one generation, without changing the
    /// battle, observations or static world. Reinstall the renderer's tables
    /// after this call; world rows can have moved.
    pub fn install_lab_assets(&mut self, assets: BTreeMap<String, Asset>, detail: Detail) {
        self.catalog_revision = super::catalog::next_revision();
        self.lab.assets = assets;
        self.lab.detail = Some(detail);
        self.register_lab_rows();
    }

    pub fn set_lab_models(&mut self, models: Vec<ModelInput>) -> Result<(), String> {
        validate_models(&models)?;
        self.lab.models = models;
        Ok(())
    }

    pub(super) fn clear_lab_rows(&mut self) {
        if let Some((rows, strings)) = self.lab.frontier.take() {
            self.world.appearances.truncate(rows);
            self.world.strings.truncate(strings);
            self.world.rows.retain(|_, row| (*row as usize) < rows);
        }
    }

    pub(super) fn register_lab_rows(&mut self) {
        self.clear_lab_rows();
        self.lab.frontier = Some((self.world.appearances.len(), self.world.strings.len()));
        register_rows(&mut self.world, &self.lab.assets);
    }

    pub(super) fn lab_rest_palette(&self, row: u32) -> Option<Vec<Joint>> {
        let r = self.world.appearances.get(row as usize)?;
        let name = self.world.name(r);
        let asset = self.lab.assets.get(name)?;
        super::catalog::asset_rest_palette(asset, r.lies)
    }
}

impl Lab {
    pub(super) fn append(
        &self,
        view: &View,
        world: &World,
        f: &mut Frame,
        models: &mut Vec<Model>,
    ) {
        let Some(detail) = self.detail else {
            return;
        };
        for m in &self.models {
            let Some(asset) = self.assets.get(&m.appearance) else {
                continue;
            };
            let Some((bounds, tiers, card)) = asset.info(&m.pose) else {
                continue;
            };
            if tiers == 0 {
                continue;
            }
            let (state, lies, ground) = match &m.pose {
                Pose::Static { state, ground, .. } => (state.as_str(), NONE, *ground),
                Pose::Corpse { lies } => ("", *lies, false),
                _ => ("", NONE, false),
            };
            let fog = model_fog(lies != NONE, !state.is_empty(), ground);
            let draws = if state.is_empty() {
                0
            } else if ground {
                DRAWS_PROP_GROUND
            } else {
                DRAWS_PROP
            };
            let Some(&row) =
                world
                    .rows
                    .get(&(m.appearance.clone(), state.to_owned(), lies, draws, false))
            else {
                continue;
            };
            let scale = m
                .scale
                .iter()
                .map(|v| v.abs() as f64)
                .fold(0.0f64, f64::max);
            let motion_reach = match &m.pose {
                Pose::Static {
                    motion: Some(m), ..
                } => crate::world::length([m[12], m[13], m[14]]),
                _ => 0.0,
            };
            let at = [m.x, m.y, m.z];
            let has_card = card && m.ghost.is_none() && !m.highlight;
            let tier = match m.tier {
                Some(t) if t == IMPOSTOR as u32 && has_card => t as i32,
                Some(t) => t.min(tiers - 1) as i32,
                None => model_detail(
                    &detail,
                    view,
                    at,
                    bounds.size * scale,
                    (bounds.radius + motion_reach) * scale,
                    has_card,
                    None,
                ),
            };
            if tier == CULLED {
                f.counts.culled += 1;
                continue;
            }
            let Some((palette, scroll)) = asset.palette(&m.pose) else {
                continue;
            };
            let base = if palette.is_empty() {
                0
            } else {
                f.palettes.len() as u32
            };
            f.palettes.extend(palette);
            let colour = m.ghost.unwrap_or_else(|| {
                if fog == FOG_UNITS {
                    m.xray.unwrap_or([0.0; 4])
                } else {
                    [0.0; 4]
                }
            });
            models.push(Model {
                row,
                tier: if tier == IMPOSTOR {
                    IMPOSTOR as u32
                } else {
                    (tier as u32).min(tiers - 1)
                },
                pass: if m.ghost.is_some() {
                    GHOST_PASS
                } else {
                    VIEW_PASS
                },
                at,
                yaw: m.yaw,
                tint: m.tint,
                highlight: m.highlight,
                scale: m.scale,
                colour,
                palette: base,
                scroll,
                fog,
            });
        }
    }
}

impl Presentation {
    pub fn rest_palette(&self, row: u32) -> Option<Vec<Joint>> {
        self.battle_rest_palette(row)
            .or_else(|| self.lab_rest_palette(row))
    }
    /// A skinned row's body as it holds still (`World.appearances`): a
    /// corpse row as it lies (its `corpse_poses` entry), a body row in its
    /// far pose (what its impostor card shows). Each joint's bind-to-model
    /// motion: a renderer skins the body's mesh by it once, when it loads
    /// the row (`posedMesh`), and draws that mesh where the frame's
    /// instance takes the identity. None for a rigid row, or before a world
    /// loads.
    fn battle_rest_palette(&self, row: u32) -> Option<Vec<Joint>> {
        let loaded = self.loaded.as_ref()?;
        let r = self.world.appearances.get(row as usize)?;
        let name = self.world.name(r);
        let Some(Appearance::Skinned {
            joints,
            clips,
            table,
            far_pose,
            corpse_poses,
            ..
        }) = loaded.units.appearances.get(name)
        else {
            return None;
        };
        let pose = match r.lies {
            NONE => far_pose,
            lies => corpse_poses.get(lies as usize)?,
        };
        let mut worlds = Vec::new();
        let table = loaded.tables.get(table)?;
        skin::joint_worlds(
            joints,
            clips,
            table,
            &pose.clip,
            pose.phase,
            None,
            &mut worlds,
        );
        let ibs = &loaded.inverse_binds[name];
        Some(
            worlds
                .iter()
                .zip(ibs)
                .map(|(w, ib)| joint(&skin::affine(&skin::mul(w, ib))))
                .collect(),
        )
    }
}

pub(super) fn validate_models(models: &[ModelInput]) -> Result<(), String> {
    for m in models {
        if let Pose::Posed { palette, .. } = &m.pose {
            if palette.len() % 12 != 0 {
                return Err(format!(
                    "{}: palette must contain complete Joint records",
                    m.appearance
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn register_rows(world: &mut World, assets: &BTreeMap<String, Asset>) {
    for (name, asset) in assets {
        match asset {
            Asset::Static(states) => {
                for s in states {
                    for draws in [DRAWS_PROP, DRAWS_PROP_GROUND] {
                        world.drawn_row(name, &s.name, NONE, 0, false, draws);
                    }
                }
            }
            Asset::Animated { appearance, .. } => match appearance.as_ref() {
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
            },
        }
    }
}
