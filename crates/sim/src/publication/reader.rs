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
//! The view is keyed and shaped as the browser's `ObservationView`, every
//! number the published float32 widened. Fog bits, ground runs and the
//! static groups (corpses, known props, fallen bodies) sit beside it, shared
//! by pointer while unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Map, Value};

use super::{
    fixed_row_width, GroupLayout, AUDIBLE_FIELDS, BLAST_FIELDS, CONTACT_FIELDS, COPY_FIELDS,
    CORPSE_FIELDS, CRASH_FIELDS, DELIVERY_FIELDS, FALLEN_BODY_FIELDS, GROUND_FIELDS,
    GROUP_ENCODINGS, GROUP_LAYOUTS, GUIDED_FIELDS, HEADER, HEARD_WORD_BITS, IDENTIFIED_FIELDS,
    KNOWN_PROP_FIELDS, LEAN_FIELDS, LIMB_BITS, MEMBER_ID_FIELDS, MEMBER_ORDER_FIELDS, MOUNT_FIELDS,
    OBJECTIVE_FIELDS, OWN_FIELDS, PENDING_FIELDS, POSE_FIELDS, PROJECTILE_FIELDS, RANGE_FIELDS,
    REPORT_EVENT_FIELDS, REPORT_TOTAL_FIELDS,
};

/// One side's observation at a tick.
#[derive(Clone, Debug)]
pub struct Observation {
    /// Every key of the browser's view but the static groups; its `fog`
    /// lacks `bits` and its `groundPatch` lacks `runs`.
    pub view: Map<String, Value>,
    /// The [`STATIC_GROUPS`]' views, in that order.
    pub statics: [Arc<Value>; 3],
    /// Visible ground cells, one bit each, row-major.
    pub fog: Arc<Vec<u32>>,
    /// The learned ground's changes, four words a run: tile,
    /// `start + len * 256`, `crater + scorch * 256` and
    /// `tracks + trampled * 256 + cleared * 65536`.
    pub runs: Vec<f32>,
}

/// Groups whose unchanged words keep their previous view.
pub const STATIC_GROUPS: [&str; 3] = ["corpses", "knownProps", "fallenBodies"];

/// What the next record of a stream builds on.
#[derive(Default)]
struct Baseline {
    groups: Vec<Vec<u32>>,
    /// The fog grid (`nx`, `ny`, `cellM`) and its bits.
    fog: Option<([f64; 3], Arc<Vec<u32>>)>,
    statics: Option<[Arc<Value>; 3]>,
}

/// One ordered side-publication stream.
pub struct Reader {
    /// The layout's name tables (`unitKinds`, …) and the ground's `sides`.
    tables: HashMap<String, Vec<Value>>,
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

/// Field `$name` of `$row` under the writer's `$fields`, widened.
macro_rules! f {
    ($row:expr, $fields:expr, $name:literal) => {
        $row[const { at(&$fields, $name) }] as f64
    };
}

const LIMB: f64 = (1u32 << LIMB_BITS) as f64;

/// An exact integer from its limbs; null when absent.
fn limbs(low: f64, high: f64) -> Value {
    if low < 0.0 {
        Value::Null
    } else {
        json!(low + high * LIMB)
    }
}

/// A value, or null where the writer marks absence with a negative.
fn present(v: f64) -> Value {
    if v < 0.0 {
        Value::Null
    } else {
        json!(v)
    }
}

/// Fixed-width points of a flat section.
fn each<const N: usize>(flat: &[f32]) -> std::slice::Iter<'_, [f32; N]> {
    flat.as_chunks::<N>().0.iter()
}

fn points(flat: &[f32], width: usize) -> Value {
    flat.chunks_exact(width)
        .map(|p| p.iter().map(|&v| v as f64).collect::<Vec<_>>())
        .collect()
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
                && same_names(&ground["fields"], &GROUND_FIELDS)
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
        let mut tables: HashMap<String, Vec<Value>> = layout
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), v.as_array()?.clone())))
            .collect();
        tables.insert(
            "sides".into(),
            ground["sides"].as_array().cloned().unwrap_or_default(),
        );
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
                    .position(|s| s == name)
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
        let (observation, grid) = self.frame(words, &data, &groups, tail, baseline)?;
        // Commit only after the complete record has decoded.
        (self.epoch, self.side, self.revision, self.ground_revision) =
            (epoch, side, revision, ground_revision);
        self.base = Baseline {
            groups,
            fog: Some((grid, observation.fog.clone())),
            statics: Some(observation.statics.clone()),
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

    /// A name from one of the layout's tables; an index outside it is an error.
    fn name(&self, table: &str, index: f64) -> Result<Value> {
        let names = &self.tables[table];
        (whole(index) && (index as usize) < names.len())
            .then(|| names[index as usize].clone())
            .ok_or_else(|| format!("{table} index {index} is outside its table"))
    }

    /// A name, or null for a negative index.
    fn optional(&self, table: &str, index: f64) -> Result<Value> {
        if index < 0.0 {
            Ok(Value::Null)
        } else {
            self.name(table, index)
        }
    }

    /// Splits the groups into rows, reads the fog and the ground, and maps
    /// every row to the browser's view. Also returns the fog grid.
    fn frame(
        &self,
        words: &[u32],
        data: &[f32],
        groups: &[Vec<u32>],
        tail: usize,
        base: &Baseline,
    ) -> Result<(Observation, [f64; 3])> {
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
            (full || previous.is_some()) && previous.is_none_or(|(g, _)| *g == grid),
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
            Some((_, bits)) if !full && count == 0 => bits.clone(),
            _ => {
                let mut bits = if full {
                    vec![0; fog_words]
                } else {
                    previous.unwrap().1.to_vec()
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
                Arc::new(bits)
            }
        };
        let runs = self.ground_runs(&words[tail + count..], runs as usize)?;

        let mut view = Map::new();
        let mut statics = base.statics.clone().unwrap_or_default();
        for (g, layout) in GROUP_LAYOUTS.iter().enumerate() {
            let shared = STATIC_GROUPS.iter().position(|&s| s == layout.name);
            if shared.is_some() && base.statics.is_some() && base.groups.get(g) == Some(&groups[g])
            {
                continue;
            }
            let value: Vec<Value> = rows[g]
                .iter()
                .map(|r| self.row(layout.name, r))
                .collect::<Result<_>>()?;
            match shared {
                Some(s) => statics[s] = Arc::new(Value::Array(value)),
                None => {
                    view.insert(layout.name.into(), Value::Array(value));
                }
            }
        }
        let encounter = f!(h, HEADER, "encounterResult");
        let encounter = if encounter < 0.0 {
            Value::Null
        } else {
            json!({ "heldS": f!(h, HEADER, "encounterHeldS"), "result": self.name("encounterResults", encounter)? })
        };
        let [objectives, pending, totals, events] = [
            "objectives",
            "pendingPurchases",
            "reportTotals",
            "reportEvents",
        ]
        .map(|g| view.remove(g).unwrap());
        let reported =
            f!(h, HEADER, "reportTotalsCount") > 0.0 || f!(h, HEADER, "reportEventCount") > 0.0;
        let fixed =
            |names: &[&'static str], i: f64| names.get(i as usize).filter(|_| whole(i)).copied();
        let skirmish = if f!(h, HEADER, "skirmishPhase") < 0.0 {
            Value::Null
        } else {
            json!({
                "phase": fixed(&["preparation", "active", "finished"], f!(h, HEADER, "skirmishPhase")),
                "ready": [f!(h, HEADER, "readyBlue") != 0.0, f!(h, HEADER, "readyRed") != 0.0],
                "preparationRemainingS": f!(h, HEADER, "preparationRemainingS"),
                "credits": f!(h, HEADER, "credits"),
                "scores": [f!(h, HEADER, "scoreBlue"), f!(h, HEADER, "scoreRed")],
                "result": fixed(&["blue", "red", "draw"], f!(h, HEADER, "matchResult")),
                "objectives": objectives,
                "occupiedSlots": f!(h, HEADER, "occupiedSlots"),
                "maxUnits": f!(h, HEADER, "maxUnits"),
                "pending": pending,
                "report": if reported { json!({ "totals": totals, "events": events }) } else { Value::Null },
            })
        };
        let ground = json!({
            "epoch": f!(h, HEADER, "groundEpoch"),
            "side": self.name("sides", f!(h, HEADER, "groundSide"))?,
            "baseRevision": f!(h, HEADER, "groundBase"),
            "revision": f!(h, HEADER, "groundRevision"),
            "full": f!(h, HEADER, "groundFull") == 1.0,
        });
        view.insert("tick".into(), json!(f!(h, HEADER, "tick")));
        view.insert("encounter".into(), encounter);
        view.insert("skirmish".into(), skirmish);
        view.insert("fog".into(), json!({ "cellM": cell_m, "nx": nx, "ny": ny }));
        view.insert("groundPatch".into(), ground);
        Ok((
            Observation {
                view,
                statics,
                fog,
                runs,
            },
            grid,
        ))
    }

    /// One row of group `group` as the browser's view of it.
    fn row(&self, group: &str, r: &Row) -> Result<Value> {
        let (f, s) = (r.fields, &r.sections[..]);
        // A field of this row, under the arm's `FIELDS`.
        macro_rules! v {
            ($n:literal) => {
                f[const { at(FIELDS, $n) }] as f64
            };
        }
        let leans = |flat: &[f32]| -> Result<Value> {
            each::<{ LEAN_FIELDS.len() }>(flat)
                .map(|p| {
                    let side = self.optional("leanSides", f!(p, LEAN_FIELDS, "side"))?;
                    Ok(if side.is_null() {
                        side
                    } else {
                        json!({ "side": side, "at": [f!(p, LEAN_FIELDS, "x"), f!(p, LEAN_FIELDS, "y")] })
                    })
                })
                .collect()
        };
        let per_member = |flat: &[f32], read: fn(&[f32; 4]) -> Value| -> Value {
            each::<{ MEMBER_ID_FIELDS.len() }>(flat).map(read).collect()
        };
        let ids = |p: &[f32; 4]| {
            limbs(
                f!(p, MEMBER_ID_FIELDS, "idLo"),
                f!(p, MEMBER_ID_FIELDS, "idHi"),
            )
        };
        let slots = |p: &[f32; 4]| json!(f!(p, MEMBER_ID_FIELDS, "slot"));
        let active = |p: &[f32; 4]| present(f!(p, MEMBER_ID_FIELDS, "activeMount"));
        let poses = |flat: &[f32]| -> Value {
            each::<{ POSE_FIELDS.len() }>(flat)
                .map(|p| {
                    json!({
                        "mount": f!(p, POSE_FIELDS, "mount"),
                        "bearing": f!(p, POSE_FIELDS, "bearing"),
                        "elevation": f!(p, POSE_FIELDS, "elevation"),
                        "shots": limbs(f!(p, POSE_FIELDS, "shotsLo"), f!(p, POSE_FIELDS, "shotsHi")),
                        "operator": limbs(f!(p, POSE_FIELDS, "operatorLo"), f!(p, POSE_FIELDS, "operatorHi")),
                    })
                })
                .collect()
        };
        let column = |flat: &[f32]| -> Value { flat.iter().map(|&v| v as f64).collect() };
        Ok(match group {
            "own" => {
                const FIELDS: &[&str] = &OWN_FIELDS;
                // Sections in the writer's order (`GROUP_LAYOUTS`).
                let [route, queue, members, member_hp, member_ids, orders, member_leans, sees, mounts, weapon_poses, eyes] =
                    s[..]
                else {
                    unreachable!("own rows carry eleven sections")
                };
                let mounts: Vec<Value> = each::<{ MOUNT_FIELDS.len() }>(mounts)
                    .map(|p| self.mount(p))
                    .collect::<Result<_>>()?;
                let orders: Vec<Value> = each::<{ MEMBER_ORDER_FIELDS.len() }>(orders)
                    .map(|p| {
                        Ok(json!({
                            "spot": [f!(p, MEMBER_ORDER_FIELDS, "x"), f!(p, MEMBER_ORDER_FIELDS, "y")],
                            "coverNow": self.optional("coverTiers", f!(p, MEMBER_ORDER_FIELDS, "coverNow"))?,
                            "coverThere": self.optional("coverTiers", f!(p, MEMBER_ORDER_FIELDS, "coverThere"))?,
                        }))
                    })
                    .collect::<Result<_>>()?;
                let deployment = self.optional("postures", v!("deployTarget"))?;
                let garrison = self.optional("garrisonPhases", v!("garrisonPhase"))?;
                json!({
                    "id": v!("id"),
                    "kind": self.name("unitKinds", v!("kind"))?,
                    "position": [v!("x"), v!("y"), v!("z")],
                    "yaw": v!("yaw"),
                    "goal": if v!("goalX").is_nan() { Value::Null } else { json!([v!("goalX"), v!("goalY")]) },
                    "policy": self.optional("policies", v!("policy"))?,
                    "direction": self.optional("directions", v!("direction"))?,
                    "reversing": v!("reversing") == 1.0,
                    "withdrawing": v!("withdrawing") == 1.0,
                    "protection": if v!("protectionCharges") < 0.0 { Value::Null } else {
                        json!({ "charges": v!("protectionCharges"), "cooldown": present(v!("protectionCooldownProgress")) })
                    },
                    "state": self.name("moveStates", v!("state"))?,
                    "blocker": present(v!("blocker")),
                    "route": points(route, 2),
                    "queue": points(queue, 2),
                    "members": points(members, 3),
                    "memberIds": per_member(member_ids, ids),
                    "memberSlots": per_member(member_ids, slots),
                    "memberActiveMounts": per_member(member_ids, active),
                    "memberOrders": orders,
                    "memberLeans": leans(member_leans)?,
                    "area": if v!("areaM").is_nan() { Value::Null } else {
                        json!({ "anchor": [v!("areaX"), v!("areaY")], "radius": v!("areaM") })
                    },
                    "finalFacing": v!("finalFacing"),
                    "sees": column(sees),
                    "engagement": self.name("engagements", v!("engagement"))?,
                    "mounts": mounts,
                    "weaponPoses": poses(weapon_poses),
                    "hp": v!("hp"),
                    "smoking": v!("smoking") == 1.0,
                    "memberHp": column(member_hp),
                    "suppression": self.name("suppressionTiers", v!("suppression"))?,
                    "concealed": v!("concealed") == 1.0,
                    "deployment": if deployment.is_null() { deployment } else {
                        json!({ "progress": v!("deployProgress"), "target": deployment })
                    },
                    "garrison": if garrison.is_null() { garrison } else {
                        json!({
                            "building": limbs(v!("garrisonBuildingLo"), v!("garrisonBuildingHi")),
                            "phase": garrison,
                            "progress": v!("garrisonProgress"),
                            "center": [v!("garrisonX"), v!("garrisonY")],
                            "half": [v!("garrisonHalfX"), v!("garrisonHalfY")],
                        })
                    },
                    "stock": present(v!("stock")),
                    "service": self.name("serviceStatuses", v!("service"))?,
                    "sight": {
                        "eyes": points(eyes, 3),
                        "forward": v!("sightForward"),
                        "shape": { "front": v!("sightFront"), "side": v!("sightSide"), "rear": v!("sightRear") },
                        "range": v!("sightRange"),
                    },
                })
            }
            "identified" => {
                const FIELDS: &[&str] = &IDENTIFIED_FIELDS;
                let [members, member_ids, member_leans, weapon_poses] = s[..] else {
                    unreachable!("identified rows carry four sections")
                };
                json!({
                    "id": v!("id"),
                    "kind": self.name("unitKinds", v!("kind"))?,
                    "cost": v!("cost"),
                    "position": [v!("x"), v!("y"), v!("z")],
                    "yaw": v!("yaw"),
                    "velocity": [v!("vx"), v!("vy")],
                    "members": points(members, 3),
                    "memberIds": per_member(member_ids, ids),
                    "memberSlots": per_member(member_ids, slots),
                    "memberActiveMounts": per_member(member_ids, active),
                    "memberLeans": leans(member_leans)?,
                    "weaponPoses": poses(weapon_poses),
                    "reversing": v!("reversing") == 1.0,
                    "smoking": v!("smoking") == 1.0,
                })
            }
            "contacts" => {
                const FIELDS: &[&str] = &CONTACT_FIELDS;
                // Bit k of the heard mask, 24 rows a word.
                let mask = v!("heardLow") as i64 | (v!("heardHigh") as i64) << HEARD_WORD_BITS;
                let kinds = &self.tables["roundKinds"];
                let heard: Vec<&Value> = (0..kinds.len())
                    .filter(|&k| mask >> k & 1 == 1)
                    .map(|k| &kinds[k])
                    .collect();
                json!({
                    "id": v!("id"),
                    "source": self.name("contactSources", v!("source"))?,
                    "center": [v!("x"), v!("y"), v!("z")],
                    "layer": self.name("layers", v!("layer"))?,
                    "aloft": v!("aloft") == 1.0,
                    "radius": v!("radius"),
                    "evidenceTick": v!("evidenceTick"),
                    "expiresTick": v!("expiresTick"),
                    "kind": self.optional("unitKinds", v!("kind"))?,
                    "heard": heard,
                })
            }
            "audible" => {
                const FIELDS: &[&str] = &AUDIBLE_FIELDS;
                json!({
                    "listener": v!("listener"),
                    "category": self.name("soundCategories", v!("category"))?,
                    "sector": v!("sector"),
                    "band": self.name("soundBands", v!("band"))?,
                    "moving": v!("moving") == 1.0,
                })
            }
            "knownProps" => {
                const FIELDS: &[&str] = &KNOWN_PROP_FIELDS;
                json!({
                    "kind": self.name("propKinds", v!("kind"))?,
                    "center": [v!("x"), v!("y")],
                    "yaw": v!("yaw"),
                    "half": [v!("hx"), v!("hy"), v!("hz")],
                    "baseZ": v!("baseZ"),
                    "replaces": limbs(v!("replacesLo"), v!("replacesHi")),
                    "id": limbs(v!("idLo"), v!("idHi")),
                    "building": limbs(v!("buildingLo"), v!("buildingHi")),
                    "structureOwner": limbs(v!("structureOwnerLo"), v!("structureOwnerHi")),
                    "authoredProp": limbs(v!("authoredPropLo"), v!("authoredPropHi")),
                    "destroyed": v!("destroyed") == 1.0,
                    "wreckOf": self.optional("unitKinds", v!("wreckOf"))?,
                })
            }
            "projectiles" => {
                const FIELDS: &[&str] = &PROJECTILE_FIELDS;
                let hit = self.name("hitKinds", v!("hit"))?;
                let ricochets: Vec<Value> = each::<4>(s[1])
                    .map(|p| json!({ "point": p[0] as f64, "normal": [p[1] as f64, p[2] as f64, p[3] as f64] }))
                    .collect();
                json!({
                    "path": points(s[0], 3),
                    "ricochets": ricochets,
                    "own": v!("own") == 1.0,
                    "kind": self.name("roundKinds", v!("kind"))?,
                    "shooterMember": limbs(v!("shooterLo"), v!("shooterHi")),
                    "impactNormal": if hit == "none" { Value::Null } else { json!([v!("nx"), v!("ny"), v!("nz")]) },
                    "hit": hit,
                })
            }
            "blasts" => {
                const FIELDS: &[&str] = &BLAST_FIELDS;
                json!({
                    "point": [v!("x"), v!("y"), v!("z")],
                    "radius": v!("radius"),
                    "kind": self.name("roundKinds", v!("kind"))?,
                })
            }
            "guided" => {
                const FIELDS: &[&str] = &GUIDED_FIELDS;
                json!({
                    "id": limbs(v!("idLo"), v!("idHi")),
                    "position": [v!("x"), v!("y"), v!("z")],
                    "point": [v!("px"), v!("py"), v!("pz")],
                    "supported": v!("supported") == 1.0,
                })
            }
            "corpses" => {
                const FIELDS: &[&str] = &CORPSE_FIELDS;
                json!({
                    "position": [v!("x"), v!("y"), v!("z")],
                    "own": v!("own") == 1.0,
                    "soldier": limbs(v!("soldierLo"), v!("soldierHi")),
                    "kind": self.name("unitKinds", v!("kind"))?,
                    "slot": v!("slot"),
                    "yaw": v!("yaw"),
                })
            }
            "fallenBodies" => {
                const FIELDS: &[&str] = &FALLEN_BODY_FIELDS;
                json!({
                    "prop": limbs(v!("propLo"), v!("propHi")),
                    "at": [v!("x"), v!("y")],
                    "toward": [v!("towardX"), v!("towardY")],
                    "tick": v!("tick"),
                })
            }
            "crashes" => {
                const FIELDS: &[&str] = &CRASH_FIELDS;
                json!({
                    "id": v!("id"),
                    "own": v!("own") == 1.0,
                    "kind": self.name("unitKinds", v!("kind"))?,
                    "position": [v!("x"), v!("y"), v!("z")],
                    "yaw": v!("yaw"),
                    "pitch": v!("pitch"),
                    "roll": v!("roll"),
                })
            }
            "objectives" => {
                const FIELDS: &[&str] = &OBJECTIVE_FIELDS;
                let side = |i: f64| {
                    (i >= 0.0)
                        .then(|| ["blue", "red"].get(i as usize))
                        .flatten()
                };
                json!({
                    "id": self.name("objectiveIds", v!("id"))?,
                    "center": [v!("x"), v!("y")],
                    "radiusM": v!("radius"),
                    "owner": side(v!("owner")),
                    "capturing": side(v!("capturing")),
                    "captureProgress": v!("captureProgress"),
                    "contested": v!("contested") != 0.0,
                })
            }
            "reportTotals" => {
                const FIELDS: &[&str] = &REPORT_TOTAL_FIELDS;
                json!({ "spawned": v!("spawned"), "spent": v!("spent"), "killed": v!("killed"), "refunded": v!("refunded") })
            }
            "reportEvents" => {
                const FIELDS: &[&str] = &REPORT_EVENT_FIELDS;
                let (side, unit_id) = (
                    self.name("sides", v!("side"))?,
                    limbs(v!("unitIdLo"), v!("unitIdHi")),
                );
                let (unit_kind, tick) = (
                    self.name("unitKinds", v!("unitKind"))?,
                    limbs(v!("tickLo"), v!("tickHi")),
                );
                match v!("kind") {
                    0.0 => {
                        json!({ "kind": "spawn", "tick": tick, "side": side, "unitId": unit_id, "unitKind": unit_kind, "amount": v!("amount") })
                    }
                    1.0 => {
                        json!({ "kind": "refund", "tick": tick, "side": side, "unitId": unit_id, "unitKind": unit_kind, "amount": v!("amount") })
                    }
                    _ => json!({
                        "kind": "kill", "tick": tick, "side": side, "unitId": unit_id, "unitKind": unit_kind,
                        "victimCost": v!("amount"),
                        "killer": if v!("otherSide") < 0.0 { Value::Null } else {
                            json!({
                                "side": self.name("sides", v!("otherSide"))?,
                                "unitId": limbs(v!("otherIdLo"), v!("otherIdHi")),
                                "unitKind": self.name("unitKinds", v!("otherKind"))?,
                                "cost": v!("otherAmount"),
                            })
                        },
                    }),
                }
            }
            _ => {
                const FIELDS: &[&str] = &PENDING_FIELDS;
                json!({
                    "id": v!("idLo") + v!("idHi") * LIMB,
                    "kind": self.name("unitKinds", v!("kind"))?,
                    "destination": [v!("x"), v!("y")],
                    "confirmedTick": v!("confirmedLo") + v!("confirmedHi") * LIMB,
                    "blocked": v!("blocked") != 0.0,
                })
            }
        })
    }

    fn mount(&self, p: &[f32]) -> Result<Value> {
        macro_rules! v {
            ($n:literal) => {
                f!(p, MOUNT_FIELDS, $n)
            };
        }
        // Ammo per kind: -1 unlimited, -2 no such kind on this mount.
        let ammo: Vec<Value> = [v!("ammo0"), v!("ammo1")]
            .into_iter()
            .take(v!("kinds").max(0.0) as usize)
            .map(|n| if n == -1.0 { Value::Null } else { json!(n) })
            .collect();
        let kind = self.name("targetKinds", v!("targetKind"))?;
        let target = match kind.as_str() {
            Some("none") => Value::Null,
            Some("ground") => {
                json!({ "kind": kind, "point": [v!("targetX"), v!("targetY"), v!("targetZ")] })
            }
            _ => json!({ "kind": kind, "id": v!("targetId") }),
        };
        Ok(json!({
            "mount": v!("mount"),
            "loaded": present(v!("loaded")),
            "ammo": ammo,
            "aim": v!("aim"),
            "reload": v!("reload"),
            "target": target,
            "reason": self.name("actionReasons", v!("reason"))?,
            "guiding": v!("guiding") == 1.0,
            "reloading": present(v!("reloadKind")),
        }))
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

/// Reads lsb-first bit fields from raw u32 carrier words.
struct Bits<'a> {
    words: &'a [u32],
    bit: usize,
}

impl Bits<'_> {
    fn read(&mut self, count: usize) -> Result<u32> {
        check(
            self.bit + count <= self.words.len() * 32,
            "truncated packed publication",
        )?;
        let (index, shift) = (self.bit / 32, self.bit % 32);
        let mut value = (self.words[index] as u64) >> shift;
        if shift + count > 32 {
            value |= (self.words[index + 1] as u64) << (32 - shift);
        }
        self.bit += count;
        Ok((value & ((1u64 << count) - 1)) as u32)
    }

    /// A canonical unsigned LEB128 of at most four bytes.
    fn integer(&mut self) -> Result<u32> {
        let mut value = 0;
        for byte in 0..4 {
            let part = self.read(8)?;
            value += (part & 127) << (byte * 7);
            if part < 128 {
                check(
                    byte == 0 || part != 0,
                    "noncanonical packed publication integer",
                )?;
                return Ok(value);
            }
        }
        Err("oversized packed publication integer".into())
    }

    /// Zero padding to the next carrier word, and no more.
    fn finish(&mut self) -> Result<()> {
        check(
            self.words.len() * 32 - self.bit <= 31,
            "excess packed publication padding",
        )?;
        while self.bit < self.words.len() * 32 {
            check(self.read(1)? == 0, "nonzero packed publication padding")?;
        }
        Ok(())
    }
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
