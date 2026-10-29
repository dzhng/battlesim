//! Value constraints for the typed view sections. Bounds protect geometry,
//! fades and GPU records, rather than pinning any authored tuning.
use super::PresentationSettings;
use serde::Serialize;
use serde_json::Value;

pub(crate) fn range(path: &str, value: f64, lo: f64, hi: f64) -> Result<(), String> {
    if value.is_finite() && (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(format!("{path}: must be within [{lo}, {hi}], got {value}"))
    }
}

/// All numeric inputs must be finite. Most visual magnitudes are unsigned;
/// signed offsets/angles are named explicitly by their section's reader.
pub(crate) fn numbers(value: &impl Serialize, path: &str, signed: &[&str]) -> Result<(), String> {
    fn walk(v: &Value, path: &str, signed: &[&str]) -> Result<(), String> {
        match v {
            Value::Number(n) => range(
                path,
                n.as_f64().unwrap_or(f64::NAN),
                if signed.iter().any(|s| path.ends_with(s)) {
                    -f64::MAX
                } else {
                    0.0
                },
                f64::MAX,
            ),
            Value::Null => Err(format!("{path}: expected a finite number")),
            Value::Object(o) => {
                for (k, v) in o {
                    if !v.is_null() {
                        walk(v, &format!("{path}.{k}"), signed)?;
                    }
                }
                Ok(())
            }
            Value::Array(a) => {
                for (i, v) in a.iter().enumerate() {
                    walk(v, &format!("{path}[{i}]"), signed)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    walk(
        &serde_json::to_value(value).map_err(|e| format!("{path}: {e}"))?,
        path,
        signed,
    )
}

pub(super) fn presentation(s: &PresentationSettings) -> Result<(), String> {
    ground(&s.terrain, &s.grass)?;
    numbers(&s.camera, "presentation.camera", &[".yaw"])?;
    numbers(&s.overlay, "presentation.overlay", &[])?;
    numbers(&s.hud, "presentation.hud", &[])?;
    numbers(&s.contacts, "presentation.contacts", &[".hatch_angle_deg"])?;
    numbers(&s.models, "presentation.models", &[])?;
    numbers(&s.buildings, "presentation.buildings", &[])?;
    numbers(&s.stand_ins.tints, "presentation.stand_ins.tints", &[])?;
    numbers(&s.transit, "presentation.transit", &[])?;
    range(
        "presentation.ground.backdrop_reach_m",
        s.ground.backdrop_reach_m,
        0.0,
        1e6,
    )?;
    let positive = |p: &str, v| range(&format!("presentation.{p}"), v, f64::MIN_POSITIVE, f64::MAX);
    positive("buildings.chunk_m", s.buildings.chunk_m)?;
    positive("buildings.pool_records", s.buildings.pool_records as f64)?;
    let c = &s.camera;
    range(
        "presentation.camera.fov_y",
        c.fov_y,
        0.01,
        std::f64::consts::PI - 0.01,
    )?;
    positive("camera.near_m", c.near_m)?;
    positive("camera.zoom_min", c.zoom_min)?;
    range(
        "presentation.camera.zoom_max",
        c.zoom_max,
        c.zoom_min,
        f64::MAX,
    )?;
    if c.pitch_curve.is_empty()
        || c.pitch_curve
            .windows(2)
            .any(|w| w[0][0] >= w[1][0] || w[0][1] > w[1][1])
    {
        return Err(
            "presentation.camera.pitch_curve: must rise in distance and never fall in pitch".into(),
        );
    }
    for [distance, pitch] in &c.pitch_curve {
        positive("camera.pitch_curve.distance", *distance)?;
        range(
            "presentation.camera.pitch_curve.pitch",
            *pitch,
            0.12,
            std::f64::consts::FRAC_PI_2 - 0.02,
        )?;
    }
    if c.pitch_curve[0][0] > c.zoom_min || c.pitch_curve.last().unwrap()[0] < c.zoom_max {
        return Err("presentation.camera.pitch_curve: must cover the zoom range".into());
    }
    let clearance = &c.clearance;
    for (key, v) in [
        ("lift_steps", clearance.lift_steps),
        ("slide_steps", clearance.slide_steps),
        ("sweep_max_steps", clearance.sweep_max_steps),
    ] {
        if v.fract() != 0.0 {
            return Err(format!(
                "presentation.camera.clearance.{key}: must be an integer"
            ));
        }
        range(
            &format!("presentation.camera.clearance.{key}"),
            v,
            1.0,
            64.0,
        )?;
    }
    positive("camera.clearance.smoothing_s", clearance.smoothing_s)?;
    positive("camera.clearance.sweep_step_m", clearance.sweep_step_m)?;
    for (key, value) in [
        ("hud", serde_json::to_value(&s.hud)),
        ("overlay", serde_json::to_value(&s.overlay)),
        ("contacts", serde_json::to_value(&s.contacts)),
        ("map_border", serde_json::to_value(&s.map_border)),
    ] {
        display(
            &value.map_err(|e| e.to_string())?,
            &format!("presentation.{key}"),
        )?;
    }
    range(
        "presentation.camera.clearance.refine_steps",
        clearance.refine_steps,
        0.0,
        16.0,
    )?;
    if clearance.refine_steps.fract() != 0.0 {
        return Err("presentation.camera.clearance.refine_steps: must be an integer".into());
    }
    for (key, v) in [
        ("release_m", clearance.release_m),
        ("lift_max_m", clearance.lift_max_m),
        ("slide_max_m", clearance.slide_max_m),
        ("slide_max_rad", clearance.slide_max_rad),
    ] {
        positive(&format!("camera.clearance.{key}"), v)?;
    }
    range(
        "presentation.buildings.chunk_m",
        s.buildings.chunk_m,
        16.0,
        f64::MAX,
    )?;
    range(
        "presentation.buildings.pool_records",
        s.buildings.pool_records as f64,
        1024.0,
        f64::MAX,
    )?;
    range(
        "presentation.buildings.tint_jitter",
        s.buildings.tint_jitter,
        0.0,
        0.5,
    )?;
    if !s.buildings.lod_px_per_m.windows(2).all(|w| w[0] > w[1])
        || s.buildings.lod_px_per_m[2] <= 0.0
    {
        return Err("presentation.buildings.lod_px_per_m: must fall and remain positive".into());
    }
    let overlay = &s.overlay;
    range(
        "presentation.overlay.glow.radius_px",
        overlay.glow.radius_px,
        f64::MIN_POSITIVE,
        24.0,
    )?;
    for (key, v) in [
        ("overlay", overlay.glow.overlay),
        ("ground", overlay.glow.ground),
        ("callouts", overlay.glow.callouts),
    ] {
        range(&format!("presentation.overlay.glow.{key}"), v, 0.0, 8.0)?;
    }
    let stroke = &overlay.stroke;
    positive("overlay.stroke.full_m_per_px", stroke.full_m_per_px)?;
    if stroke.thin_m_per_px <= stroke.full_m_per_px {
        return Err("presentation.overlay.stroke.thin_m_per_px: must exceed full_m_per_px".into());
    }
    range(
        "presentation.overlay.stroke.thin_scale",
        stroke.thin_scale,
        f64::MIN_POSITIVE,
        1.0,
    )?;
    positive("overlay.stroke.min_px", stroke.min_px)?;
    positive("overlay.stroke.min_m", stroke.min_m)?;
    range(
        "presentation.overlay.xray.min_hidden_fragment_fraction",
        overlay.xray.min_hidden_fragment_fraction,
        0.0,
        1.0,
    )?;
    positive("overlay.orders.flash.fade_s", overlay.orders.flash.fade_s)?;
    positive("contacts.hatch_width_m", s.contacts.hatch_width_m)?;
    positive("contacts.outline_width_m", s.contacts.outline_width_m)?;
    if s.contacts.hatch_width_m >= s.contacts.hatch_spacing_m {
        return Err(
            "presentation.contacts.hatch_width_m: must be less than hatch_spacing_m".into(),
        );
    }
    positive("contacts.fade_s", s.contacts.fade_s)?;
    positive("contacts.hatch_spacing_m", s.contacts.hatch_spacing_m)?;
    range(
        "presentation.contacts.air.max_radius_px",
        s.contacts.air.max_radius_px,
        s.contacts.air.min_radius_px,
        f64::MAX,
    )?;
    positive("map_border.width_px", s.map_border.width_px)?;
    positive("map_border.min_width_m", s.map_border.min_width_m)?;
    Ok(())
}

pub(crate) fn ground(
    terrain: &crate::static_world::ground::tuning::TerrainTuning,
    grass: &crate::static_world::ground::tuning::GrassTuning,
) -> Result<(), String> {
    numbers(terrain, "presentation.terrain", &[])?;
    numbers(grass, "presentation.grass", &[])?;
    let positive = |p: &str, v| range(&format!("presentation.{p}"), v, f64::MIN_POSITIVE, f64::MAX);
    positive("grass.edge_m", grass.edge_m)?;
    positive("terrain.bank.ease_m", terrain.bank.ease_m)?;
    positive("terrain.scars.depth_cap", terrain.scars.depth_cap)?;
    positive("terrain.water.shore_fade_m", terrain.water.shore_fade_m)?;
    positive(
        "terrain.water.caustics.tile_m",
        terrain.water.caustics.tile_m,
    )?;
    positive(
        "terrain.water.refraction.blur_m",
        terrain.water.refraction.blur_m,
    )?;
    for v in terrain.water.normals.tiles_m {
        positive("terrain.water.normals.tiles_m", v)?;
    }
    Ok(())
}

/// UI arrays are display RGB(A); opacity fields are shares of their source.
fn display(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                range(
                    &format!("{path}[{i}]"),
                    v.as_f64().unwrap_or(f64::NAN),
                    0.0,
                    1.0,
                )?;
            }
        }
        Value::Object(o) => {
            for (key, v) in o {
                let p = format!("{path}.{key}");
                if key.ends_with("_alpha") {
                    range(&p, v.as_f64().unwrap_or(f64::NAN), 0.0, 1.0)?;
                } else {
                    display(v, &p)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
