//! Fog surface classes and the shared sight compute pipelines.
//! Canonical presentation uploads, GPU resources and encoding live in the
//! shared sight owner used by both platform shells.

pub use crate::fog_sight::FogSightResources as Fog;

use presentation::records::{FOG_FACES, FOG_GROUND, FOG_PAINTED_FACES};
use wgpu::*;

/// What a drawn surface is to fog (`FogLayer`): its `ground`, `seen` and
/// `painted` words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Layer {
    /// The terrain, its grass and the backdrop.
    PaintedGround,
    /// What movers stand on that fog takes face by face: the water, a
    /// bridge deck.
    PaintedFaces,
    /// What lies on the ground as a body: the fallen.
    Ground,
    /// Buildings, props, scenery and wreck pieces, face by face.
    Faces,
    /// Units: never fogged.
    Units,
}

impl Layer {
    /// The layer a batch's fog class names (`Batch.fog`).
    pub fn of(fog: u32) -> Self {
        match fog {
            FOG_FACES => Layer::Faces,
            FOG_GROUND => Layer::Ground,
            FOG_PAINTED_FACES => Layer::PaintedFaces,
            _ => Layer::Units,
        }
    }
}

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

    /// Encode one fog compute dispatch with the shared pipeline and stable
    /// pass label. The fog owner supplies the stage's bind groups.
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
