//! The transport's end of publication: one side's stream, and a battle's
//! streams to every side it serves.
use contract::ids::Side;

use super::{
    encode_delivery, pack_record, smoke_looks, FogPatch, GroundHeader, GROUPS,
    MAX_PUBLICATION_BYTES,
};
use crate::battle::Battle;

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
            &smoke_looks(battle.rules()),
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

/// A battle's publications to every side it serves: one [`Publisher`] per
/// side, so Blue and Red published in the same tick each get a delta against
/// their own baseline. Epochs rise across the sides: a consumer that leaves
/// one side's stream for another's reads the snapshot it opens with as new.
#[derive(Default)]
pub struct Publishers {
    sides: [Publisher; Side::ALL.len()],
}

impl Publishers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pack `side`'s observation into its own stream.
    pub fn publish(&mut self, battle: &Battle, side: Side) -> Result<&[f32], String> {
        self.sides[side.index()].publish(battle, side)
    }

    /// `side`'s latest packed record; empty before its first.
    pub fn record(&self, side: Side) -> &[f32] {
        self.sides[side.index()].record()
    }

    /// `side`'s next record opens an epoch later than any side has reached,
    /// with a full snapshot: its consumer reconnected, reset its view, or
    /// came over from another side. The other sides' streams carry on.
    pub fn resync(&mut self, side: Side) {
        let latest = self.sides.iter().map(|p| p.epoch).max().unwrap_or(0);
        let publisher = &mut self.sides[side.index()];
        publisher.epoch = latest;
        publisher.cursor = None;
    }
}
