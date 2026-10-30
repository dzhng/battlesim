//! The static world's batches as the client holds them between frames: the
//! renderer's half of `StaticBatches`. Each batch the frame lists of a
//! placed row (a building's module, a prop, the scenery) is held by its key
//! in a buffer of its own, a power of two of records, rewritten only when
//! the frame writes it and dropped when the frame stops listing it.
//! Models and scenery draw these same held buffers on both targets.
use std::collections::{HashMap, HashSet};

use presentation::records::{Instance, NONE};
use wgpu::*;

/// One held batch: a run of records over one row's tier, in one pass, with
/// the tier it casts with and how fog takes it (`Batch`).
pub struct Held {
    pub key: u32,
    pub row: u32,
    pub tier: u32,
    pub pass: u32,
    pub caster: u32,
    pub cascade_mask: u32,
    pub fog: u32,
    pub count: u32,
    pub buffer: Buffer,
    records: Vec<Instance>,
}

pub struct Statics {
    /// Per `World.appearances` row, whether it is placed (its batches are
    /// the static world's).
    placed: Vec<bool>,
    held: HashMap<u32, Held>,
    /// The keys the last frame listed, in its order.
    listed: Vec<u32>,
}

impl Statics {
    pub fn from_rows(rows: &[presentation::records::Appearance]) -> Self {
        Self {
            placed: rows.iter().map(|a| a.placed != 0).collect(),
            held: HashMap::new(),
            listed: Vec::new(),
        }
    }

    /// Takes `frame`'s static batches: writes those it wrote, keeps those it
    /// names unchanged, and drops those it no longer lists. A batch named
    /// unchanged that is not held is refused: the frame's batches were
    /// written for a renderer that took the frames between.
    pub fn prepare_records(
        &mut self,
        device: &Device,
        queue: &Queue,
        instances: &[Instance],
        batches: &[presentation::records::Batch],
    ) -> Result<(), String> {
        self.listed.clear();
        for b in batches {
            if !self.holds(b.appearance) {
                continue;
            }
            if b.first != NONE {
                let bytes =
                    u64::from(b.count.max(1).next_power_of_two()) * size_of::<Instance>() as u64;
                let fits = self
                    .held
                    .get(&b.key)
                    .is_some_and(|h| h.buffer.size() == bytes);
                if !fits {
                    let buffer = device.create_buffer(&BufferDescriptor {
                        label: Some("static-batch"),
                        size: bytes,
                        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.held.insert(
                        b.key,
                        Held {
                            key: b.key,
                            row: b.appearance,
                            tier: b.tier,
                            pass: b.pass,
                            caster: b.caster,
                            cascade_mask: b.cascade_mask,
                            fog: b.fog,
                            count: b.count,
                            buffer,
                            records: Vec::new(),
                        },
                    );
                }
                let records = &instances[b.first as usize..(b.first + b.count) as usize];
                let held = self
                    .held
                    .get_mut(&b.key)
                    .expect("a written static batch is held");
                held.records.clear();
                held.records.extend_from_slice(records);
                queue.write_buffer(&held.buffer, 0, bytemuck::cast_slice(records));
            }
            let Some(h) = self.held.get_mut(&b.key) else {
                return Err(format!(
                    "static batch {} (row {}) is named unchanged, and none is held",
                    b.key, b.appearance
                ));
            };
            (
                h.row,
                h.tier,
                h.pass,
                h.caster,
                h.cascade_mask,
                h.fog,
                h.count,
            ) = (
                b.appearance,
                b.tier,
                b.pass,
                b.caster,
                b.cascade_mask,
                b.fog,
                b.count,
            );
            self.listed.push(b.key);
        }
        let listed: HashSet<u32> = self.listed.iter().copied().collect();
        self.held.retain(|key, _| listed.contains(key));
        Ok(())
    }

    /// Whether `row`'s batches are the static world's, held here.
    pub fn holds(&self, row: u32) -> bool {
        self.placed.get(row as usize).copied().unwrap_or(false)
    }

    /// The buffer of the held batch `key`.
    pub fn buffer(&self, key: u32) -> &Buffer {
        &self.held[&key].buffer
    }

    /// CPU records retained with a held GPU batch for device reconstruction.
    pub fn records(&self, key: u32) -> &[Instance] {
        &self.held[&key].records
    }

    /// The batches the last frame listed, in its order.
    pub fn listed(&self) -> impl Iterator<Item = &Held> {
        self.listed.iter().map(|key| &self.held[key])
    }
}

impl Statics {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for held in self.held.values() {
            memory.buffer(&held.buffer);
        }
    }
}
