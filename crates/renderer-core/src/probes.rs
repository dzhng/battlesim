//! On-demand diagnostics over the same GPU owners and exported shaders as a frame.
use crate::fog::{Fog, Layer};
use crate::fog_sight::FogUpload;
use crate::pipelines::{BoundCompute, Pipelines};
use crate::readback::Readback;
use crate::{bind, buffer};
use wgpu::*;

pub mod grass;
pub mod terrain;

#[derive(Clone, Copy, serde::Deserialize)]
pub struct FogPoint {
    pub position: [f32; 3],
    pub normal: Option<[f32; 3]>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PointRecord {
    position: [f32; 3],
    ground: u32,
    normal: [f32; 3],
    pad: u32,
}

/// GPU copies are submitted synchronously; completion owns no live scene borrow.
pub enum ProbeReadback {
    Copied(Readback),
    Values(Vec<u32>),
}
impl ProbeReadback {
    pub async fn read_u32(self, device: &Device) -> Result<Vec<u32>, String> {
        match self {
            Self::Copied(read) => Ok(u32s(&read.read(device).await?).collect()),
            Self::Values(values) => Ok(values),
        }
    }
    pub async fn read_u8(self, device: &Device) -> Result<Vec<u8>, String> {
        Ok(self
            .read_u32(device)
            .await?
            .into_iter()
            .map(|v| v as u8)
            .collect())
    }
    pub async fn read_f32(self, device: &Device) -> Result<Vec<f32>, String> {
        Ok(self
            .read_u32(device)
            .await?
            .into_iter()
            .map(f32::from_bits)
            .collect())
    }
}

pub struct FogProbes {
    probe: BoundCompute,
    shape: BoundCompute,
    probe_layout: BindGroupLayout,
    shape_layout: BindGroupLayout,
}

fn check_storage(device: &Device, label: &str, bytes: u64) -> Result<(), String> {
    if bytes == 0
        || bytes > device.limits().max_storage_buffer_binding_size
        || bytes > device.limits().max_buffer_size
    {
        return Err(format!(
            "{label}: {bytes} bytes exceed available storage bounds"
        ));
    }
    Ok(())
}
fn storage(device: &Device, label: &str, bytes: u64) -> Result<Buffer, String> {
    check_storage(device, label, bytes)?;
    Ok(device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: bytes.max(16),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    }))
}
fn workgroups(device: &Device, count: usize, size: usize) -> Result<u32, String> {
    let groups = count.div_ceil(size);
    if groups > device.limits().max_compute_workgroups_per_dimension as usize {
        return Err("probe rows exceed the GPU dispatch limit".into());
    }
    Ok(groups as u32)
}
fn u32s(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
}

impl FogProbes {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            probe: pipelines.bound_compute("fogProbe"),
            shape: pipelines.bound_compute("fogShape"),
            probe_layout: pipelines.layout("probeLayout").clone(),
            shape_layout: pipelines.layout("shapeLayout").clone(),
        }
    }

    /// `upload` is prepared by canonical presentation for this exact point
    /// count. Only due horizon maps rebuild; the last frame's tiles stay intact.
    pub fn probe(
        &self,
        device: &Device,
        queue: &Queue,
        fog: &mut Fog,
        upload: &FogUpload,
        points: &[FogPoint],
    ) -> Result<ProbeReadback, String> {
        if upload.active && upload.params.probe_count as usize != points.len() {
            return Err("fog upload probe count differs from requested points".into());
        }
        let groups = workgroups(
            device,
            points.len(),
            presentation::fog::sight::BUILD_WORKGROUP,
        )?;
        let size = (points.len() as u64)
            .checked_mul(size_of::<PointRecord>() as u64)
            .ok_or("fog probe records overflow")?;
        if !points.is_empty() {
            check_storage(device, "fog-probe-points", size)?;
        }
        // Even an inactive/empty request consumed canonical retained-table work.
        // Keep it in the GPU owner before the next publication or drawn frame.
        fog.upload_probe(device, queue, upload);
        if !upload.active {
            return Ok(ProbeReadback::Values(vec![1; points.len()]));
        }
        if points.is_empty() {
            let mut encoder = device.create_command_encoder(&Default::default());
            fog.encode_maps(&mut encoder);
            queue.submit([encoder.finish()]);
            return Ok(ProbeReadback::Values(Vec::new()));
        }
        let records: Vec<_> = points
            .iter()
            .map(|p| PointRecord {
                position: p.position,
                ground: u32::from(p.normal.is_none()),
                normal: p.normal.unwrap_or([0.0; 3]),
                pad: 0,
            })
            .collect();
        let input = buffer(
            device,
            queue,
            "fog-probe-points",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&records),
        );
        let output = storage(device, "fog-probe-output", points.len() as u64 * 4)?;
        let group = bind(
            device,
            &self.probe_layout,
            "fog-probe",
            &[input.as_entire_binding(), output.as_entire_binding()],
        );
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("fog-probe"),
        });
        fog.encode_maps(&mut encoder);
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("fog-probe"),
                timestamp_writes: None,
            });
            self.probe.set(
                &mut pass,
                &[fog.group(Layer::Faces), ("probeLayout", &group)],
            );
            pass.dispatch_workgroups(groups, 1, 1);
        }
        let read = Readback::buffer(device, &mut encoder, &output, 0..points.len() as u64 * 4)?;
        queue.submit([encoder.finish()]);
        Ok(ProbeReadback::Copied(read))
    }

    pub fn shape(
        &self,
        device: &Device,
        queue: &Queue,
        rows: &[[f32; 4]],
    ) -> Result<ProbeReadback, String> {
        if rows.is_empty() {
            return Ok(ProbeReadback::Values(Vec::new()));
        }
        let groups = workgroups(
            device,
            rows.len(),
            presentation::fog::sight::BUILD_WORKGROUP,
        )?;
        check_storage(device, "fog-shape-rows", rows.len() as u64 * 16)?;
        let input = buffer(
            device,
            queue,
            "fog-shape-rows",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(rows),
        );
        let output = storage(device, "fog-shape-output", rows.len() as u64 * 4)?;
        let group = bind(
            device,
            &self.shape_layout,
            "fog-shape-probe",
            &[input.as_entire_binding(), output.as_entire_binding()],
        );
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("fog-shape-probe"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("fog-shape-probe"),
                timestamp_writes: None,
            });
            self.shape.set(&mut pass, &[("shapeLayout", &group)]);
            pass.dispatch_workgroups(groups, 1, 1);
        }
        let read = Readback::buffer(device, &mut encoder, &output, 0..rows.len() as u64 * 4)?;
        queue.submit([encoder.finish()]);
        Ok(ProbeReadback::Copied(read))
    }

    pub fn tile_counts(device: &Device, queue: &Queue, fog: &Fog) -> Result<ProbeReadback, String> {
        let Some((source, bytes)) = fog.probe_tile_counts() else {
            return Ok(ProbeReadback::Values(Vec::new()));
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        let read = Readback::buffer(device, &mut encoder, source, 0..bytes)?;
        queue.submit([encoder.finish()]);
        Ok(ProbeReadback::Copied(read))
    }

    pub fn whole_flags(
        device: &Device,
        queue: &Queue,
        fog: &Fog,
        count: usize,
    ) -> Result<ProbeReadback, String> {
        if count == 0 {
            return Ok(ProbeReadback::Values(Vec::new()));
        }
        let bytes = (count as u64)
            .checked_mul(4)
            .ok_or("fog whole flag count overflows")?;
        let mut encoder = device.create_command_encoder(&Default::default());
        let read = Readback::buffer(device, &mut encoder, fog.probe_whole_flags(), 0..bytes)?;
        queue.submit([encoder.finish()]);
        Ok(ProbeReadback::Copied(read))
    }
}
