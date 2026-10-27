//! Reads the records [`Publisher`](super::Publisher) writes back into one
//! side's observation, for every client.
//!
//! A [`Reader`] follows one ordered stream. It keeps the baseline a record
//! builds on (each group's words, the fog field, the ground cursor) and
//! commits it only once a whole record has decoded. The layout a stream
//! names must be this writer's: its header, groups, sections and delivery
//! fields are checked against the writer's own tables, and only the battle's
//! name tables and ground grid come from it.
//!
//! An [`Observation`] has the browser's `ObservationView` shape and names
//! (`web/src/battle/sim/observationView.ts` documents each field). Every
//! number is the published float32, widened; JSON carries a non-finite one as
//! null. Its serialized form leaves out what a client takes apart: the fog
//! bits, the ground runs, and the static groups (corpses, known props, fallen
//! bodies), which are shared by pointer while unchanged.

mod bits;
mod view;

use bits::Bits;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Value};

use super::{
    fixed_row_width, GroupLayout, AIR_TRANSIT_FIELDS, AUDIBLE_FIELDS, BLAST_FIELDS, CONTACT_FIELDS,
    COPY_FIELDS, CORPSE_FIELDS, CRASH_FIELDS, DELIVERY_FIELDS, FALLEN_BODY_FIELDS, GRIND_FIELDS,
    GROUND_FIELDS, GROUP_ENCODINGS, GROUP_LAYOUTS, GUIDED_FIELDS, HEADER, HEARD_WORD_BITS,
    IDENTIFIED_FIELDS, KNOWN_PROP_FIELDS, LEAN_FIELDS, LIMB_BITS, MEMBER_ID_FIELDS,
    MEMBER_ORDER_FIELDS, MOUNT_FIELDS, OBJECTIVE_FIELDS, OWN_FIELDS, PENDING_FIELDS, PLAYER_FIELDS,
    POSE_FIELDS, PROJECTILE_FIELDS, PROP_DISPLAY_POSE_FIELDS, QUEUE_FIELDS, RANGE_FIELDS,
    REFUSAL_FIELDS, REPORT_EVENT_FIELDS, REPORT_TOTAL_FIELDS, SHOUT_FIELDS,
};

pub use view::*;

/// What the next record of a stream builds on.
#[derive(Default)]
struct Baseline {
    groups: Vec<Vec<u32>>,
    fog: Option<Arc<Fog>>,
    statics: Option<(Shared<Corpse>, Shared<KnownProp>, Shared<FallenBody>)>,
}

/// One ordered side-publication stream.
pub struct Reader {
    /// The layout's name tables (`unitKinds`, …) and the ground's `sides`.
    tables: HashMap<String, Vec<Name>>,
    max_words: usize,
    max_fog_words: usize,
    grid: (usize, usize),
    epoch: f64,
    floor: f64,
    side: Option<usize>,
    expected_side: Option<usize>,
    revision: f64,
    ground_revision: f64,
    base: Baseline,
}

type Result<T> = std::result::Result<T, String>;

fn check(ok: bool, what: &str) -> Result<()> {
    ok.then_some(()).ok_or_else(|| what.to_string())
}

fn is_int(v: f64) -> bool {
    v.is_finite() && v == v.trunc() && v.abs() <= 9_007_199_254_740_991.0
}

fn whole(v: f64) -> bool {
    is_int(v) && v >= 0.0
}

const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut i = 0;
    while i < a.len() && a.len() == b.len() && a[i] == b[i] {
        i += 1;
    }
    a.len() == b.len() && i == a.len()
}

/// A name's index in one of the writer's tables, checked at compile time.
const fn at(names: &[&str], name: &str) -> usize {
    let mut i = 0;
    while !same(names[i], name) {
        i += 1;
    }
    i
}

const fn group(name: &str) -> usize {
    let mut i = 0;
    while !same(GROUP_LAYOUTS[i].name, name) {
        i += 1;
    }
    i
}

/// Field `$name` of `$row` under the writer's `$fields`, widened.
macro_rules! f {
    ($row:expr, $fields:expr, $name:literal) => {
        $row[const { at(&$fields, $name) }] as f64
    };
}

/// Field `$name` of `$row` under the caller's `FIELDS`, widened.
macro_rules! v {
    ($row:expr, $name:literal) => {
        $row[const { at(FIELDS, $name) }] as f64
    };
}

const LIMB: f64 = (1u32 << LIMB_BITS) as f64;

/// An exact integer from its limbs; none when absent.
fn limbs(low: f64, high: f64) -> Option<f64> {
    (low >= 0.0).then_some(low + high * LIMB)
}

/// A value, or none where the writer marks absence with a negative.
fn present(v: f64) -> Option<f64> {
    (v >= 0.0).then_some(v)
}

/// Fixed-width points of a flat section.
fn each<const N: usize>(flat: &[f32]) -> std::slice::Iter<'_, [f32; N]> {
    flat.as_chunks::<N>().0.iter()
}

fn points<const N: usize>(flat: &[f32]) -> Vec<[f64; N]> {
    each::<N>(flat).map(|p| p.map(|v| v as f64)).collect()
}

fn column(flat: &[f32]) -> Vec<f64> {
    flat.iter().map(|&v| v as f64).collect()
}

fn same_names(found: &Value, expected: &[&str]) -> bool {
    found
        .as_array()
        .is_some_and(|a| a.len() == expected.len() && a.iter().zip(expected).all(|(v, e)| v == e))
}

/// A copy's row alignment: a whole row for a group without sections, else one word.
fn stride(group: usize) -> usize {
    fixed_row_width(group).max(1)
}

/// The name a fixed table gives `i`, if any.
fn fixed(names: &[&'static str], i: f64) -> Option<&'static str> {
    names.get(i as usize).filter(|_| whole(i)).copied()
}

/// The fixed name a negative index leaves out.
fn optional_fixed(names: &[&'static str], i: f64) -> Option<&'static str> {
    (i >= 0.0).then(|| fixed(names, i)).flatten()
}

impl Reader {
    /// A reader for the stream `layout_json` names; a layout this writer
    /// doesn't publish is refused.
    pub fn new(layout_json: &str) -> Result<Reader> {
        let layout: Value =
            serde_json::from_str(layout_json).map_err(|e| format!("publication layout: {e}"))?;
        let groups = layout["groups"].as_array().map_or(&[][..], |g| &g[..]);
        let mine = groups.len() == GROUP_LAYOUTS.len()
            && groups.iter().zip(&GROUP_LAYOUTS).all(|(g, e)| {
                g["name"] == e.name
                    && g["count"] == e.count
                    && same_names(&g["fields"], e.fields)
                    && g["sections"].as_array().is_some_and(|s| {
                        s.len() == e.sections.len()
                            && s.iter().zip(e.sections).all(|(s, e)| {
                                s["name"] == e.name
                                    && s["count"] == e.count
                                    && same_names(&s["fields"], e.fields)
                            })
                    })
            });
        let delivery = &layout["groupDelivery"];
        let alignments: Vec<usize> = (0..GROUP_LAYOUTS.len()).map(stride).collect();
        let ground = &layout["ground"];
        check(
            mine && same_names(&layout["header"], &HEADER)
                && same_names(&delivery["fields"], &DELIVERY_FIELDS)
                && same_names(&delivery["range"], &RANGE_FIELDS)
                && same_names(&delivery["copy"], &COPY_FIELDS)
                && same_names(&delivery["encodings"], &GROUP_ENCODINGS)
                && delivery["copyAlignments"] == json!(alignments)
                && layout["fog"]["count"] == "fogFloats"
                && ground["count"] == "groundRunCount"
                && (same_names(&ground["fields"], &GROUND_FIELDS))
                && ground["tileSize"] == 16
                && layout["limbBits"] == LIMB_BITS,
            "the publication layout is not this writer's",
        )?;
        let number = |v: &Value| v.as_u64().unwrap_or(0) as usize;
        let grid = (number(&ground["cols"]), number(&ground["rows"]));
        check(
            grid.0 > 0 && grid.1 > 0 && (grid.0 as u64) * (grid.1 as u64) <= 1 << 32,
            "ground grid does not match its admitted tile codec",
        )?;
        let names = |v: &Value| -> Option<Vec<Name>> {
            v.as_array()?
                .iter()
                .map(|s| s.as_str().map(Name::from))
                .collect()
        };
        let mut tables: HashMap<String, Vec<Name>> = layout
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), names(v)?)))
            .collect();
        tables.insert("sides".into(), names(&ground["sides"]).unwrap_or_default());
        check(
            tables.get("roundKinds").map_or(0, Vec::len) <= super::MAX_WEAPON_ROWS,
            "the layout names more round kinds than a heard mask carries",
        )?;
        Ok(Reader {
            tables,
            max_words: number(&ground["maxRecordBytes"]) / 4,
            max_fog_words: number(&layout["fog"]["maxWords"]),
            grid,
            epoch: 0.0,
            floor: 0.0,
            side: None,
            expected_side: None,
            revision: 0.0,
            ground_revision: 0.0,
            base: Baseline::default(),
        })
    }

    /// In-flight records from before are read as stale, without becoming
    /// observations; the next must open a new epoch (for `side`, when named).
    pub fn invalidate(&mut self, side: Option<&str>) -> Result<()> {
        self.expected_side = match side {
            None => None,
            Some(name) => Some(
                self.tables["sides"]
                    .iter()
                    .position(|s| &**s == name)
                    .ok_or_else(|| format!("the layout has no side {name}"))?,
            ),
        };
        self.floor = self.epoch;
        self.base = Baseline::default();
        Ok(())
    }

    /// Decodes the next record from its raw u32 carrier words. `None` is a
    /// stale record (an older epoch, or another side after
    /// [`invalidate`](Reader::invalidate)); an error changes no baseline.
    pub fn decode(&mut self, words: &[u32]) -> Result<Option<Observation>> {
        let data: Vec<f32> = words.iter().map(|&w| f32::from_bits(w)).collect();
        check(
            words.len() <= self.max_words && data.len() >= HEADER.len(),
            "publication exceeds its admitted record bound",
        )?;
        let h = &data[..HEADER.len()];
        let epoch = f!(h, HEADER, "groundEpoch");
        check(
            is_int(epoch) && epoch > 0.0,
            "publication epoch must be positive",
        )?;
        if epoch < self.epoch || epoch <= self.floor {
            return Ok(None);
        }
        let tag = f!(h, HEADER, "groundSide");
        let side =
            (whole(tag) && (tag as usize) < self.tables["sides"].len()).then_some(tag as usize);
        if self.expected_side.is_some() && side != self.expected_side {
            return Ok(None);
        }
        let exact = |lo: f64, hi: f64| {
            (whole(lo) && whole(hi) && lo < LIMB && hi < LIMB).then_some(lo + hi * LIMB)
        };
        let (Some(base), Some(revision)) = (
            exact(f!(h, HEADER, "fogBaseLo"), f!(h, HEADER, "fogBaseHi")),
            exact(
                f!(h, HEADER, "fogRevisionLo"),
                f!(h, HEADER, "fogRevisionHi"),
            ),
        ) else {
            return Err("fog revision limbs must be exact 16-bit integers".into());
        };
        let fresh = epoch > self.epoch;
        let (ground_base, ground_revision) =
            (f!(h, HEADER, "groundBase"), f!(h, HEADER, "groundRevision"));
        let ground_full = f!(h, HEADER, "groundFull");
        check(
            side.is_some()
                && whole(ground_base)
                && is_int(ground_revision)
                && ground_base <= ground_revision
                && ground_revision < (1 << 24) as f64
                && if fresh {
                    ground_full == 1.0 && ground_base == 0.0
                } else {
                    ground_full == 0.0 && ground_base == self.ground_revision && side == self.side
                },
            "ground publication does not follow its side/epoch/revision baseline",
        )?;
        check(
            if fresh {
                f!(h, HEADER, "fogFull") == 1.0 && base == 0.0
            } else {
                side == self.side && base == self.revision
            },
            "fog publication does not follow its side/epoch/revision baseline",
        )?;
        check(revision > base, "fog publication revision must advance")?;
        let empty = Baseline::default();
        let baseline = if fresh { &empty } else { &self.base };
        let (groups, tail) = self.reconstruct(words, &data, &baseline.groups)?;
        let observation = self.frame(words, &data, &groups, tail, baseline)?;
        // Commit only after the complete record has decoded.
        (self.epoch, self.side, self.revision, self.ground_revision) =
            (epoch, side, revision, ground_revision);
        self.base = Baseline {
            groups,
            fog: Some(observation.fog.clone()),
            statics: Some((
                observation.corpses.clone(),
                observation.known_props.clone(),
                observation.fallen_bodies.clone(),
            )),
        };
        Ok(Some(observation))
    }

    /// Reconstructs every non-map group's words from its delivery against
    /// `previous`; the fog and ground tail starts at the returned offset.
    fn reconstruct(
        &self,
        words: &[u32],
        data: &[f32],
        previous: &[Vec<u32>],
    ) -> Result<(Vec<Vec<u32>>, usize)> {
        let (mut cursor, mut length) = (HEADER.len(), HEADER.len());
        let mut groups = Vec::with_capacity(GROUP_LAYOUTS.len());
        for g in 0..GROUP_LAYOUTS.len() {
            check(
                cursor + 3 <= data.len(),
                "invalid observation group payload",
            )?;
            let [size, encoding, payload] = [0, 1, 2].map(|i| data[cursor + i] as f64);
            cursor += DELIVERY_FIELDS.len();
            let mode =
                (whole(encoding) && encoding < 4.0).then(|| GROUP_ENCODINGS[encoding as usize]);
            check(
                whole(size)
                    && whole(payload)
                    && mode.is_some()
                    && length as f64 + size <= self.max_words as f64
                    && cursor as f64 + payload <= data.len() as f64
                    && (!previous.is_empty() || matches!(mode, Some("snapshot" | "packed"))),
                "invalid observation group payload",
            )?;
            let (size, end) = (size as usize, cursor + payload as usize);
            let old = previous.get(g).map_or(&[][..], |v| &v[..]);
            let values = match mode.unwrap() {
                "packed" => unpack_group(
                    g,
                    &words[cursor..end],
                    size,
                    (!previous.is_empty()).then_some(old),
                )?,
                "snapshot" => {
                    check(
                        end - cursor == size,
                        "observation group snapshot length mismatch",
                    )?;
                    words[cursor..end].to_vec()
                }
                "copies" => {
                    let stride = stride(g);
                    check(
                        size.is_multiple_of(stride),
                        "observation copy alignment disagrees with its group",
                    )?;
                    let mut values = Vec::with_capacity(size);
                    let mut at = cursor;
                    while at < end {
                        check(at + 2 <= end, "invalid observation row copy")?;
                        let (source, count) = (data[at] as f64, data[at + 1] as f64);
                        at += 2;
                        check(
                            whole(count)
                                && count > 0.0
                                && (count as usize).is_multiple_of(stride)
                                && values.len() as f64 + count <= size as f64
                                && if source == -1.0 {
                                    at as f64 + count <= end as f64
                                } else {
                                    whole(source)
                                        && (source as usize).is_multiple_of(stride)
                                        && source + count <= old.len() as f64
                                },
                            "invalid observation row copy",
                        )?;
                        let count = count as usize;
                        if source == -1.0 {
                            values.extend_from_slice(&words[at..at + count]);
                            at += count;
                        } else {
                            let source = source as usize;
                            values.extend_from_slice(&old[source..source + count]);
                        }
                    }
                    check(
                        values.len() == size,
                        "observation row copies do not fill their group",
                    )?;
                    values
                }
                _ => {
                    let mut values = old[..size.min(old.len())].to_vec();
                    values.resize(size, 0);
                    let (mut last, mut at) = (0.0, cursor);
                    while at < end {
                        check(at + 2 <= end, "invalid observation group replacement range")?;
                        let (start, count) = (data[at] as f64, data[at + 1] as f64);
                        at += 2;
                        check(
                            whole(start)
                                && whole(count)
                                && count > 0.0
                                && start >= last
                                && start + count <= size as f64
                                && at as f64 + count <= end as f64
                                && !(start > old.len() as f64 && last < start),
                            "invalid observation group replacement range",
                        )?;
                        let (start, count) = (start as usize, count as usize);
                        values[start..start + count].copy_from_slice(&words[at..at + count]);
                        at += count;
                        last = (start + count) as f64;
                    }
                    check(
                        size <= old.len() || last >= size as f64,
                        "observation group growth must supply its new words",
                    )?;
                    values
                }
            };
            groups.push(values);
            cursor = end;
            length += size;
            check(
                length <= self.max_words,
                "reconstructed observation exceeds its admitted record bound",
            )?;
        }
        check(
            length + data.len() - cursor <= self.max_words,
            "reconstructed observation exceeds its admitted record bound",
        )?;
        Ok((groups, cursor))
    }

    /// Splits the groups into rows, reads the fog and the ground, and maps
    /// every row to its view.
    fn frame(
        &self,
        words: &[u32],
        data: &[f32],
        groups: &[Vec<u32>],
        tail: usize,
        base: &Baseline,
    ) -> Result<Observation> {
        let h = &data[..HEADER.len()];
        let floats: Vec<Vec<f32>> = groups
            .iter()
            .map(|g| g.iter().map(|&w| f32::from_bits(w)).collect())
            .collect();
        let mut rows: Vec<Vec<Row>> = Vec::with_capacity(groups.len());
        for (g, layout) in GROUP_LAYOUTS.iter().enumerate() {
            let payload = &floats[g][..];
            let count = h[HEADER.iter().position(|n| *n == layout.count).unwrap()] as f64;
            check(
                whole(count) && count * layout.fields.len() as f64 <= payload.len() as f64,
                "observation group row count exceeds its payload",
            )?;
            rows.push(split_rows(layout, payload, count as usize)?);
        }

        let grid = [
            f!(h, HEADER, "fogNx"),
            f!(h, HEADER, "fogNy"),
            f!(h, HEADER, "fogCellM"),
        ];
        let [nx, ny, cell_m] = grid;
        let cells = if whole(nx) && whole(ny) {
            nx * ny
        } else {
            f64::INFINITY
        };
        let fog_words = (cells / 32.0).ceil();
        check(
            fog_words <= self.max_fog_words as f64 && (cells == 0.0 || cell_m > 0.0),
            "fog field dimensions exceed the admitted delivery bound",
        )?;
        let (cells, fog_words) = (cells as usize, fog_words as usize);
        let (count, fog_full) = (f!(h, HEADER, "fogFloats"), f!(h, HEADER, "fogFull"));
        check(
            fog_full == 0.0 || fog_full == 1.0,
            "unknown fog payload encoding",
        )?;
        let full = fog_full == 1.0;
        check(
            whole(count)
                && if full {
                    count == (fog_words * 2) as f64
                } else {
                    (count as usize).is_multiple_of(3) && count <= (fog_words * 2) as f64
                },
            "fog payload does not match its encoding",
        )?;
        let previous = base.fog.as_ref();
        check(
            (full || previous.is_some()) && previous.is_none_or(|p| [p.nx, p.ny, p.cell_m] == grid),
            "fog delta has no matching field baseline",
        )?;
        let count = count as usize;
        let runs = f!(h, HEADER, "groundRunCount");
        check(whole(runs), "ground runs do not match their count")?;
        let logical = (HEADER.len() + groups.iter().map(Vec::len).sum::<usize>() + count) as f64
            + runs * GROUND_FIELDS.len() as f64;
        check(
            tail + count <= data.len() && logical <= self.max_words as f64,
            "publication length does not match its layout",
        )?;
        let fog = match previous {
            Some(p) if !full && count == 0 => p.clone(),
            _ => {
                let mut bits = if full {
                    vec![0; fog_words]
                } else {
                    previous.unwrap().bits.clone()
                };
                let word = |at: usize| -> Result<u32> {
                    let (lo, hi) = (data[at] as f64, data[at + 1] as f64);
                    check(
                        whole(lo) && whole(hi) && lo < LIMB && hi < LIMB,
                        "fog word limbs must be exact 16-bit integers",
                    )?;
                    Ok(lo as u32 | (hi as u32) << LIMB_BITS)
                };
                if full {
                    for (i, bit) in bits.iter_mut().enumerate() {
                        *bit = word(tail + i * 2)?;
                    }
                } else {
                    let mut last = -1.0;
                    for at in (tail..tail + count).step_by(3) {
                        let index = data[at] as f64;
                        check(
                            is_int(index) && index > last && index < fog_words as f64,
                            "fog delta word indices must be ordered and inside the field",
                        )?;
                        bits[index as usize] = word(at + 1)?;
                        last = index;
                    }
                }
                check(
                    cells % 32 == 0 || bits[fog_words - 1] >> (cells % 32) == 0,
                    "fog padding bits must be zero",
                )?;
                Arc::new(Fog {
                    cell_m,
                    nx,
                    ny,
                    bits,
                })
            }
        };
        let runs = self.ground_runs(&words[tail + count..], runs as usize)?;

        let kept = |g: usize| base.statics.is_some() && base.groups.get(g) == Some(&groups[g]);
        let statics = base.statics.as_ref();
        let (corpses, props, fallen) =
            (group("corpses"), group("knownProps"), group("fallenBodies"));
        let skirmish = if f!(h, HEADER, "skirmishPhase") < 0.0 {
            None
        } else {
            let pending = map(&rows[group("pendingPurchases")], |r| self.pending(r))?;
            let mut players = map(&rows[group("players")], |r| self.player(r))?;
            for purchase in pending {
                if let Some(player) = players.iter_mut().find(|p| p.index == purchase.owner) {
                    player.pending.push(purchase);
                } else {
                    return Err("pending purchase names an unpublished player".into());
                }
            }
            let reported =
                f!(h, HEADER, "reportTotalsCount") > 0.0 || f!(h, HEADER, "reportEventCount") > 0.0;
            Some(Skirmish {
                phase: fixed(
                    &["preparation", "active", "finished"],
                    f!(h, HEADER, "skirmishPhase"),
                ),
                players,
                preparation_remaining_s: f!(h, HEADER, "preparationRemainingS"),
                scores: [f!(h, HEADER, "scoreBlue"), f!(h, HEADER, "scoreRed")],
                result: optional_fixed(&["blue", "red", "draw"], f!(h, HEADER, "matchResult")),
                objectives: map(&rows[group("objectives")], |r| self.objective(r))?,
                report: if reported {
                    Some(Report {
                        totals: map(&rows[group("reportTotals")], |r| Ok(totals(r)))?,
                        events: map(&rows[group("reportEvents")], |r| self.event(r))?,
                    })
                } else {
                    None
                },
            })
        };
        let encounter = f!(h, HEADER, "encounterResult");
        Ok(Observation {
            tick: f!(h, HEADER, "tick"),
            own: map(&rows[group("own")], |r| self.own(r))?,
            identified: map(&rows[group("identified")], |r| self.identified(r))?,
            contacts: map(&rows[group("contacts")], |r| self.contact(r))?,
            audible: map(&rows[group("audible")], |r| self.cue(r))?,
            refusals: map(&rows[group("refusals")], |r| self.refusal(r))?,
            known_props: match statics {
                Some((_, kept_props, _)) if kept(props) => kept_props.clone(),
                _ => Arc::new(map(&rows[props], |r| self.known_prop(r))?),
            },
            projectiles: map(&rows[group("projectiles")], |r| self.projectile(r))?,
            blasts: map(&rows[group("blasts")], |r| self.blast(r))?,
            air_transits: map(&rows[group("airTransits")], |r| air_transit(self, r))?,
            corpses: match statics {
                Some((kept_corpses, _, _)) if kept(corpses) => kept_corpses.clone(),
                _ => Arc::new(map(&rows[corpses], |r| self.corpse(r))?),
            },
            fallen_bodies: match statics {
                Some((_, _, kept_fallen)) if kept(fallen) => kept_fallen.clone(),
                _ => Arc::new(map(&rows[fallen], |r| Ok(fallen_body(r)))?),
            },
            crashes: map(&rows[group("crashes")], |r| self.crash(r))?,
            shouts: map(&rows[group("shouts")], |r| self.shout(r))?,
            guided: map(&rows[group("guided")], |r| Ok(guided(r)))?,
            encounter: if encounter < 0.0 {
                None
            } else {
                Some(Encounter {
                    held_s: f!(h, HEADER, "encounterHeldS"),
                    result: self.name("encounterResults", encounter)?,
                })
            },
            skirmish,
            fog,
            ground_patch: GroundPatch {
                epoch: f!(h, HEADER, "groundEpoch"),
                side: self.name("sides", f!(h, HEADER, "groundSide"))?,
                base_revision: f!(h, HEADER, "groundBase"),
                revision: f!(h, HEADER, "groundRevision"),
                full: f!(h, HEADER, "groundFull") == 1.0,
                runs,
            },
            prop_display_poses: map(&rows[group("propDisplayPoses")], |r| {
                Ok(prop_display_pose(r))
            })?,
            grinds: map(&rows[group("grinds")], |r| Ok(grind(r)))?,
        })
    }

    /// A name from one of the layout's tables; an index outside it is an error.
    fn name(&self, table: &str, index: f64) -> Result<Name> {
        let names = &self.tables[table];
        (whole(index) && (index as usize) < names.len())
            .then(|| names[index as usize].clone())
            .ok_or_else(|| format!("{table} index {index} is outside its table"))
    }

    /// A name, or none for a negative index.
    fn optional(&self, table: &str, index: f64) -> Result<Option<Name>> {
        (index >= 0.0).then(|| self.name(table, index)).transpose()
    }

    fn leans(&self, flat: &[f32]) -> Result<Vec<Option<MemberLean>>> {
        const FIELDS: &[&str] = &LEAN_FIELDS;
        each::<{ LEAN_FIELDS.len() }>(flat)
            .map(|p| {
                let side = self.optional("leanSides", v!(p, "side"))?;
                Ok(side.map(|side| MemberLean {
                    side,
                    at: [v!(p, "x"), v!(p, "y")],
                }))
            })
            .collect()
    }

    fn own(&self, r: &Row) -> Result<OwnUnit> {
        const FIELDS: &[&str] = &OWN_FIELDS;
        let f = r.fields;
        // Sections in the writer's order (`GROUP_LAYOUTS`).
        let [route, queue, members, member_hp, member_ids, orders, leans, sees, mounts, poses, eyes] =
            r.sections[..]
        else {
            unreachable!("own rows carry eleven sections")
        };
        let charges = v!(f, "protectionCharges");
        let orders = each::<{ MEMBER_ORDER_FIELDS.len() }>(orders).map(|p| {
            const FIELDS: &[&str] = &MEMBER_ORDER_FIELDS;
            Ok(MemberOrder {
                spot: [v!(p, "x"), v!(p, "y")],
                cover_now: self.optional("coverTiers", v!(p, "coverNow"))?,
                cover_there: self.optional("coverTiers", v!(p, "coverThere"))?,
            })
        });
        Ok(OwnUnit {
            withdrawing: v!(f, "withdrawing") == 1.0,
            fuel_s: present(v!(f, "fuelSeconds")),
            afterburner: v!(f, "afterburner") == 1.0,
            in_dogfight: v!(f, "inDogfight") == 1.0,
            flares: (v!(f, "flareCharges") >= 0.0).then(|| Protection {
                charges: v!(f, "flareCharges"),
                cooldown: present(v!(f, "flareCooldown")),
            }),
            id: v!(f, "id"),
            owner: present(v!(f, "owner")),
            kind: self.name("unitKinds", v!(f, "kind"))?,
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            yaw: v!(f, "yaw"),
            goal: (!v!(f, "goalX").is_nan()).then(|| [v!(f, "goalX"), v!(f, "goalY")]),
            goal_unloads: v!(f, "goalUnloads") == 1.0,
            policy: self.optional("policies", v!(f, "policy"))?,
            direction: self.optional("directions", v!(f, "direction"))?,
            reversing: v!(f, "reversing") == 1.0,
            protection: (charges >= 0.0).then(|| Protection {
                charges,
                cooldown: present(v!(f, "protectionCooldownProgress")),
            }),
            state: self.name("moveStates", v!(f, "state"))?,
            blocker: present(v!(f, "blocker")),
            route: points(route),
            queue: each::<{ QUEUE_FIELDS.len() }>(queue)
                .map(|&[x, y, _]| [x as f64, y as f64])
                .collect(),
            queue_unloads: each::<{ QUEUE_FIELDS.len() }>(queue)
                .map(|&[_, _, unloads]| unloads == 1.0)
                .collect(),
            members: points(members),
            member_ids: member_column(member_ids, |p| limbs(p[0] as f64, p[1] as f64)),
            member_slots: member_column(member_ids, |p| p[2] as f64),
            member_active_mounts: member_column(member_ids, |p| present(p[3] as f64)),
            member_orders: orders.collect::<Result<_>>()?,
            member_leans: self.leans(leans)?,
            area: (!v!(f, "areaM").is_nan()).then(|| SquadArea {
                anchor: [v!(f, "areaX"), v!(f, "areaY")],
                radius: v!(f, "areaM"),
            }),
            final_facing: v!(f, "finalFacing"),
            sees: column(sees),
            engagement: self.name("engagements", v!(f, "engagement"))?,
            mounts: each::<{ MOUNT_FIELDS.len() }>(mounts)
                .map(|p| self.mount(p))
                .collect::<Result<_>>()?,
            weapon_poses: weapon_poses(poses),
            hp: v!(f, "hp"),
            smoking: v!(f, "smoking") == 1.0,
            member_hp: column(member_hp),
            suppression: self.name("suppressionTiers", v!(f, "suppression"))?,
            concealed: v!(f, "concealed") == 1.0,
            deployment: self
                .optional("postures", v!(f, "deployTarget"))?
                .map(|target| Deployment {
                    progress: v!(f, "deployProgress"),
                    target,
                }),
            hold: self
                .optional("holdPhases", v!(f, "holdPhase"))?
                .map(|phase| -> Result<Hold> {
                    let id = limbs(v!(f, "hostLo"), v!(f, "hostHi")).unwrap_or(0.0);
                    let kind = self.name("hostKinds", v!(f, "hostKind"))?;
                    let host = if &*kind == "building" {
                        Host::Building {
                            id,
                            center: [v!(f, "hostX"), v!(f, "hostY")],
                            half: [v!(f, "hostHalfX"), v!(f, "hostHalfY")],
                        }
                    } else {
                        Host::Carrier { id }
                    };
                    Ok(Hold {
                        host,
                        phase,
                        progress: v!(f, "holdProgress"),
                    })
                })
                .transpose()?,
            stock: present(v!(f, "stock")),
            service: self.name("serviceStatuses", v!(f, "service"))?,
            sight: Sight {
                eyes: points(eyes),
                forward: v!(f, "sightForward"),
                shape: SightShape {
                    front: v!(f, "sightFront"),
                    side: v!(f, "sightSide"),
                    rear: v!(f, "sightRear"),
                },
                range: v!(f, "sightRange"),
            },
        })
    }

    fn refusal(&self, r: &Row) -> Result<Refusal> {
        const FIELDS: &[&str] = &REFUSAL_FIELDS;
        Ok(Refusal {
            unit: limbs(v!(r.fields, "unitLo"), v!(r.fields, "unitHi")).unwrap_or(0.0),
            reason: self.name("orderErrors", v!(r.fields, "reason"))?,
        })
    }

    fn mount(&self, p: &[f32; MOUNT_FIELDS.len()]) -> Result<Mount> {
        const FIELDS: &[&str] = &MOUNT_FIELDS;
        // Ammo per kind: -1 unlimited, -2 no such kind on this mount.
        let ammo = [v!(p, "ammo0"), v!(p, "ammo1")]
            .into_iter()
            .take(v!(p, "kinds").max(0.0) as usize)
            .map(|n| (n != -1.0).then_some(n))
            .collect();
        let kind = self.name("targetKinds", v!(p, "targetKind"))?;
        let target = match &*kind {
            "none" => None,
            "ground" => Some(MountTarget {
                kind,
                id: None,
                point: Some([v!(p, "targetX"), v!(p, "targetY"), v!(p, "targetZ")]),
            }),
            _ => Some(MountTarget {
                kind,
                id: Some(v!(p, "targetId")),
                point: None,
            }),
        };
        Ok(Mount {
            mount: v!(p, "mount"),
            loaded: present(v!(p, "loaded")),
            ammo,
            aim: v!(p, "aim"),
            reload: v!(p, "reload"),
            target,
            reason: self.name("actionReasons", v!(p, "reason"))?,
            guiding: v!(p, "guiding") == 1.0,
            reloading: present(v!(p, "reloadKind")),
        })
    }

    fn identified(&self, r: &Row) -> Result<Identified> {
        const FIELDS: &[&str] = &IDENTIFIED_FIELDS;
        let f = r.fields;
        let [members, member_ids, leans, poses] = r.sections[..] else {
            unreachable!("identified rows carry four sections")
        };
        Ok(Identified {
            id: v!(f, "id"),
            kind: self.name("unitKinds", v!(f, "kind"))?,
            cost: v!(f, "cost"),
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            yaw: v!(f, "yaw"),
            velocity: [v!(f, "vx"), v!(f, "vy")],
            members: points(members),
            member_ids: member_column(member_ids, |p| limbs(p[0] as f64, p[1] as f64)),
            member_slots: member_column(member_ids, |p| p[2] as f64),
            member_active_mounts: member_column(member_ids, |p| present(p[3] as f64)),
            member_leans: self.leans(leans)?,
            weapon_poses: weapon_poses(poses),
            reversing: v!(f, "reversing") == 1.0,
            smoking: v!(f, "smoking") == 1.0,
        })
    }

    fn contact(&self, r: &Row) -> Result<Contact> {
        const FIELDS: &[&str] = &CONTACT_FIELDS;
        let f = r.fields;
        // Bit k of the heard mask, 24 rows a word.
        let mask = [v!(f, "heardLow"), v!(f, "heardMid"), v!(f, "heardHigh")]
            .into_iter()
            .enumerate()
            .fold(0_u64, |mask, (w, word)| {
                mask | (word as u64) << (w * HEARD_WORD_BITS)
            });
        let kinds = &self.tables["roundKinds"];
        Ok(Contact {
            id: v!(f, "id"),
            source: self.name("contactSources", v!(f, "source"))?,
            center: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            layer: self.name("layers", v!(f, "layer"))?,
            aloft: v!(f, "aloft") == 1.0,
            radius: v!(f, "radius"),
            evidence_tick: v!(f, "evidenceTick"),
            expires_tick: v!(f, "expiresTick"),
            kind: self.optional("unitKinds", v!(f, "kind"))?,
            heard: (0..kinds.len())
                .filter(|&k| mask >> k & 1 == 1)
                .map(|k| kinds[k].clone())
                .collect(),
            bearing_deg: present(v!(f, "bearingDeg")),
            range_m: present(v!(f, "rangeM")),
        })
    }

    fn cue(&self, r: &Row) -> Result<SoundCue> {
        const FIELDS: &[&str] = &AUDIBLE_FIELDS;
        let f = r.fields;
        Ok(SoundCue {
            listener: v!(f, "listener"),
            category: self.name("soundCategories", v!(f, "category"))?,
            sector: v!(f, "sector"),
            band: self.name("soundBands", v!(f, "band"))?,
            moving: v!(f, "moving") == 1.0,
        })
    }

    fn known_prop(&self, r: &Row) -> Result<KnownProp> {
        const FIELDS: &[&str] = &KNOWN_PROP_FIELDS;
        let f = r.fields;
        Ok(KnownProp {
            kind: self.name("propKinds", v!(f, "kind"))?,
            center: [v!(f, "x"), v!(f, "y")],
            yaw: v!(f, "yaw"),
            half: [v!(f, "hx"), v!(f, "hy"), v!(f, "hz")],
            base_z: v!(f, "baseZ"),
            replaces: limbs(v!(f, "replacesLo"), v!(f, "replacesHi")),
            id: limbs(v!(f, "idLo"), v!(f, "idHi")),
            building: limbs(v!(f, "buildingLo"), v!(f, "buildingHi")),
            structure_owner: limbs(v!(f, "structureOwnerLo"), v!(f, "structureOwnerHi")),
            authored_prop: limbs(v!(f, "authoredPropLo"), v!(f, "authoredPropHi")),
            destroyed: v!(f, "destroyed") == 1.0,
            wreck_of: self.optional("unitKinds", v!(f, "wreckOf"))?,
        })
    }

    fn projectile(&self, r: &Row) -> Result<Projectile> {
        const FIELDS: &[&str] = &PROJECTILE_FIELDS;
        let f = r.fields;
        let hit = self.name("hitKinds", v!(f, "hit"))?;
        Ok(Projectile {
            path: points(r.sections[0]),
            ricochets: each::<4>(r.sections[1])
                .map(|p| Ricochet {
                    point: p[0] as f64,
                    normal: [p[1], p[2], p[3]].map(f64::from),
                })
                .collect(),
            own: v!(f, "own") == 1.0,
            kind: self.name("roundKinds", v!(f, "kind"))?,
            shooter_member: limbs(v!(f, "shooterLo"), v!(f, "shooterHi")),
            impact_normal: (&*hit != "none").then(|| [v!(f, "nx"), v!(f, "ny"), v!(f, "nz")]),
            hit,
        })
    }

    fn blast(&self, r: &Row) -> Result<Blast> {
        const FIELDS: &[&str] = &BLAST_FIELDS;
        let f = r.fields;
        Ok(Blast {
            point: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            radius: v!(f, "radius"),
            kind: self.name("roundKinds", v!(f, "kind"))?,
        })
    }

    fn corpse(&self, r: &Row) -> Result<Corpse> {
        const FIELDS: &[&str] = &CORPSE_FIELDS;
        let f = r.fields;
        Ok(Corpse {
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            own: v!(f, "own") == 1.0,
            soldier: limbs(v!(f, "soldierLo"), v!(f, "soldierHi")),
            kind: self.name("unitKinds", v!(f, "kind"))?,
            slot: v!(f, "slot"),
            yaw: v!(f, "yaw"),
        })
    }

    fn crash(&self, r: &Row) -> Result<Crash> {
        const FIELDS: &[&str] = &CRASH_FIELDS;
        let f = r.fields;
        Ok(Crash {
            id: v!(f, "id"),
            own: v!(f, "own") == 1.0,
            kind: self.name("unitKinds", v!(f, "kind"))?,
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            yaw: v!(f, "yaw"),
            pitch: v!(f, "pitch"),
            roll: v!(f, "roll"),
        })
    }

    fn shout(&self, r: &Row) -> Result<Shout> {
        const FIELDS: &[&str] = &SHOUT_FIELDS;
        let f = r.fields;
        Ok(Shout {
            own: v!(f, "own") == 1.0,
            id: v!(f, "idLo") + v!(f, "idHi") * LIMB,
            kind: self.name("unitKinds", v!(f, "kind"))?,
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            trigger: self.name("shoutTriggers", v!(f, "trigger"))?,
            intensity: self.name("intensities", v!(f, "intensity"))?,
        })
    }

    fn objective(&self, r: &Row) -> Result<Objective> {
        const FIELDS: &[&str] = &OBJECTIVE_FIELDS;
        let f = r.fields;
        Ok(Objective {
            id: self.name("objectiveIds", v!(f, "id"))?,
            center: [v!(f, "x"), v!(f, "y")],
            radius_m: v!(f, "radius"),
            owner: optional_fixed(&["blue", "red"], v!(f, "owner")),
            capturing: optional_fixed(&["blue", "red"], v!(f, "capturing")),
            capture_progress: v!(f, "captureProgress"),
            contested: v!(f, "contested") != 0.0,
        })
    }

    fn pending(&self, r: &Row) -> Result<PendingPurchase> {
        const FIELDS: &[&str] = &PENDING_FIELDS;
        let f = r.fields;
        Ok(PendingPurchase {
            owner: v!(f, "owner"),
            id: v!(f, "idLo") + v!(f, "idHi") * LIMB,
            kind: self.name("unitKinds", v!(f, "kind"))?,
            destination: [v!(f, "x"), v!(f, "y")],
            confirmed_tick: v!(f, "confirmedLo") + v!(f, "confirmedHi") * LIMB,
            blocked: v!(f, "blocked") != 0.0,
            arrival_tick: v!(f, "arrivalLo") + v!(f, "arrivalHi") * LIMB,
        })
    }

    fn player(&self, r: &Row) -> Result<Player> {
        const FIELDS: &[&str] = &PLAYER_FIELDS;
        let f = r.fields;
        let id_index = v!(f, "id");
        let id = self
            .tables
            .get("playerIds")
            .and_then(|ids| ids.get(id_index as usize))
            .cloned()
            .ok_or_else(|| format!("player id index {id_index} is outside its table"))?;
        Ok(Player {
            index: v!(f, "index"),
            id,
            controller: self.name("controllers", v!(f, "controller"))?,
            ready: v!(f, "ready") != 0.0,
            credits: v!(f, "credits"),
            unit_count: v!(f, "unitCount"),
            cap: v!(f, "cap"),
            pending: Vec::new(),
        })
    }

    fn event(&self, r: &Row) -> Result<ReportEvent> {
        const FIELDS: &[&str] = &REPORT_EVENT_FIELDS;
        let f = r.fields;
        let killer = if v!(f, "otherSide") < 0.0 {
            None
        } else {
            Some(Killer {
                side: self.name("sides", v!(f, "otherSide"))?,
                unit_id: limbs(v!(f, "otherIdLo"), v!(f, "otherIdHi")),
                unit_kind: self.name("unitKinds", v!(f, "otherKind"))?,
                owner: present(v!(f, "otherOwner")),
                cost: v!(f, "otherAmount"),
            })
        };
        let kill = v!(f, "kind") != 0.0 && v!(f, "kind") != 1.0;
        Ok(ReportEvent {
            kind: fixed(&["spawn", "refund"], v!(f, "kind")).unwrap_or("kill"),
            tick: limbs(v!(f, "tickLo"), v!(f, "tickHi")),
            side: self.name("sides", v!(f, "side"))?,
            unit_id: limbs(v!(f, "unitIdLo"), v!(f, "unitIdHi")),
            unit_kind: self.name("unitKinds", v!(f, "unitKind"))?,
            owner: present(v!(f, "owner")),
            amount: (!kill).then(|| v!(f, "amount")),
            victim_cost: kill.then(|| v!(f, "amount")),
            killer: kill.then_some(killer),
        })
    }

    /// The ground's run tail, admitted and checked against the grid.
    fn ground_runs(&self, tail: &[u32], n: usize) -> Result<Vec<f32>> {
        check(
            n * GROUND_FIELDS.len() <= self.max_words && n * 29 <= tail.len() * 32,
            "ground runs do not match their count",
        )?;
        let mut bits = Bits {
            words: tail,
            bit: 0,
        };
        let mut runs = vec![0f32; n * 4];
        let mut tile = 0u64;
        for row in runs.as_chunks_mut::<4>().0.iter_mut() {
            tile += bits.integer()? as u64;
            let (start, len, mask) = (bits.read(8)?, bits.read(8)? + 1, bits.read(5)?);
            // crater, scorch | tracks, trampled, cleared
            let mut marks = [0u32; 2];
            for mark in (0..5).filter(|m| mask & (1 << m) != 0) {
                let value = bits.read(8)?;
                check(
                    value != 0,
                    "ground mark presence must encode a nonzero byte",
                )?;
                let (word, shift) = if mark < 2 { (0, mark) } else { (1, mark - 2) };
                marks[word] += value << (shift * 8);
            }
            check(
                tile < 1 << 24,
                "ground tile exceeds its exact address range",
            )?;
            let span = start + len * 256;
            row.copy_from_slice(&[tile as f32, span as f32, marks[0] as f32, marks[1] as f32]);
        }
        bits.finish()?;
        let (cols, rows) = self.grid;
        let tiles_x = cols.div_ceil(16);
        let (mut prior, mut end) = (-1i64, 0usize);
        for row in each::<4>(&runs) {
            let (tile, span) = (row[0] as usize, row[1] as usize);
            let (start, len) = (span % 256, span / 256);
            check(
                tile < tiles_x * rows.div_ceil(16)
                    && len > 0
                    && start + len <= 256
                    && tile as i64 >= prior
                    && !(tile as i64 == prior && start < end),
                "ground run is unordered, overlapping or outside its exact encoding",
            )?;
            let (x, y, last) = (
                (tile % tiles_x) * 16,
                (tile / tiles_x) * 16,
                start + len - 1,
            );
            check(
                y + last / 16 < rows
                    && x + start % 16 < cols
                    && (x + 16 <= cols || (start / 16 == last / 16 && x + last % 16 < cols)),
                "ground run crosses unused cells in a partial edge tile",
            )?;
            (prior, end) = (tile as i64, start + len);
        }
        Ok(runs)
    }
}

/// One row of a group: its fields, then each section's points flat, in the
/// writer's section order.
struct Row<'a> {
    fields: &'a [f32],
    sections: Vec<&'a [f32]>,
}

/// Each row's variable sections follow all the rows, in row order.
fn split_rows<'a>(group: &GroupLayout, payload: &'a [f32], count: usize) -> Result<Vec<Row<'a>>> {
    let width = group.fields.len();
    let mut rows: Vec<Row> = payload[..count * width]
        .chunks_exact(width)
        .map(|fields| Row {
            fields,
            sections: Vec::new(),
        })
        .collect();
    let mut offset = count * width;
    for row in &mut rows {
        for section in group.sections {
            let n = row.fields[group
                .fields
                .iter()
                .position(|f| *f == section.count)
                .unwrap()];
            let end = offset as f64 + n as f64 * section.fields.len() as f64;
            check(
                whole(n as f64) && end <= payload.len() as f64,
                "observation group section count exceeds its payload",
            )?;
            row.sections.push(&payload[offset..end as usize]);
            offset = end as usize;
        }
    }
    check(
        offset == payload.len(),
        "observation group values do not match their counts",
    )?;
    Ok(rows)
}

/// The packed delivery's bit grammar (`groupDelivery.packed`).
fn unpack_group(
    group: usize,
    packed: &[u32],
    size: usize,
    baseline: Option<&[u32]>,
) -> Result<Vec<u32>> {
    let mut bits = Bits {
        words: packed,
        bit: 0,
    };
    let form = bits.read(8)?;
    check(
        form <= 2 && (baseline.is_some() || form == 1),
        "packed observation group requires its baseline",
    )?;
    let old = baseline.unwrap_or(&[]);
    let mut result = vec![0u32; size];
    let literal = |bits: &mut Bits, at: usize| -> Result<u32> {
        let tag = bits.read(4)?;
        check(
            tag <= 9 && !(form == 1 && tag >= 5),
            "invalid packed observation literal tag",
        )?;
        let count = if tag < 5 { tag } else { tag - 5 } as usize;
        let value = if count == 0 { 0 } else { bits.read(count * 8)? };
        Ok(if tag < 5 {
            value
        } else {
            value ^ old.get(at).copied().unwrap_or(0)
        })
    };
    match form {
        1 => {
            for (at, word) in result.iter_mut().enumerate() {
                *word = literal(&mut bits, at)?;
            }
        }
        0 => {
            let keep = size.min(old.len());
            result[..keep].copy_from_slice(&old[..keep]);
            let operations = bits.integer()? as usize;
            check(
                operations <= size,
                "invalid packed observation replacement count",
            )?;
            let mut last = 0;
            for _ in 0..operations {
                let (start, count) = (bits.integer()? as usize, bits.integer()? as usize);
                check(
                    count != 0
                        && start >= last
                        && start + count <= size
                        && !(start > old.len() && last < start),
                    "invalid packed observation replacement range",
                )?;
                for (at, word) in result.iter_mut().enumerate().skip(start).take(count) {
                    *word = literal(&mut bits, at)?;
                }
                last = start + count;
            }
            check(
                size <= old.len() || last >= size,
                "packed observation growth must supply its new words",
            )?;
        }
        _ => {
            let stride = stride(group);
            check(
                size.is_multiple_of(stride),
                "packed observation copy alignment disagrees with its group",
            )?;
            let mut at = 0;
            while at < size {
                let (source, count) = (bits.integer()? as usize, bits.integer()? as usize);
                check(
                    count != 0
                        && count.is_multiple_of(stride)
                        && at + count <= size
                        && (source == 0
                            || ((source - 1).is_multiple_of(stride)
                                && source - 1 + count <= old.len())),
                    "invalid packed observation source copy",
                )?;
                for k in at..at + count {
                    result[k] = if source == 0 {
                        literal(&mut bits, k)?
                    } else {
                        old[source - 1 + k - at]
                    };
                }
                at += count;
            }
        }
    }
    bits.finish()?;
    Ok(result)
}

fn map<T>(rows: &[Row], read: impl Fn(&Row) -> Result<T>) -> Result<Vec<T>> {
    rows.iter().map(read).collect()
}

/// One value per member, from the `memberIds` section (id limbs, slot, mount).
fn member_column<T>(flat: &[f32], read: impl Fn(&[f32; 4]) -> T) -> Vec<T> {
    each::<{ MEMBER_ID_FIELDS.len() }>(flat).map(read).collect()
}

fn weapon_poses(flat: &[f32]) -> Vec<WeaponPose> {
    const FIELDS: &[&str] = &POSE_FIELDS;
    each::<{ POSE_FIELDS.len() }>(flat)
        .map(|p| WeaponPose {
            mount: v!(p, "mount"),
            bearing: v!(p, "bearing"),
            elevation: v!(p, "elevation"),
            shots: limbs(v!(p, "shotsLo"), v!(p, "shotsHi")),
            operator: limbs(v!(p, "operatorLo"), v!(p, "operatorHi")),
        })
        .collect()
}

fn guided(r: &Row) -> Guided {
    const FIELDS: &[&str] = &GUIDED_FIELDS;
    let f = r.fields;
    Guided {
        id: limbs(v!(f, "idLo"), v!(f, "idHi")),
        position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
        point: [v!(f, "px"), v!(f, "py"), v!(f, "pz")],
        supported: v!(f, "supported") == 1.0,
    }
}

fn air_transit(reader: &Reader, r: &Row) -> Result<AirTransit> {
    const FIELDS: &[&str] = &AIR_TRANSIT_FIELDS;
    let f = r.fields;
    Ok(AirTransit {
        kind: reader.name("unitKinds", v!(f, "kind"))?,
        point: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
        heading: v!(f, "heading"),
        speed_mps: v!(f, "speedMps"),
        edge_tick: v!(f, "edgeTick"),
        dir: reader.name("airTransitDirections", v!(f, "dir"))?,
    })
}

fn fallen_body(r: &Row) -> FallenBody {
    const FIELDS: &[&str] = &FALLEN_BODY_FIELDS;
    let f = r.fields;
    FallenBody {
        prop: limbs(v!(f, "propLo"), v!(f, "propHi")),
        at: [v!(f, "x"), v!(f, "y")],
        toward: [v!(f, "towardX"), v!(f, "towardY")],
        tick: v!(f, "tick"),
    }
}

fn prop_display_pose(r: &Row) -> PropDisplayPose {
    const FIELDS: &[&str] = &PROP_DISPLAY_POSE_FIELDS;
    let f = r.fields;
    PropDisplayPose {
        prop_id: v!(f, "propId"),
        center: [v!(f, "x"), v!(f, "y")],
        yaw: v!(f, "yaw"),
        base_z: v!(f, "baseZ"),
    }
}

fn grind(r: &Row) -> Grind {
    const FIELDS: &[&str] = &GRIND_FIELDS;
    let f = r.fields;
    Grind {
        point: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
        heading: [v!(f, "headingX"), v!(f, "headingY")],
    }
}

fn totals(r: &Row) -> ReportTotals {
    const FIELDS: &[&str] = &REPORT_TOTAL_FIELDS;
    let f = r.fields;
    ReportTotals {
        spawned: v!(f, "spawned"),
        spent: v!(f, "spent"),
        killed: v!(f, "killed"),
        refunded: v!(f, "refunded"),
    }
}
