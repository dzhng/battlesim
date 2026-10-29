//! Packs one side's observation into a flat f32 record the browser copies out
//! of WASM memory. The layout is published by [`layout_json`]; consumers never
//! hardcode offsets, strides or tags.
//!
//! Record: a complete header, then independent non-map groups carrying either
//! complete f32 words, group-local replacements, or exact source copies. A compact
//! arm stores their lossless bit representation in raw u32 carriers. Finally
//! visibility snapshots/indexed replacements carry exact 16-bit limbs, followed
//! by ground's exact tile-local runs and packed byte marks.
//!
//! [`Publisher`] retains one bounded side/epoch baseline outside the battle and
//! its digest. A new side, epoch or resync opens every delivery cursor together.
//!
//! Integers that grow without bound (soldier ids, shot counters) would lose
//! exactness in one float past 2²⁴, so they travel as two 16-bit limbs: a
//! field pair `<name>Lo`, `<name>Hi` holding `lo + hi · 2^limbBits`, both -1
//! when absent.
//!
//! A firing report names the weapon rows it was heard firing as one bit per
//! row, so the rules may hold at most [`MAX_WEAPON_ROWS`] (64) weapon rows:
//! the mask travels as three floats of 24 bits (`heardLow`, `heardMid`,
//! `heardHigh`), and rules with more rows are refused at load.
use contract::command::TargetRef;
use contract::ids::Side;
use contract::observation::{
    HostRef, MemberLean, ObservationFrame, ShoutSpeaker, ShoutTrigger, WeaponPose,
};
use contract::players::ControllerKind;
use contract::scenario::CoverTier;

use crate::battle::Battle;
use crate::ground::GroundRunPatch;

mod layout;
pub mod reader;
mod tags;

pub use layout::layout_json;
use tags::*;

/// Atomic publication allowance; uniform Large ground plus finest fog is about
/// 40.5 MB. Higher-entropy workloads fail admission explicitly, never lose cells.
pub const MAX_PUBLICATION_BYTES: usize = 64 * 1024 * 1024;

/// At most the fixed 18 km extent at the finest accepted 2 m fog grid.
/// A snapshot holds two exact 16-bit limbs per word: at most 20,250,000 bytes.
pub const MAX_FOG_WORDS: usize = 2_531_250;
const LIMB_BITS: u32 = 16;
/// Ammunition kinds per mount the record carries (the cannon's AP and HE).
pub const MAX_AMMO_KINDS: usize = 2;
/// Weapon rows a firing report's heard mask can name: one bit each of the
/// contact's `heard` word. It travels as `HEARD_WORDS` floats of
/// `HEARD_WORD_BITS` each, since a float holds an integer exactly below 2^24.
pub const MAX_WEAPON_ROWS: usize = u64::BITS as usize;
const HEARD_WORD_BITS: usize = 24;
const HEARD_WORDS: usize = MAX_WEAPON_ROWS.div_ceil(HEARD_WORD_BITS);
const MOUNT_FIELDS: [&str; 15] = [
    "mount",
    "loaded",
    "ammo0",
    "ammo1",
    "aim",
    "reload",
    "targetKind",
    "targetId",
    "targetX",
    "targetY",
    "targetZ",
    "reason",
    "kinds",
    "guiding",
    "reloadKind",
];

const POSE_FIELDS: [&str; 7] = [
    "mount",
    "bearing",
    "elevation",
    "shotsLo",
    "shotsHi",
    "operatorLo",
    "operatorHi",
];

/// Header words; the non-map groups' delivery metadata starts here.
pub const HEADER_WORDS: usize = HEADER.len();
const HEADER: [&str; 44] = [
    "tick",
    "ownCount",
    "identifiedCount",
    "contactCount",
    "audibleCount",
    "refusalCount",
    "knownPropCount",
    "projectileCount",
    "blastCount",
    "corpseCount",
    "guidedCount",
    "airTransitCount",
    "encounterHeldS",
    "encounterResult",
    "fogCellM",
    "fogNx",
    "fogNy",
    "fogFloats",
    "fogFull",
    "fogBaseLo",
    "fogBaseHi",
    "fogRevisionLo",
    "fogRevisionHi",
    "groundEpoch",
    "groundSide",
    "groundBase",
    "groundRevision",
    "groundFull",
    "groundRunCount",
    "fallenBodyCount",
    "skirmishPhase",
    "playersCount",
    "preparationRemainingS",
    "pendingCount",
    "reportTotalsCount",
    "reportEventCount",
    "objectiveCount",
    "scoreBlue",
    "scoreRed",
    "matchResult",
    "crashCount",
    "shoutCount",
    "propDisplayPoseCount",
    "grindCount",
];
const OBJECTIVE_FIELDS: [&str; 8] = [
    "id",
    "x",
    "y",
    "radius",
    "owner",
    "capturing",
    "captureProgress",
    "contested",
];
const PENDING_FIELDS: [&str; 11] = [
    "idLo",
    "idHi",
    "owner",
    "kind",
    "x",
    "y",
    "confirmedLo",
    "confirmedHi",
    "blocked",
    "arrivalLo",
    "arrivalHi",
];
const GROUND_FIELDS: [&str; 4] = ["tile", "span", "craterScorch", "tracksTrampledCleared"];
const CONTACT_FIELDS: [&str; 16] = [
    "id",
    "source",
    "x",
    "y",
    "z",
    "layer",
    "aloft",
    "radius",
    "evidenceTick",
    "expiresTick",
    "kind",
    "heardLow",
    "heardMid",
    "heardHigh",
    "bearingDeg",
    "rangeM",
];
const AUDIBLE_FIELDS: [&str; 5] = ["listener", "category", "sector", "band", "moving"];
const REFUSAL_FIELDS: [&str; 3] = ["unitLo", "unitHi", "reason"];
const PROJECTILE_FIELDS: [&str; 10] = [
    "pointCount",
    "ricochetCount",
    "own",
    "kind",
    "shooterLo",
    "shooterHi",
    "hit",
    "nx",
    "ny",
    "nz",
];
const BLAST_FIELDS: [&str; 5] = ["x", "y", "z", "radius", "kind"];
const GUIDED_FIELDS: [&str; 9] = ["idLo", "idHi", "x", "y", "z", "px", "py", "pz", "supported"];
const AIR_TRANSIT_FIELDS: [&str; 8] = [
    "kind", "x", "y", "z", "heading", "speedMps", "edgeTick", "dir",
];
const CORPSE_FIELDS: [&str; 9] = [
    "x",
    "y",
    "z",
    "own",
    "soldierLo",
    "soldierHi",
    "kind",
    "slot",
    "yaw",
];
const FALLEN_BODY_FIELDS: [&str; 7] = ["propLo", "propHi", "x", "y", "towardX", "towardY", "tick"];
const CRASH_FIELDS: [&str; 9] = ["id", "own", "kind", "x", "y", "z", "yaw", "pitch", "roll"];
const SHOUT_FIELDS: [&str; 9] = [
    "own",
    "idLo",
    "idHi",
    "kind",
    "x",
    "y",
    "z",
    "trigger",
    "intensity",
];
const PROP_DISPLAY_POSE_FIELDS: [&str; 5] = ["propId", "x", "y", "yaw", "baseZ"];
const GRIND_FIELDS: [&str; 5] = ["x", "y", "z", "headingX", "headingY"];
const REPORT_TOTAL_FIELDS: [&str; 4] = ["spawned", "spent", "killed", "refunded"];
const REPORT_EVENT_FIELDS: [&str; 15] = [
    "tickLo",
    "tickHi",
    "kind",
    "side",
    "unitIdLo",
    "unitIdHi",
    "unitKind",
    "owner",
    "otherSide",
    "otherIdLo",
    "otherIdHi",
    "otherKind",
    "otherOwner",
    "amount",
    "otherAmount",
];
const PLAYER_FIELDS: [&str; 7] = [
    "index",
    "id",
    "controller",
    "ready",
    "credits",
    "unitCount",
    "cap",
];
const KNOWN_PROP_FIELDS: [&str; 20] = [
    "kind",
    "x",
    "y",
    "yaw",
    "hx",
    "hy",
    "hz",
    "baseZ",
    "replacesLo",
    "replacesHi",
    "destroyed",
    "idLo",
    "idHi",
    "buildingLo",
    "buildingHi",
    "structureOwnerLo",
    "structureOwnerHi",
    "authoredPropLo",
    "authoredPropHi",
    "wreckOf",
];
const OWN_FIELDS: [&str; 56] = [
    "id",
    "owner",
    "kind",
    "x",
    "y",
    "z",
    "yaw",
    "goalX",
    "goalY",
    "policy",
    "direction",
    "reversing",
    "state",
    "blocker",
    "routeCount",
    "queueCount",
    "memberCount",
    "seesCount",
    "engagement",
    "mountCount",
    "hp",
    "suppression",
    "concealed",
    "deployProgress",
    "deployTarget",
    "hostKind",
    "hostLo",
    "hostHi",
    "holdPhase",
    "holdProgress",
    "hostX",
    "hostY",
    "hostHalfX",
    "hostHalfY",
    "stock",
    "service",
    "sightForward",
    "sightFront",
    "sightSide",
    "sightRear",
    "sightRange",
    "sightEyeCount",
    "finalFacing",
    "areaX",
    "areaY",
    "areaM",
    "withdrawing",
    "fuelSeconds",
    "afterburner",
    "inDogfight",
    "flareCharges",
    "flareCooldown",
    "protectionCharges",
    "protectionCooldownProgress",
    "smoking",
    "goalUnloads",
];
const IDENTIFIED_FIELDS: [&str; 13] = [
    "id",
    "kind",
    "cost",
    "x",
    "y",
    "z",
    "yaw",
    "vx",
    "vy",
    "memberCount",
    "poseCount",
    "reversing",
    "smoking",
];
const XY_FIELDS: [&str; 2] = ["x", "y"];
const XYZ_FIELDS: [&str; 3] = ["x", "y", "z"];
/// A queued order's destination, and 1 when it lets the passengers out there.
const QUEUE_FIELDS: [&str; 3] = ["x", "y", "unload"];
const MEMBER_ID_FIELDS: [&str; 4] = ["idLo", "idHi", "slot", "activeMount"];
const MEMBER_ORDER_FIELDS: [&str; 4] = ["x", "y", "coverNow", "coverThere"];
const RICOCHET_FIELDS: [&str; 4] = ["point", "nx", "ny", "nz"];
const DELIVERY_FIELDS: [&str; 3] = ["length", "encoding", "floats"];
const RANGE_FIELDS: [&str; 2] = ["start", "length"];
const COPY_FIELDS: [&str; 2] = ["source", "length"];

/// A variable-length run of points that follows a group's rows: its count is
/// the row's `count` field.
struct SectionLayout {
    name: &'static str,
    count: &'static str,
    fields: &'static [&'static str],
}

/// One non-map group: its header count, row fields and variable sections.
struct GroupLayout {
    name: &'static str,
    count: &'static str,
    fields: &'static [&'static str],
    sections: &'static [SectionLayout],
}

const fn section(
    name: &'static str,
    count: &'static str,
    fields: &'static [&'static str],
) -> SectionLayout {
    SectionLayout {
        name,
        count,
        fields,
    }
}

const fn rows(
    name: &'static str,
    count: &'static str,
    fields: &'static [&'static str],
) -> GroupLayout {
    GroupLayout {
        name,
        count,
        fields,
        sections: &[],
    }
}

/// The non-map groups in record order; [`layout_json`] publishes them and the
/// [`reader`] refuses a layout that differs.
const GROUP_LAYOUTS: [GroupLayout; GROUPS] = [
    GroupLayout {
        name: "own",
        count: "ownCount",
        fields: &OWN_FIELDS,
        sections: &[
            section("route", "routeCount", &XY_FIELDS),
            section("queue", "queueCount", &QUEUE_FIELDS),
            section("members", "memberCount", &XYZ_FIELDS),
            section("memberHp", "memberCount", &["hp"]),
            section("memberIds", "memberCount", &MEMBER_ID_FIELDS),
            section("memberOrders", "memberCount", &MEMBER_ORDER_FIELDS),
            section("memberLeans", "memberCount", &LEAN_FIELDS),
            section("sees", "seesCount", &["id"]),
            section("mounts", "mountCount", &MOUNT_FIELDS),
            section("weaponPoses", "mountCount", &POSE_FIELDS),
            section("sightEyes", "sightEyeCount", &XYZ_FIELDS),
        ],
    },
    GroupLayout {
        name: "identified",
        count: "identifiedCount",
        fields: &IDENTIFIED_FIELDS,
        sections: &[
            section("members", "memberCount", &XYZ_FIELDS),
            section("memberIds", "memberCount", &MEMBER_ID_FIELDS),
            section("memberLeans", "memberCount", &LEAN_FIELDS),
            section("weaponPoses", "poseCount", &POSE_FIELDS),
        ],
    },
    rows("contacts", "contactCount", &CONTACT_FIELDS),
    rows("audible", "audibleCount", &AUDIBLE_FIELDS),
    rows("refusals", "refusalCount", &REFUSAL_FIELDS),
    GroupLayout {
        name: "projectiles",
        count: "projectileCount",
        fields: &PROJECTILE_FIELDS,
        sections: &[
            section("path", "pointCount", &XYZ_FIELDS),
            section("ricochets", "ricochetCount", &RICOCHET_FIELDS),
        ],
    },
    rows("blasts", "blastCount", &BLAST_FIELDS),
    rows("guided", "guidedCount", &GUIDED_FIELDS),
    rows("airTransits", "airTransitCount", &AIR_TRANSIT_FIELDS),
    rows("corpses", "corpseCount", &CORPSE_FIELDS),
    rows("knownProps", "knownPropCount", &KNOWN_PROP_FIELDS),
    rows("fallenBodies", "fallenBodyCount", &FALLEN_BODY_FIELDS),
    rows("pendingPurchases", "pendingCount", &PENDING_FIELDS),
    rows("reportTotals", "reportTotalsCount", &REPORT_TOTAL_FIELDS),
    rows("reportEvents", "reportEventCount", &REPORT_EVENT_FIELDS),
    rows("objectives", "objectiveCount", &OBJECTIVE_FIELDS),
    rows("crashes", "crashCount", &CRASH_FIELDS),
    rows("shouts", "shoutCount", &SHOUT_FIELDS),
    rows("players", "playersCount", &PLAYER_FIELDS),
    rows(
        "propDisplayPoses",
        "propDisplayPoseCount",
        &PROP_DISPLAY_POSE_FIELDS,
    ),
    rows("grinds", "grindCount", &GRIND_FIELDS),
];

/// An integer as its two exact 16-bit limbs.
pub(crate) fn limbs(n: u32) -> [f32; 2] {
    [(n & 0xffff) as f32, (n >> LIMB_BITS) as f32]
}

fn limbs_or_absent(n: Option<u32>) -> [f32; 2] {
    n.map_or([-1.0; 2], limbs)
}

fn pose(p: &WeaponPose) -> [f32; 7] {
    let [lo, hi] = limbs(p.shots);
    let [operator_lo, operator_hi] = limbs_or_absent(p.operator);
    [
        p.mount as f32,
        p.bearing as f32,
        p.elevation as f32,
        lo,
        hi,
        operator_lo,
        operator_hi,
    ]
}

const LEAN_FIELDS: [&str; 3] = ["side", "x", "y"];

/// A member's lean, or none (side -1, NaN point).
fn lean(l: &Option<MemberLean>) -> [f32; 3] {
    l.map_or([-1.0, f32::NAN, f32::NAN], |l| {
        [tag(&LEAN_SIDES, &l.side), l.at[0] as f32, l.at[1] as f32]
    })
}

/// The `memberIds` rows: each soldier's exact id, slot and selected weapon.
fn member_ids<'a>(
    ids: &'a [u32],
    slots: &'a [u8],
    active_mounts: &'a [Option<u8>],
) -> impl Iterator<Item = f32> + 'a {
    ids.iter()
        .zip(slots)
        .zip(active_mounts)
        .flat_map(|((&id, &slot), &active)| {
            let [lo, hi] = limbs(id);
            [lo, hi, slot as f32, active.map_or(-1.0, |m| m as f32)]
        })
}

/// The transport's end of a side's publications: it packs group word changes,
/// visibility changes and the ground cells its consumer lacks. A stream (an epoch) opens with a full
/// snapshot when publishing starts, when the side changes and on
/// [`Publisher::resync`], then carries only cells learned or changed since
/// the previous record. The cursor is transport state, outside the digest.
#[derive(Default)]
pub struct Publisher {
    epoch: u32,
    /// The side and knowledge revision the consumer holds.
    cursor: Option<(Side, u32)>,
    ground_bytes: usize,
    out: Vec<f32>,
    fog: Vec<u32>,
    fog_revision: u32,
    fog_grid: Option<(f64, u32, u32)>,
    logical: Vec<f32>,
    encoded: Vec<f32>,
    groups: Vec<f32>,
    group_ends: Vec<usize>,
}

impl Publisher {
    pub fn new() -> Self {
        Self::default()
    }

    /// The next record opens a new epoch with a full snapshot (a consumer
    /// reconnected or reset its view).
    pub fn resync(&mut self) {
        self.cursor = None;
    }

    /// Pack `side`'s observation at the battle's tick with its ground patch.
    pub fn publish(&mut self, battle: &Battle, side: Side) -> Result<&[f32], String> {
        let ground_grid = battle.ground();
        if ground_grid
            .cols()
            .checked_mul(ground_grid.rows())
            .is_none_or(|cells| cells > u32::MAX as usize)
            || ground_grid
                .cols()
                .div_ceil(16)
                .checked_mul(ground_grid.rows().div_ceil(16))
                .is_none_or(|tiles| tiles > 1 << 24)
        {
            return Err("ground grid exceeds its exact delivery address range".into());
        }
        let field = &battle.observe(side).ground_visibility;
        let grid = (field.cell_m, field.nx, field.ny);
        let base = match self.cursor {
            Some((s, revision)) if s == side && self.fog_grid == Some(grid) => Some(revision),
            _ => None,
        };
        let known = battle.known_ground(side);
        let epoch = if base.is_none() {
            self.epoch
                .checked_add(1)
                .ok_or("publication epoch exhausted")?
        } else {
            self.epoch
        };
        let baseline = base.unwrap_or(0);
        // A current cursor needs no walk over retained tiles. A snapshot/delta
        // counts runs without collecting a second ground array.
        let count = if known.revision() == baseline {
            0
        } else {
            known
                .change_runs_since(baseline)
                .take(MAX_PUBLICATION_BYTES / 16 + 1)
                .count()
        };
        let ground = GroundHeader {
            epoch,
            side,
            base: baseline,
            revision: known.revision(),
            full: base.is_none(),
            count,
        };
        let frame = battle.observe(side);
        let changed: Vec<usize> = if base.is_some() {
            frame
                .ground_visibility
                .bits
                .iter()
                .zip(&self.fog)
                .enumerate()
                .filter_map(|(i, (now, before))| (now != before).then_some(i))
                .collect()
        } else {
            Vec::new()
        };
        let full = base.is_none() || changed.len() * 3 >= frame.ground_visibility.bits.len() * 2;
        let revision = self
            .fog_revision
            .checked_add(1)
            .ok_or("fog publication revision exhausted")?;
        let fog = FogPatch {
            full,
            base: if base.is_some() { self.fog_revision } else { 0 },
            revision,
            changed: &changed,
        };
        let mut ends = Vec::with_capacity(GROUPS);
        pack_record(
            frame,
            &ground,
            &fog,
            known.change_runs_since(baseline).take(count),
            &mut self.logical,
            Some(&mut ends),
        )?;
        let ground_bytes = encode_delivery(
            &self.logical,
            &ends,
            if base.is_some() {
                &self.groups[..]
            } else {
                &[]
            },
            if base.is_some() {
                &self.group_ends[..]
            } else {
                &[]
            },
            &mut self.encoded,
        )?;
        self.groups
            .try_reserve_exact(ends.last().unwrap().saturating_sub(self.groups.len()))
            .map_err(|e| format!("publication baseline allocation: {e}"))?;
        std::mem::swap(&mut self.out, &mut self.encoded);
        self.groups.clear();
        self.groups
            .extend_from_slice(&self.logical[..*ends.last().unwrap()]);
        self.group_ends = ends;
        if full {
            self.fog.clone_from(&frame.ground_visibility.bits);
        } else {
            for &i in &changed {
                self.fog[i] = frame.ground_visibility.bits[i];
            }
        }
        self.fog_revision = revision;
        self.fog_grid = Some(grid);
        self.epoch = epoch;
        self.cursor = Some((side, ground.revision));
        self.ground_bytes = ground_bytes;
        Ok(&self.out)
    }

    /// The latest packed record.
    pub fn record(&self) -> &[f32] {
        &self.out
    }

    /// Encoded ground bytes in the latest successful record, for reports.
    pub fn ground_patch_bytes(&self) -> usize {
        self.ground_bytes
    }
}

/// Cursor metadata for the same side/epoch as observation visibility.
pub struct GroundHeader {
    pub epoch: u32,
    pub side: Side,
    pub base: u32,
    pub revision: u32,
    pub full: bool,
    pub count: usize,
}

/// Visibility payload decision. Revisions are transport state, not battle state.
pub struct FogPatch<'a> {
    pub full: bool,
    pub base: u32,
    pub revision: u32,
    pub changed: &'a [usize],
}

/// Canonical complete logical record, before group and ground delivery encoding. This is
/// the reconstruction oracle; [`Publisher::publish`] owns the transport record.
/// Fog/ground payloads still follow their supplied cursor headers.
/// Admission precedes output allocation.
pub fn pack_logical(
    frame: &ObservationFrame,
    ground: &GroundHeader,
    patch: &FogPatch<'_>,
    runs: impl Iterator<Item = GroundRunPatch>,
    out: &mut Vec<f32>,
) -> Result<(), String> {
    pack_record(frame, ground, patch, runs, out, None)
}

fn pack_record(
    frame: &ObservationFrame,
    ground: &GroundHeader,
    patch: &FogPatch<'_>,
    runs: impl Iterator<Item = GroundRunPatch>,
    out: &mut Vec<f32>,
    mut ends: Option<&mut Vec<usize>>,
) -> Result<(), String> {
    let fog = &frame.ground_visibility;
    let cells = u64::from(fog.nx) * u64::from(fog.ny);
    let words = cells.div_ceil(32);
    if words > MAX_FOG_WORDS as u64 {
        return Err(format!(
            "fog delivery requires {words} words; admitted bound is {MAX_FOG_WORDS}"
        ));
    }
    let (cells, words) = (cells as usize, words as usize);
    if fog.bits.len() != words {
        return Err("fog field dimensions must match its words".into());
    }
    let fog_floats = if patch.full {
        words * 2
    } else {
        patch.changed.len() * 3
    };
    let [base_lo, base_hi] = limbs(patch.base);
    let [revision_lo, revision_hi] = limbs(patch.revision);
    // Plain ground counters remain exact inside their existing lifetime bound.
    if ground.epoch >= 1 << 24 || ground.revision >= 1 << 24 || ground.base >= 1 << 24 {
        return Err("ground cursor exceeds its exact float range".into());
    }
    let length = packed_len(frame, fog_floats, ground.count)?;
    out.try_reserve_exact(length.saturating_sub(out.len()))
        .map_err(|e| format!("publication allocation: {e}"))?;
    out.clear();
    out.extend([
        frame.tick as f32,
        frame.own.len() as f32,
        frame.identified.len() as f32,
        frame.contacts.len() as f32,
        frame.audible.len() as f32,
        frame.refusals.len() as f32,
        frame.known_props.len() as f32,
        frame.projectiles.len() as f32,
        frame.blasts.len() as f32,
        frame.corpses.len() as f32,
        frame.guided.len() as f32,
        frame.air_transits.len() as f32,
        frame.encounter.map_or(-1.0, |e| e.held_s as f32),
        frame
            .encounter
            .map_or(-1.0, |e| tag(&ENCOUNTER_RESULTS, &e.result)),
        fog.cell_m as f32,
        fog.nx as f32,
        fog.ny as f32,
        fog_floats as f32,
        patch.full as u8 as f32,
        base_lo,
        base_hi,
        revision_lo,
        revision_hi,
        ground.epoch as f32,
        tag(&Side::ALL, &ground.side),
        ground.base as f32,
        ground.revision as f32,
        ground.full as u8 as f32,
        ground.count as f32,
        frame.fallen_bodies.len() as f32,
        frame
            .skirmish
            .as_ref()
            .map_or(-1.0, |s| s.phase as u8 as f32),
        frame
            .skirmish
            .as_ref()
            .map_or(0.0, |s| s.players.len() as f32),
        frame
            .skirmish
            .as_ref()
            .map_or(0.0, |s| s.preparation_remaining_s as f32),
        frame.skirmish.as_ref().map_or(0.0, |s| {
            s.players.iter().map(|p| p.pending.len()).sum::<usize>() as f32
        }),
        frame.skirmish.as_ref().map_or(0.0, |s| {
            s.report.as_ref().map_or(0, |r| r.totals.len()) as f32
        }),
        frame.skirmish.as_ref().map_or(0.0, |s| {
            s.report.as_ref().map_or(0, |r| r.events.len()) as f32
        }),
        frame
            .skirmish
            .as_ref()
            .map_or(0.0, |s| s.objectives.len() as f32),
        frame.skirmish.as_ref().map_or(0.0, |s| s.scores[0] as f32),
        frame.skirmish.as_ref().map_or(0.0, |s| s.scores[1] as f32),
        frame
            .skirmish
            .as_ref()
            .and_then(|s| s.result)
            .map_or(-1.0, |r| match r {
                contract::skirmish::MatchResult::Winner { side } => side.index() as f32,
                contract::skirmish::MatchResult::Draw => 2.0,
            }),
        frame.crashes.len() as f32,
        frame.shouts.len() as f32,
        frame.prop_display_poses.len() as f32,
        frame.grinds.len() as f32,
    ]);
    for u in &frame.own {
        let (host_kind, host_id, host_center, host_half) = match u.hold.map(|h| h.host) {
            Some(HostRef::Building { id, center, half }) => {
                (tag(&HOST_KINDS, &"building"), Some(id), center, half)
            }
            Some(HostRef::Carrier { id }) => (
                tag(&HOST_KINDS, &"carrier"),
                Some(id),
                [f64::NAN; 2],
                [f64::NAN; 2],
            ),
            None => (-1.0, None, [f64::NAN; 2], [f64::NAN; 2]),
        };
        let [host_lo, host_hi] = limbs_or_absent(host_id);
        let [gx, gy] = u.goal.map_or([f32::NAN; 2], |g| [g[0] as f32, g[1] as f32]);
        out.extend([
            u.id.0 as f32,
            u.owner.map_or(-1.0, |p| f32::from(p.0)),
            u.kind.0 as f32,
            u.position[0] as f32,
            u.position[1] as f32,
            u.position[2] as f32,
            u.yaw as f32,
            gx,
            gy,
            u.policy.map_or(-1.0, |p| tag(&POLICIES, &p)),
            u.direction.map_or(-1.0, |d| tag(&DIRECTIONS, &d)),
            u.reversing as u8 as f32,
            tag(&MOVE_STATES, &u.state),
            u.blocker.map_or(-1.0, |b| b.0 as f32),
            u.route.len() as f32,
            u.queue.len() as f32,
            u.members.len() as f32,
            u.sees.len() as f32,
            tag(&ENGAGEMENTS, &u.engagement),
            u.mounts.len() as f32,
            u.hp as f32,
            tag(&SUPPRESSION_TIERS, &u.suppression),
            u.concealed as u8 as f32,
            u.deployment.map_or(-1.0, |d| d.progress as f32),
            u.deployment.map_or(-1.0, |d| tag(&POSTURES, &d.target)),
            host_kind,
            host_lo,
            host_hi,
            u.hold.map_or(-1.0, |h| tag(&HOLD_PHASES, &h.phase)),
            u.hold.map_or(-1.0, |h| h.progress as f32),
            host_center[0] as f32,
            host_center[1] as f32,
            host_half[0] as f32,
            host_half[1] as f32,
            u.stock.map_or(-1.0, |n| n as f32),
            tag(&SERVICE_STATUSES, &u.service),
            u.sight.forward as f32,
            u.sight.shape.front as f32,
            u.sight.shape.side as f32,
            u.sight.shape.rear as f32,
            u.sight.range as f32,
            u.sight.eyes.len() as f32,
            u.final_facing as f32,
            u.area.map_or(f32::NAN, |a| a.anchor[0] as f32),
            u.area.map_or(f32::NAN, |a| a.anchor[1] as f32),
            u.area.map_or(f32::NAN, |a| a.radius as f32),
            u.withdrawing as u8 as f32,
            u.fuel_s.map_or(-1.0, |fuel| fuel as f32),
            u.afterburner as u8 as f32,
            u.in_dogfight as u8 as f32,
            u.flares.map_or(-1.0, |f| f.charges as f32),
            u.flares.and_then(|f| f.cooldown).map_or(-1.0, |p| p as f32),
            u.protection.map_or(-1.0, |p| p.charges as f32),
            u.protection
                .and_then(|p| p.cooldown)
                .map_or(-1.0, |p| p as f32),
            u.smoking as u8 as f32,
            u.goal_unloads as u8 as f32,
        ]);
    }
    for u in &frame.own {
        out.extend(u.route.iter().flat_map(|p| [p[0] as f32, p[1] as f32]));
        out.extend(
            u.queue
                .iter()
                .zip(&u.queue_unloads)
                .flat_map(|(p, &unloads)| [p[0] as f32, p[1] as f32, unloads as u8 as f32]),
        );
        out.extend(
            u.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(u.member_hp.iter().map(|&hp| hp as f32));
        out.extend(member_ids(
            &u.member_ids,
            &u.member_slots,
            &u.member_active_mounts,
        ));
        let tier = |t: Option<CoverTier>| t.map_or(-1.0, |t| tag(&COVER_TIERS, &t));
        out.extend(u.member_orders.iter().flat_map(|m| {
            [
                m.spot[0] as f32,
                m.spot[1] as f32,
                tier(m.cover_now),
                tier(m.cover_there),
            ]
        }));
        out.extend(u.member_leans.iter().flat_map(lean));
        out.extend(u.sees.iter().map(|id| id.0 as f32));
        for m in &u.mounts {
            let ammo = |k: usize| m.ammo.get(k).map_or(-2.0, |a| a.map_or(-1.0, |n| n as f32));
            let (kind, id, p) = match m.target {
                None => (0.0, -1.0, [0.0; 3]),
                Some(TargetRef::Identified { id }) => (1.0, id.0 as f32, [0.0; 3]),
                Some(TargetRef::Contact { id }) => (2.0, id.0 as f32, [0.0; 3]),
                Some(TargetRef::Ground { point }) => (3.0, -1.0, point.map(|v| v as f32)),
            };
            out.extend([
                m.mount as f32,
                m.loaded.map_or(-1.0, |k| k as f32),
                ammo(0),
                ammo(1),
                m.aim as f32,
                m.reload as f32,
                kind,
                id,
                p[0],
                p[1],
                p[2],
                tag(&REASONS, &m.reason),
                m.ammo.len() as f32,
                m.guiding as u8 as f32,
                m.reloading.map_or(-1.0, |k| k as f32),
            ]);
        }
        // One pose per mount: the section is counted by mountCount.
        assert_eq!(u.weapon_poses.len(), u.mounts.len());
        out.extend(u.weapon_poses.iter().flat_map(pose));
        out.extend(
            u.sight
                .eyes
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for e in &frame.identified {
        out.extend([
            e.id.0 as f32,
            e.kind.0 as f32,
            e.cost as f32,
            e.position[0] as f32,
            e.position[1] as f32,
            e.position[2] as f32,
            e.yaw as f32,
            e.velocity[0] as f32,
            e.velocity[1] as f32,
            e.members.len() as f32,
            e.weapon_poses.len() as f32,
            e.reversing as u8 as f32,
            e.smoking as u8 as f32,
        ]);
    }
    for e in &frame.identified {
        out.extend(
            e.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(member_ids(
            &e.member_ids,
            &e.member_slots,
            &e.member_active_mounts,
        ));
        out.extend(e.member_leans.iter().flat_map(lean));
        out.extend(e.weapon_poses.iter().flat_map(pose));
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.contacts {
        out.extend([
            c.id.0 as f32,
            tag(&CONTACT_SOURCES, &c.source),
            c.center[0] as f32,
            c.center[1] as f32,
            c.center[2] as f32,
            tag(&LAYERS, &c.layer),
            if c.aloft { 1.0 } else { 0.0 },
            c.radius as f32,
            c.evidence_tick as f32,
            c.expires_tick as f32,
            c.kind.map_or(-1.0, |k| k.0 as f32),
        ]);
        out.extend(
            (0..HEARD_WORDS).map(|w| {
                ((c.heard >> (w * HEARD_WORD_BITS)) & ((1 << HEARD_WORD_BITS) - 1)) as f32
            }),
        );
        out.extend([
            c.bearing_deg.unwrap_or(-1.0) as f32,
            c.range_m.unwrap_or(-1.0) as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for a in &frame.audible {
        out.extend([
            a.listener.0 as f32,
            tag(&SOUND_CATEGORIES, &a.category),
            a.sector as f32,
            tag(&SOUND_BANDS, &a.band),
            a.moving as u8 as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for refusal in &frame.refusals {
        let [lo, hi] = limbs(refusal.unit.0);
        out.extend([
            lo,
            hi,
            ORDER_ERROR_NAMES
                .iter()
                .position(|name| *name == refusal.reason.reason_name())
                .expect("every order error has a publication name") as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.projectiles {
        let [lo, hi] = limbs_or_absent(p.shooter_member);
        let n = p.impact_normal.unwrap_or([0.0; 3]);
        out.extend([
            p.path.len() as f32,
            p.ricochets.len() as f32,
            p.own as u8 as f32,
            p.kind as f32,
            lo,
            hi,
            tag(&SEGMENT_HITS, &p.hit),
            n[0] as f32,
            n[1] as f32,
            n[2] as f32,
        ]);
    }
    for p in &frame.projectiles {
        out.extend(
            p.path
                .iter()
                .flat_map(|q| [q[0] as f32, q[1] as f32, q[2] as f32]),
        );
        out.extend(p.ricochets.iter().flat_map(|r| {
            [
                r.point as f32,
                r.normal[0] as f32,
                r.normal[1] as f32,
                r.normal[2] as f32,
            ]
        }));
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for b in &frame.blasts {
        out.extend([
            b.point[0] as f32,
            b.point[1] as f32,
            b.point[2] as f32,
            b.radius as f32,
            b.kind as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for g in &frame.guided {
        let [lo, hi] = limbs(u32::try_from(g.id).expect("a guided id fits two limbs"));
        out.extend([
            lo,
            hi,
            g.position[0] as f32,
            g.position[1] as f32,
            g.position[2] as f32,
            g.point[0] as f32,
            g.point[1] as f32,
            g.point[2] as f32,
            g.supported as u8 as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for t in &frame.air_transits {
        out.extend([
            t.kind.0 as f32,
            t.point[0] as f32,
            t.point[1] as f32,
            t.point[2] as f32,
            t.heading as f32,
            t.speed_mps as f32,
            t.edge_tick as f32,
            tag(&AIR_TRANSIT_DIRECTIONS, &t.dir),
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.corpses {
        let [lo, hi] = limbs(c.soldier);
        out.extend([
            c.position[0] as f32,
            c.position[1] as f32,
            c.position[2] as f32,
            c.own as u8 as f32,
            lo,
            hi,
            c.kind.0 as f32,
            c.slot as f32,
            c.yaw as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.known_props {
        let [replaces_lo, replaces_hi] = limbs_or_absent(p.replaces);
        let [id_lo, id_hi] = limbs(p.id);
        let [building_lo, building_hi] = limbs_or_absent(p.building);
        let [owner_lo, owner_hi] = limbs_or_absent(p.structure_owner);
        let [authored_lo, authored_hi] = limbs_or_absent(p.authored_prop);
        out.extend([
            p.kind.0 as f32,
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.half_extents[0] as f32,
            p.half_extents[1] as f32,
            p.half_extents[2] as f32,
            p.base_z as f32,
            replaces_lo,
            replaces_hi,
            p.destroyed as u8 as f32,
            id_lo,
            id_hi,
            building_lo,
            building_hi,
            owner_lo,
            owner_hi,
            authored_lo,
            authored_hi,
            p.wreck_of.map_or(-1.0, |t| t.0 as f32),
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for f in &frame.fallen_bodies {
        let [lo, hi] = limbs(f.prop);
        out.extend([
            lo,
            hi,
            f.at[0] as f32,
            f.at[1] as f32,
            f.toward[0] as f32,
            f.toward[1] as f32,
            f.tick as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(skirmish) = &frame.skirmish {
        for player in &skirmish.players {
            for p in &player.pending {
                let [lo, hi] = limbs(p.id.0);
                let [tick_lo, tick_hi] = limbs(
                    p.confirmed_tick
                        .try_into()
                        .map_err(|_| "purchase tick exceeds publication range")?,
                );
                // An aircraft's arrival tick, 0 for a purchase that doesn't wait.
                let [arrival_lo, arrival_hi] = limbs(
                    p.arrival_tick
                        .unwrap_or(0)
                        .try_into()
                        .map_err(|_| "arrival tick exceeds publication range")?,
                );
                out.extend([
                    lo,
                    hi,
                    f32::from(player.index.0),
                    p.kind.0 as f32,
                    p.destination[0] as f32,
                    p.destination[1] as f32,
                    tick_lo,
                    tick_hi,
                    p.blocked as u8 as f32,
                    arrival_lo,
                    arrival_hi,
                ]);
            }
        }
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(report) = frame.skirmish.as_ref().and_then(|s| s.report.as_ref()) {
        for totals in &report.totals {
            out.extend([
                totals.spawned as f32,
                totals.spent as f32,
                totals.killed as f32,
                totals.refunded as f32,
            ]);
        }
        if let Some(ends) = ends.as_mut() {
            ends.push(out.len());
        }
        for event in &report.events {
            let (tick, kind, side, unit, other, amount, other_amount) = match event {
                contract::skirmish::MatchEvent::Spawn { tick, unit, cost } => {
                    (*tick, 0.0, unit.side, *unit, None, *cost as f32, 0.0)
                }
                contract::skirmish::MatchEvent::Refund {
                    tick,
                    unit,
                    credits,
                } => (*tick, 1.0, unit.side, *unit, None, *credits as f32, 0.0),
                contract::skirmish::MatchEvent::Kill {
                    tick,
                    victim,
                    victim_cost,
                    killer,
                    killer_cost,
                } => (
                    *tick,
                    2.0,
                    victim.side,
                    *victim,
                    *killer,
                    *victim_cost as f32,
                    killer_cost.unwrap_or_default() as f32,
                ),
            };
            let [tick_lo, tick_hi] = limbs(
                tick.try_into()
                    .map_err(|_| "report tick exceeds publication range")?,
            );
            let [id_lo, id_hi] = limbs(unit.id.0);
            let owner = unit.owner.map_or(-1.0, |p| f32::from(p.0));
            let (other_side, other_id_lo, other_id_hi, other_kind, other_owner) =
                other.map_or((-1.0, -1.0, -1.0, -1.0, -1.0), |u| {
                    let [lo, hi] = limbs(u.id.0);
                    (
                        u.side.index() as f32,
                        lo,
                        hi,
                        u.kind.0 as f32,
                        u.owner.map_or(-1.0, |p| f32::from(p.0)),
                    )
                });
            out.extend([
                tick_lo,
                tick_hi,
                kind,
                side.index() as f32,
                id_lo,
                id_hi,
                unit.kind.0 as f32,
                owner,
                other_side,
                other_id_lo,
                other_id_hi,
                other_kind,
                other_owner,
                amount,
                other_amount,
            ]);
        }
    } else if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(skirmish) = &frame.skirmish {
        for (i, o) in skirmish.objectives.iter().enumerate() {
            out.extend([
                i as f32,
                o.center[0] as f32,
                o.center[1] as f32,
                o.radius_m as f32,
                o.owner.map_or(-1.0, |s| s.index() as f32),
                o.capturing.map_or(-1.0, |s| s.index() as f32),
                o.capture_progress as f32,
                o.contested as u8 as f32,
            ]);
        }
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.crashes {
        out.extend([
            c.id as f32,
            c.own as u8 as f32,
            c.kind.0 as f32,
            c.position[0] as f32,
            c.position[1] as f32,
            c.position[2] as f32,
            c.yaw as f32,
            c.pitch as f32,
            c.roll as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for s in &frame.shouts {
        let (own, id) = match s.speaker {
            ShoutSpeaker::Own(id) => (1.0, id.0),
            ShoutSpeaker::Seen(id) => (0.0, id.0),
        };
        let [lo, hi] = limbs(id);
        out.extend([
            own,
            lo,
            hi,
            s.kind.0 as f32,
            s.position[0] as f32,
            s.position[1] as f32,
            s.position[2] as f32,
            tag(&ShoutTrigger::ALL, &s.trigger),
            tag(&INTENSITIES, &s.intensity),
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(skirmish) = &frame.skirmish {
        for player in &skirmish.players {
            out.extend([
                f32::from(player.index.0),
                // The browser resolves this index through the layout's playerIds table.
                player.index.0 as f32,
                tag(
                    &[
                        ControllerKind::Human,
                        ControllerKind::ActiveAi,
                        ControllerKind::PassiveAi,
                    ],
                    &player.controller,
                ),
                player.ready as u8 as f32,
                player.credits as f32,
                player.unit_count as f32,
                player.cap as f32,
            ]);
        }
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.prop_display_poses {
        out.extend([
            p.prop_id as f32,
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.base_z as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for g in &frame.grinds {
        out.extend([
            g.point[0] as f32,
            g.point[1] as f32,
            g.point[2] as f32,
            g.heading[0] as f32,
            g.heading[1] as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    let word = |i: usize| {
        let value = fog.bits[i];
        if i + 1 == words && !cells.is_multiple_of(32) {
            value & ((1u32 << (cells % 32)) - 1)
        } else {
            value
        }
    };
    if patch.full {
        for i in 0..words {
            out.extend(limbs(word(i)));
        }
    } else {
        for &i in patch.changed {
            // MAX_FOG_WORDS keeps the word index exact in a single float.
            out.push(i as f32);
            out.extend(limbs(word(i)));
        }
    }
    let mut encoded = 0;
    for run in runs {
        if encoded >= ground.count
            || run.tile >= 1 << 24
            || run.len == 0
            || usize::from(run.start) + usize::from(run.len) > 256
        {
            out.clear();
            return Err("ground run exceeds its exact tile-local encoding".into());
        }
        let c = run.marks;
        out.extend([
            run.tile as f32,
            u32::from(run.start) as f32 + u32::from(run.len) as f32 * 256.0,
            (u32::from(c.crater) + (u32::from(c.scorch) << 8)) as f32,
            (u32::from(c.tracks) + (u32::from(c.trampled) << 8) + (u32::from(c.cleared) << 16))
                as f32,
        ]);
        encoded += 1;
    }
    if encoded != ground.count || out.len() != length {
        out.clear();
        return Err("publication values do not match their counts".into());
    }
    Ok(())
}

fn packed_len(frame: &ObservationFrame, fog: usize, runs: usize) -> Result<usize, String> {
    let mut length = HEADER.len();
    let mut add = |rows: usize, width: usize| -> Result<(), String> {
        length = rows
            .checked_mul(width)
            .and_then(|n| length.checked_add(n))
            .filter(|n| *n <= MAX_PUBLICATION_BYTES / 4)
            .ok_or("publication exceeds its 64 MiB atomic allocation allowance")?;
        Ok(())
    };
    add(
        frame.skirmish.as_ref().map_or(0, |s| {
            s.players.iter().map(|p| p.pending.len()).sum::<usize>()
        }),
        PENDING_FIELDS.len(),
    )?;
    add(
        frame.skirmish.as_ref().map_or(0, |s| s.players.len()),
        PLAYER_FIELDS.len(),
    )?;
    add(
        frame
            .skirmish
            .as_ref()
            .and_then(|s| s.report.as_ref())
            .map_or(0, |r| r.totals.len()),
        REPORT_TOTAL_FIELDS.len(),
    )?;
    add(
        frame
            .skirmish
            .as_ref()
            .and_then(|s| s.report.as_ref())
            .map_or(0, |r| r.events.len()),
        REPORT_EVENT_FIELDS.len(),
    )?;
    add(
        frame.skirmish.as_ref().map_or(0, |s| s.objectives.len()),
        OBJECTIVE_FIELDS.len(),
    )?;
    add(frame.own.len(), OWN_FIELDS.len())?;
    for u in &frame.own {
        add(u.route.len(), 2)?;
        add(u.queue.len(), QUEUE_FIELDS.len())?;
        add(u.members.len(), 3)?;
        add(u.member_hp.len(), 1)?;
        add(
            u.member_ids
                .len()
                .min(u.member_slots.len())
                .min(u.member_active_mounts.len()),
            4,
        )?;
        add(u.member_orders.len(), 4)?;
        add(u.member_leans.len(), LEAN_FIELDS.len())?;
        add(u.sees.len(), 1)?;
        add(u.mounts.len(), MOUNT_FIELDS.len())?;
        add(u.weapon_poses.len(), POSE_FIELDS.len())?;
        add(u.sight.eyes.len(), 3)?;
    }
    add(frame.identified.len(), IDENTIFIED_FIELDS.len())?;
    for u in &frame.identified {
        add(u.members.len(), 3)?;
        add(
            u.member_ids
                .len()
                .min(u.member_slots.len())
                .min(u.member_active_mounts.len()),
            4,
        )?;
        add(u.member_leans.len(), LEAN_FIELDS.len())?;
        add(u.weapon_poses.len(), POSE_FIELDS.len())?;
    }
    add(frame.contacts.len(), CONTACT_FIELDS.len())?;
    add(frame.audible.len(), AUDIBLE_FIELDS.len())?;
    add(frame.refusals.len(), REFUSAL_FIELDS.len())?;
    add(frame.projectiles.len(), PROJECTILE_FIELDS.len())?;
    for p in &frame.projectiles {
        add(p.path.len(), 3)?;
        add(p.ricochets.len(), 4)?;
    }
    add(frame.blasts.len(), BLAST_FIELDS.len())?;
    add(frame.guided.len(), GUIDED_FIELDS.len())?;
    add(frame.air_transits.len(), AIR_TRANSIT_FIELDS.len())?;
    add(frame.corpses.len(), CORPSE_FIELDS.len())?;
    add(frame.known_props.len(), KNOWN_PROP_FIELDS.len())?;
    add(frame.fallen_bodies.len(), FALLEN_BODY_FIELDS.len())?;
    add(frame.crashes.len(), CRASH_FIELDS.len())?;
    add(frame.shouts.len(), SHOUT_FIELDS.len())?;
    add(
        frame.prop_display_poses.len(),
        PROP_DISPLAY_POSE_FIELDS.len(),
    )?;
    add(frame.grinds.len(), GRIND_FIELDS.len())?;
    add(fog, 1)?;
    add(runs, GROUND_FIELDS.len())?;
    Ok(length)
}

/// Canonical-to-wire serialization. Each non-map group keeps its own addresses:
/// changing a squad's variable rows never
/// shifts an unchanged corpse or known body. Values are copied as f32 words;
/// equality is bitwise, including NaN payloads and signed zero.
fn encode_delivery(
    logical: &[f32],
    ends: &[usize],
    previous: &[f32],
    previous_ends: &[usize],
    out: &mut Vec<f32>,
) -> Result<usize, String> {
    let mut plans = Vec::with_capacity(ends.len());
    let mut start = HEADER.len();
    let tail = ends.last().copied().unwrap_or(HEADER.len());
    let fog_floats = logical[HEADER.iter().position(|name| *name == "fogFloats").unwrap()] as usize;
    let ground_start = tail + fog_floats;
    let mut ground_count = PackedCount(0);
    pack_ground(&logical[ground_start..], &mut ground_count);
    let ground_words = ground_count.0.div_ceil(32);
    let mut length = HEADER.len() + fog_floats + ground_words;
    for (g, &end) in ends.iter().enumerate() {
        let values = &logical[start..end];
        let old = previous_ends.get(g).map_or(&[][..], |&end| {
            let start = if g == 0 {
                HEADER.len()
            } else {
                previous_ends[g - 1]
            };
            &previous[start..end]
        });
        let stride = fixed_row_width(g);
        let (delta, delta_packed, delta_operations) = if previous.is_empty() {
            (values.len(), 0, 0)
        } else {
            measure_group(0, values, old, stride, &[])
        };
        let mut encoding = u8::from(previous.is_empty() || delta >= values.len());
        let (mut payload, mut packed_words, mut operations) = if encoding == 1 {
            measure_group(1, values, old, stride, &[])
        } else {
            (delta, delta_packed, delta_operations)
        };
        let mut index = Vec::new();
        if !previous.is_empty()
            && payload > 2
            && !old.is_empty()
            && (stride == 0
                || (old.len().is_multiple_of(stride) && values.len().is_multiple_of(stride)))
        {
            // Fixed rows retain row alignment; variable sections use sparse exact
            // short word anchors. Both share one source-span wire grammar.
            let anchor = if stride == 0 {
                VARIABLE_ANCHOR_WORDS
            } else {
                stride
            };
            index
                .try_reserve_exact(old.len() / anchor)
                .map_err(|e| format!("publication row index allocation: {e}"))?;
            index.extend(
                (0..old.len().saturating_sub(anchor - 1))
                    .step_by(anchor)
                    .map(|i| i as u32),
            );
            index.sort_unstable_by(|&a, &b| {
                compare_rows(
                    &old[a as usize..a as usize + anchor],
                    &old[b as usize..b as usize + anchor],
                )
                .then_with(|| a.cmp(&b))
            });
            let (copies, copy_packed, copy_operations) =
                measure_group(2, values, old, stride, &index);
            if copies < payload {
                encoding = 2;
                payload = copies;
                packed_words = copy_packed;
                operations = copy_operations;
            } else {
                index = Vec::new();
            }
        }
        let packed = packed_words < payload;
        if packed {
            payload = packed_words;
        }
        length = length
            .checked_add(3 + payload)
            .filter(|n| *n <= MAX_PUBLICATION_BYTES / 4)
            .ok_or("publication exceeds its 64 MiB atomic allocation allowance")?;
        plans.push((encoding, payload, old, stride, index, packed, operations));
        start = end;
    }
    // Admission and exact reservation precede writing. Geometric growth must
    // not turn the record allowance into larger retained scratch allocations.
    out.try_reserve_exact(length.saturating_sub(out.len()))
        .map_err(|e| format!("publication encoding allocation: {e}"))?;
    out.clear();
    out.extend_from_slice(&logical[..HEADER.len()]);
    start = HEADER.len();
    for (&end, (encoding, payload, old, stride, index, packed, operations)) in
        ends.iter().zip(plans)
    {
        let values = &logical[start..end];
        out.extend([
            values.len() as f32,
            if packed { 3.0 } else { encoding as f32 },
            payload as f32,
        ]);
        if packed {
            let mut writer = PackedWriter {
                out,
                pending: 0,
                bits: 0,
            };
            pack_group(
                encoding,
                values,
                old,
                stride,
                &index,
                operations,
                &mut writer,
            );
            writer.finish();
        } else if encoding == 1 {
            out.extend_from_slice(values);
        } else if encoding == 2 {
            for (source, from, to) in source_copies(values, old, stride, &index) {
                out.extend([source.map_or(-1.0, |s| s as f32), (to - from) as f32]);
                if source.is_none() {
                    out.extend_from_slice(&values[from..to]);
                }
            }
        } else {
            for (from, to) in changed_ranges(values, old) {
                out.extend([from as f32, (to - from) as f32]);
                out.extend_from_slice(&values[from..to]);
            }
        }
        start = end;
    }
    out.extend_from_slice(&logical[start..ground_start]);
    let mut writer = PackedWriter {
        out,
        pending: 0,
        bits: 0,
    };
    pack_ground(&logical[ground_start..], &mut writer);
    writer.finish();
    Ok(ground_words * 4)
}

/// Ground keeps its canonical four-word logical rows. Only their final wire
/// tail changes: ordered tile deltas and exact byte marks share the carrier
/// writer with non-map groups, without a baseline or another staging buffer.
fn pack_ground(values: &[f32], sink: &mut impl PackedSink) {
    let mut prior = 0;
    for row in values.as_chunks::<{ GROUND_FIELDS.len() }>().0 {
        let tile = row[0] as u32;
        let span = row[1] as u32;
        let a = row[2] as u32;
        let b = row[3] as u32;
        sink.integer(tile - prior);
        sink.put(span % 256, 8);
        sink.put(span / 256 - 1, 8);
        let marks = [a & 255, a >> 8, b & 255, (b >> 8) & 255, b >> 16];
        let mask = marks
            .iter()
            .enumerate()
            .fold(0, |mask, (i, &value)| mask | (u32::from(value != 0) << i));
        sink.put(mask, 5);
        for value in marks {
            if value != 0 {
                sink.put(value, 8);
            }
        }
        prior = tile;
    }
}

/// The compact carrier changes only serialization of an already selected word
/// form. No second span selection, byte buffer, or decoded state is retained.
trait PackedSink {
    fn put(&mut self, value: u32, bits: usize);
    fn integer(&mut self, mut value: u32) {
        loop {
            let byte = value & 127;
            value >>= 7;
            self.put(byte | if value == 0 { 0 } else { 128 }, 8);
            if value == 0 {
                break;
            }
        }
    }
    fn literal(&mut self, value: f32, old: f32, predict: bool) {
        let raw = value.to_bits();
        let xor = raw ^ old.to_bits();
        let raw_bytes = (32 - raw.leading_zeros() as usize).div_ceil(8);
        let xor_bytes = (32 - xor.leading_zeros() as usize).div_ceil(8);
        let residual = predict && xor_bytes < raw_bytes;
        let bytes = if residual { xor_bytes } else { raw_bytes };
        self.put((bytes + if residual { 5 } else { 0 }) as u32, 4);
        if bytes != 0 {
            self.put(if residual { xor } else { raw }, bytes * 8);
        }
    }
}
struct PackedCount(usize);
impl PackedSink for PackedCount {
    fn put(&mut self, _: u32, bits: usize) {
        self.0 += bits;
    }
}
struct PackedWriter<'a> {
    out: &'a mut Vec<f32>,
    pending: u64,
    bits: usize,
}
impl PackedSink for PackedWriter<'_> {
    fn put(&mut self, value: u32, bits: usize) {
        self.pending |= (value as u64) << self.bits;
        self.bits += bits;
        if self.bits >= 32 {
            self.out.push(f32::from_bits(self.pending as u32));
            self.pending >>= 32;
            self.bits -= 32;
        }
    }
}
impl PackedWriter<'_> {
    fn finish(self) {
        if self.bits != 0 {
            self.out.push(f32::from_bits(self.pending as u32));
        }
    }
}
fn measure_group(
    encoding: u8,
    values: &[f32],
    old: &[f32],
    stride: usize,
    index: &[u32],
) -> (usize, usize, u32) {
    let mut count = PackedCount(8);
    let (words, operations) =
        write_group_operations(encoding, values, old, stride, index, &mut count);
    if encoding == 0 {
        count.integer(operations);
    }
    (words, count.0.div_ceil(32), operations)
}
fn pack_group(
    encoding: u8,
    values: &[f32],
    old: &[f32],
    stride: usize,
    index: &[u32],
    operations: u32,
    sink: &mut impl PackedSink,
) {
    sink.put(encoding as u32, 8);
    if encoding == 0 {
        sink.integer(operations);
    }
    write_group_operations(encoding, values, old, stride, index, sink);
}
/// Measure the ordinary and compact representations together. In particular,
/// selecting a compact copy never adds another sparse-index search pass.
fn write_group_operations(
    encoding: u8,
    values: &[f32],
    old: &[f32],
    stride: usize,
    index: &[u32],
    sink: &mut impl PackedSink,
) -> (usize, u32) {
    let mut words = 0;
    let mut operations = 0;
    if encoding == 1 {
        for &value in values {
            sink.literal(value, 0.0, false);
        }
        words = values.len();
    } else if encoding == 0 {
        for (from, to) in changed_ranges(values, old) {
            operations += 1;
            words += 2 + to - from;
            sink.integer(from as u32);
            sink.integer((to - from) as u32);
            for (at, &value) in values.iter().enumerate().take(to).skip(from) {
                sink.literal(value, old.get(at).copied().unwrap_or(0.0), true);
            }
        }
    } else {
        for (source, from, to) in source_copies(values, old, stride, index) {
            operations += 1;
            words += 2;
            sink.integer(source.map_or(0, |s| s as u32 + 1));
            sink.integer((to - from) as u32);
            if source.is_none() {
                words += to - from;
                for (at, &value) in values.iter().enumerate().take(to).skip(from) {
                    sink.literal(value, old.get(at).copied().unwrap_or(0.0), true);
                }
            }
        }
    }
    (words, operations)
}

/// Short exact spans remain reusable when variable metadata shifts their addresses.
const VARIABLE_ANCHOR_WORDS: usize = 3;

/// Non-map groups, in record order.
pub const GROUPS: usize = 21;

/// A non-map group's fixed row width; 0 where variable sections leave no one
/// width to address complete rows by.
fn fixed_row_width(group: usize) -> usize {
    let g = &GROUP_LAYOUTS[group];
    if g.sections.is_empty() {
        g.fields.len()
    } else {
        0
    }
}

fn compare_rows(a: &[f32], b: &[f32]) -> std::cmp::Ordering {
    a.iter()
        .map(|v| v.to_bits())
        .cmp(b.iter().map(|v| v.to_bits()))
}

/// Assemble canonical order from exact old spans and new literals. Fixed rows
/// retain their alignment; variable spans use sparse short word anchors and
/// extend wordwise. Bit comparisons never depend on entity identity or hashes.
fn source_copies<'a>(
    values: &'a [f32],
    old: &'a [f32],
    stride: usize,
    index: &'a [u32],
) -> impl Iterator<Item = (Option<usize>, usize, usize)> + 'a {
    let anchor = if stride == 0 {
        VARIABLE_ANCHOR_WORDS
    } else {
        stride
    };
    let source = move |at: usize| {
        let row = values.get(at..at + anchor)?;
        if stride == 0
            && old
                .get(at..at + anchor)
                .is_some_and(|v| compare_rows(v, row).is_eq())
        {
            return Some(at);
        }
        let i = index
            .partition_point(|&p| compare_rows(&old[p as usize..p as usize + anchor], row).is_lt());
        let p = *index.get(i)? as usize;
        compare_rows(&old[p..p + anchor], row).is_eq().then_some(p)
    };
    let mut at = 0;
    std::iter::from_fn(move || {
        if at == values.len() {
            return None;
        }
        let start = at;
        let from = source(at);
        at += if from.is_some() {
            anchor
        } else {
            stride.max(1)
        };
        while at < values.len() {
            let step = stride.max(1);
            let follows = match from {
                Some(p) => old
                    .get(p + at - start..p + at - start + step)
                    .is_some_and(|row| compare_rows(row, &values[at..at + step]).is_eq()),
                None => source(at).is_none(),
            };
            if !follows {
                break;
            }
            at += step;
        }
        Some((from, start, at))
    })
}

/// Two scans choose a smaller payload without storing a range for every word.
fn changed_ranges<'a>(
    values: &'a [f32],
    old: &'a [f32],
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < values.len()
            && old
                .get(i)
                .is_some_and(|v| v.to_bits() == values[i].to_bits())
        {
            i += 1;
        }
        if i == values.len() {
            return None;
        }
        let start = i;
        i += 1;
        while i < values.len()
            && old
                .get(i)
                .is_none_or(|v| v.to_bits() != values[i].to_bits())
        {
            i += 1;
        }
        Some((start, i))
    })
}

#[cfg(test)]
mod tests;
