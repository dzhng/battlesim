//! One submitted grass field, decoded from its actual indirect arguments and clumps.
use super::u32s;
use crate::grass::Grass;
use crate::readback::Readback;
use presentation::static_world::ground::grass as field;
use wgpu::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrassCounts {
    pub near: u32,
    pub far: u32,
    pub near_found: u32,
    pub far_found: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct GrassClump {
    pub root: [f32; 3],
    pub height: f32,
    pub kind: String,
    pub colour: [f64; 3],
    pub tier: u32,
    pub laid: f64,
}

/// Borrowed resources and actual allocation facts for explicit diagnostics.
pub struct GrassProbeSource<'a> {
    pub args: &'a Buffer,
    pub clumps: &'a Buffer,
    pub capacity: [u32; 2],
    pub stride: u64,
}

pub struct CountsReadback(Option<Readback>);
struct ClumpsSnapshot {
    counts: CountsReadback,
    clumps: Readback,
    capacity: [u32; 2],
    stride: u64,
    names: Vec<String>,
}
pub struct ClumpsReadback(Option<ClumpsSnapshot>);

fn counts(bytes: &[u8]) -> Result<GrassCounts, String> {
    let words: Vec<_> = u32s(bytes).collect();
    if words.len() != field::ARGS_WORDS as usize {
        return Err(format!(
            "grass indirect arguments must contain {} words",
            field::ARGS_WORDS
        ));
    }
    Ok(GrassCounts {
        near: words[1],
        far: words[field::DRAW_WORDS as usize + 1],
        near_found: words[field::RAW_COUNT_OFFSET as usize],
        far_found: words[field::RAW_COUNT_OFFSET as usize + 1],
    })
}

pub fn queue_counts(
    device: &Device,
    queue: &Queue,
    grass: &Grass,
) -> Result<CountsReadback, String> {
    let Some(source) = grass.probe_source() else {
        return Ok(CountsReadback(None));
    };
    let mut encoder = device.create_command_encoder(&Default::default());
    let read = Readback::buffer(
        device,
        &mut encoder,
        source.args,
        0..u64::from(field::ARGS_WORDS) * size_of::<u32>() as u64,
    )?;
    queue.submit([encoder.finish()]);
    Ok(CountsReadback(Some(read)))
}

pub fn queue_clumps(
    device: &Device,
    queue: &Queue,
    grass: &Grass,
    names: &[String],
) -> Result<ClumpsReadback, String> {
    let Some(source) = grass.probe_source() else {
        return Ok(ClumpsReadback(None));
    };
    let bytes = (u64::from(source.capacity[0]) + u64::from(source.capacity[1]))
        .checked_mul(source.stride)
        .ok_or("grass clump byte count overflows")?;
    if bytes == 0 {
        return Ok(ClumpsReadback(None));
    }
    if source.stride < 36 || !source.stride.is_multiple_of(4) {
        return Err("grass GPU rows exceed their declared stride".into());
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    let arguments = Readback::buffer(
        device,
        &mut encoder,
        source.args,
        0..u64::from(field::ARGS_WORDS) * size_of::<u32>() as u64,
    )?;
    let clumps = Readback::buffer(device, &mut encoder, source.clumps, 0..bytes)?;
    // Both snapshots belong to the same submission, before the first await.
    queue.submit([encoder.finish()]);
    Ok(ClumpsReadback(Some(ClumpsSnapshot {
        counts: CountsReadback(Some(arguments)),
        clumps,
        capacity: source.capacity,
        stride: source.stride,
        names: names.to_vec(),
    })))
}

impl CountsReadback {
    pub async fn read(self, device: &Device) -> Result<GrassCounts, String> {
        match self.0 {
            None => Ok(GrassCounts::default()),
            Some(read) => counts(&read.read(device).await?),
        }
    }
}
impl ClumpsReadback {
    pub async fn read(self, device: &Device) -> Result<Vec<GrassClump>, String> {
        let Some(snapshot) = self.0 else {
            return Ok(Vec::new());
        };
        let counts = snapshot.counts.read(device).await?;
        if counts.near > snapshot.capacity[0] || counts.far > snapshot.capacity[1] {
            return Err("grass GPU counts exceed their declared capacity".into());
        }
        let bytes = snapshot.clumps.read(device).await?;
        let mut rows = Vec::with_capacity(counts.near as usize + counts.far as usize);
        for (tier, start, count) in [(0, 0, counts.near), (1, snapshot.capacity[0], counts.far)] {
            for i in 0..count {
                let offset = (u64::from(start + i) * snapshot.stride) as usize;
                let word = |at: usize| {
                    u32::from_le_bytes(
                        bytes[offset + at * 4..offset + at * 4 + 4]
                            .try_into()
                            .unwrap(),
                    )
                };
                let colour = word(4);
                rows.push(GrassClump {
                    root: [0, 1, 2].map(|n| f32::from_bits(word(n))),
                    height: f32::from_bits(word(3)),
                    kind: snapshot
                        .names
                        .get(word(5) as usize)
                        .map(String::as_str)
                        .unwrap_or("?")
                        .to_owned(),
                    colour: [0, 8, 16]
                        .map(|shift| (f64::from((colour >> shift) & 255) / 255.0).powi(2)),
                    tier,
                    laid: 1.0 - f64::from(colour >> 24) / 255.0,
                });
            }
        }
        Ok(rows)
    }
}
