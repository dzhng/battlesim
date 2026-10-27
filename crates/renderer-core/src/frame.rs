//! Canonical frame-stage ordering shared by native and browser shells.
//!
//! The shells still own their concrete resources and pass bodies. This module
//! owns the order and debug-group contract so the two platforms cannot drift
//! while the remaining pass owners move into the shared crate.

use wgpu::CommandEncoder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStage {
    SunShadow,
    Depth,
    FogSight,
    World,
    Effects,
    FogMask,
    Overlays,
    Post,
    FogRim,
}

impl FrameStage {
    pub const ORDER: [Self; 9] = [
        Self::SunShadow,
        Self::Depth,
        Self::FogSight,
        Self::World,
        Self::Effects,
        Self::FogMask,
        Self::Overlays,
        Self::Post,
        Self::FogRim,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SunShadow => "sun-shadow",
            Self::Depth => "depth",
            Self::FogSight => "fog-sight",
            Self::World => "world",
            Self::Effects => "effects",
            Self::FogMask => "fog-mask",
            Self::Overlays => "overlays",
            Self::Post => "post",
            Self::FogRim => "fog-rim",
        }
    }
}

pub trait FrameInputs {
    fn encode_stage(&mut self, stage: FrameStage, encoder: &mut CommandEncoder);
}

pub fn encode_frame<I: FrameInputs>(encoder: &mut CommandEncoder, inputs: &mut I) {
    for stage in FrameStage::ORDER {
        encoder.push_debug_group(stage.label());
        inputs.encode_stage(stage, encoder);
        encoder.pop_debug_group();
    }
}

#[cfg(test)]
mod tests {
    use super::FrameStage;

    #[test]
    fn order_is_the_shared_frame_contract() {
        assert_eq!(
            FrameStage::ORDER.map(FrameStage::label),
            [
                "sun-shadow",
                "depth",
                "fog-sight",
                "world",
                "effects",
                "fog-mask",
                "overlays",
                "post",
                "fog-rim",
            ]
        );
    }
}
