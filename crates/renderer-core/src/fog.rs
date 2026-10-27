//! Shared fog sight compute pipeline ownership.
//!
//! Presentation upload and bind-group lifetime stay in the shell because they
//! depend on its fog record layout. This owner keeps the four exported compute
//! pipelines and their dispatch labels/order in one browser-safe Rust module.

use wgpu::*;

use crate::pipelines::{BoundCompute, Pipelines};

#[derive(Clone, Copy, Debug)]
pub enum FogStage {
    Terrain,
    Merge,
    Cull,
    Whole,
}

pub struct FogComputeOwner {
    terrain: BoundCompute,
    merge: BoundCompute,
    cull: BoundCompute,
    whole: BoundCompute,
}

impl FogComputeOwner {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            terrain: pipelines.bound_compute("fogTerrain"),
            merge: pipelines.bound_compute("fogMerge"),
            cull: pipelines.bound_compute("fogCull"),
            whole: pipelines.bound_compute("fogWhole"),
        }
    }

    /// Encode one fog compute dispatch with the exported pipeline and stable
    /// pass label. The shell supplies the stage's bind groups.
    pub fn dispatch(
        &self,
        encoder: &mut CommandEncoder,
        stage: FogStage,
        groups: &[(&str, &BindGroup)],
        workgroups: [u32; 2],
    ) {
        let (label, pipeline) = match stage {
            FogStage::Terrain => ("fog-terrain", &self.terrain),
            FogStage::Merge => ("fog-merge", &self.merge),
            FogStage::Cull => ("fog-cull", &self.cull),
            FogStage::Whole => ("fog-whole", &self.whole),
        };
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some(label),
            timestamp_writes: None,
        });
        pipeline.set(&mut pass, groups);
        pass.dispatch_workgroups(workgroups[0], workgroups[1], 1);
    }
}
