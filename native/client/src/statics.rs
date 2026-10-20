//! The static world's batches as the client holds them between frames: the
//! renderer's half of `StaticBatches`. Each batch the frame lists of a
//! placed row (a building's module, a prop, the scenery) is held by its key
//! in a buffer of its own, a power of two of records, rewritten only when
//! the frame writes it and dropped when the frame stops listing it, as the
//! browser's model and scenery layers hold theirs. The models and the
//! scenery draw them from here.
use std::collections::{HashMap, HashSet};

use presentation::presentation::{Frame, Presentation};
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
    pub fn new(presentation: &Presentation) -> Self {
        let rows = &presentation.world().appearances;
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
    pub fn prepare(&mut self, device: &Device, queue: &Queue, frame: &Frame) -> Result<(), String> {
        self.listed.clear();
        for b in &frame.batches {
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
                        },
                    );
                }
                let records = &frame.instances[b.first as usize..(b.first + b.count) as usize];
                queue.write_buffer(&self.held[&b.key].buffer, 0, bytemuck::cast_slice(records));
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

    /// The batches the last frame listed, in its order.
    pub fn listed(&self) -> impl Iterator<Item = &Held> {
        self.listed.iter().map(|key| &self.held[key])
    }
}
