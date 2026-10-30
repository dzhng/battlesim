//! Buffer records written by the renderer rather than presentation.
use presentation::effects::lights::CAST_LIGHTS_MAX;
use presentation::records::{Aerial, CastLight, Sun};

presentation::records! {
    /// A diagnostic sight query, with an optional surface normal.
    Struct FogProbe {
        position: [f32; 3],
        ground: u32,
        normal: [f32; 3],
        pad: u32,
    }

    Struct Environment {
        world_to_view: [f32; 16],
        observer: [f32; 4],
        sun: Sun,
        aerial: Aerial,
    }

    Struct CastLights {
        header: [f32; 4],
        lights: [CastLight; CAST_LIGHTS_MAX],
    }

    /// A wave-field run, including a move with no simulation step.
    Struct WaveRun {
        shift: [i32; 2],
        step: u32,
        splats: u32,
        still: u32,
        pad: [u32; 3],
    }

    Struct FogLayer {
        ground: u32,
        seen: u32,
        painted: u32,
        pad: u32,
    }

    Struct GroundPaintStyle {
        albedo: f32,
        emissive: f32,
        fog_keep: f32,
        grass_glow: f32,
        grass_falloff_m: f32,
        saturation: f32,
        pad: [f32; 2],
    }

    Struct OverlayGlow {
        radius_px: f32,
        sigma_px: f32,
        strength: f32,
        pad: f32,
    }
}
