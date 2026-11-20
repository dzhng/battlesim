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
use contract::ids::Side;
use contract::observation::{HostRef, MemberLean, ObservationFrame, ShoutSpeaker, ShoutTrigger};
use contract::players::ControllerKind;
use contract::scenario::CoverTier;

use crate::battle::Battle;
use crate::ground::GroundRunPatch;

mod cells;
mod crash_row;
mod deployment;
mod identified_row;
mod layout;
mod logical;
mod mount_row;
mod operations;
mod packing;
mod pending_row;
mod player_row;
mod publisher;
pub mod reader;
mod report_row;
mod smoke;
mod step_row;
mod tags;

pub(crate) use cells::limbs;
use cells::{footprint_cells, host_cells, limbs_or_absent};
use crash_row::{crash_row, CRASH_FIELDS};
use identified_row::{identified_row, IDENTIFIED_FIELDS};
pub use layout::layout_json;
use layout::pose;
use logical::pack_record;
use mount_row::{mount_row, MOUNT_FIELDS};
use operations::{
    changed_ranges, compare_rows, measure_group, pack_group, source_copies, VARIABLE_ANCHOR_WORDS,
};
use packing::{pack_ground, PackedCount, PackedWriter};
use pending_row::{pending_row, PENDING_FIELDS};
use player_row::{player_row, PLAYER_FIELDS};
pub use publisher::{Publisher, Publishers};
use report_row::{report_event_row, REPORT_EVENT_FIELDS};
pub use smoke::smoke_looks;
use smoke::{smoke_cloud_row, SMOKE_CLOUD_FIELDS};
use step_row::{step_row, STEP_FIELDS};
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
const POSE_FIELDS: [&str; 8] = [
    "mount",
    "bearing",
    "elevation",
    "round",
    "shotsLo",
    "shotsHi",
    "operatorLo",
    "operatorHi",
];

/// Header words; the non-map groups' delivery metadata starts here.
pub const HEADER_WORDS: usize = HEADER.len();
const HEADER: [&str; 49] = [
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
    "smokeCloudCount",
    "flareCount",
    "offerCount",
    "scenarioChapter",
    "beatCount",
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
const BLAST_FIELDS: [&str; 6] = ["x", "y", "z", "radius", "kind", "hit"];
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
const FLARE_FIELDS: [&str; 6] = ["ax", "ay", "az", "bx", "by", "bz"];
/// A narrowed shop's purchase: its owner among the side's players, its unit
/// type and the carrier a squad is bought in (-1 for none).
const OFFER_FIELDS: [&str; 3] = ["owner", "kind", "carrier"];
/// A scenario beat the side was told of: its place among the director's
/// beats (the layout's `beats` table) and the tick it fired.
const BEAT_FIELDS: [&str; 3] = ["beat", "firedLo", "firedHi"];
const GRIND_FIELDS: [&str; 5] = ["x", "y", "z", "headingX", "headingY"];
const REPORT_TOTAL_FIELDS: [&str; 4] = ["spawned", "spent", "killed", "refunded"];
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
const OWN_FIELDS: [&str; 59] = [
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
    "stepCount",
    "memberCount",
    "seesCount",
    "engagement",
    "mountCount",
    "hp",
    "suppression",
    "concealed",
    "deployProgress",
    "deployTarget",
    "grounded",
    "hostKind",
    "hostLo",
    "hostHi",
    "holdPhase",
    "holdProgress",
    "hostX",
    "hostY",
    "hostHalfX",
    "hostHalfY",
    "hostSeats",
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
    "smokeCharges",
    "smoking",
    "shotAt",
];
const XY_FIELDS: [&str; 2] = ["x", "y"];
const XYZ_FIELDS: [&str; 3] = ["x", "y", "z"];
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
            section("steps", "stepCount", &STEP_FIELDS),
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
    rows("smokeClouds", "smokeCloudCount", &SMOKE_CLOUD_FIELDS),
    rows("flares", "flareCount", &FLARE_FIELDS),
    rows("offers", "offerCount", &OFFER_FIELDS),
    rows("beats", "beatCount", &BEAT_FIELDS),
];

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
/// Fog/ground payloads still follow their supplied cursor headers; a smoke
/// cloud's look is indexed in `looks`, the battle's [`smoke_looks`].
/// Admission precedes output allocation.
pub fn pack_logical(
    frame: &ObservationFrame,
    looks: &[&str],
    ground: &GroundHeader,
    patch: &FogPatch<'_>,
    runs: impl Iterator<Item = GroundRunPatch>,
    out: &mut Vec<f32>,
) -> Result<(), String> {
    pack_record(frame, looks, ground, patch, runs, out, None)
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

/// Non-map groups, in record order.
pub const GROUPS: usize = 25;

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

#[cfg(test)]
mod tests;
