//! UI-owned settings, admitted with the rest of the presentation document.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraDefault {
    pub distance: f64,
    pub yaw: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraClearance {
    pub margin_m: f64,
    pub release_m: f64,
    pub lift_max_m: f64,
    pub lift_steps: f64,
    pub slide_max_m: f64,
    pub slide_max_rad: f64,
    pub slide_steps: f64,
    pub refine_steps: f64,
    pub smoothing_s: f64,
    pub lookahead_s: f64,
    pub sweep_step_m: f64,
    pub sweep_max_steps: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub fov_y: f64,
    pub near_m: f64,
    pub zoom_min: f64,
    pub zoom_max: f64,
    pub pitch_curve: Vec<[f64; 2]>,
    pub pan_speed: f64,
    pub rotate_speed: f64,
    pub zoom_speed: f64,
    pub orbit_speed: f64,
    pub r#default: CameraDefault,
    pub clearance: CameraClearance,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayGlow {
    pub radius_px: f64,
    pub overlay: f64,
    pub ground: f64,
    pub callouts: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayPaint {
    pub albedo: f64,
    pub fog_keep: f64,
    pub grass_glow: f64,
    pub grass_falloff_m: f64,
    pub saturation: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayStroke {
    pub full_m_per_px: f64,
    pub thin_m_per_px: f64,
    pub thin_scale: f64,
    pub min_px: f64,
    pub min_m: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayXray {
    pub own: [f64; 4],
    pub selected_alpha: f64,
    pub min_hidden_fragment_fraction: f64,
    pub ally_alpha: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayOrdersGlow {
    pub order: f64,
    pub selected: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayOrdersCover {
    pub light: [f64; 4],
    pub medium: [f64; 4],
    pub heavy: [f64; 4],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayOrdersMarch {
    pub cycles_per_s: f64,
    pub amplitude: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayOrdersFlash {
    pub hold_s: f64,
    pub fade_s: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayOrders {
    pub line_px: f64,
    pub mark_px: f64,
    pub soldier_mark_px: f64,
    pub soldier_line_px: f64,
    pub color: [f64; 4],
    pub selected: [f64; 4],
    pub queued_alpha: f64,
    pub current_alpha: f64,
    pub glow: OverlayOrdersGlow,
    pub cover_glow: f64,
    pub cover: OverlayOrdersCover,
    pub cover_pip_m: f64,
    pub march: OverlayOrdersMarch,
    pub area_draw_scale: f64,
    pub vehicle_marker_margin_m: f64,
    pub building_marker_margin_m: f64,
    pub drop_line_px: f64,
    pub drop_line_spacing: f64,
    pub drop_line_alpha: f64,
    pub drop_line_hold: f64,
    pub drop_line_reach: f64,
    pub flash: OverlayOrdersFlash,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlaySupply {
    pub reach: [f64; 4],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayRuler {
    pub line_px: f64,
    pub tick_px: f64,
    pub tick_line_px: f64,
    pub end_px: f64,
    pub tick: [f64; 4],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Overlay {
    pub glow: OverlayGlow,
    pub paint: OverlayPaint,
    pub stroke: OverlayStroke,
    pub xray: OverlayXray,
    pub orders: OverlayOrders,
    pub supply: OverlaySupply,
    pub zone: [f64; 4],
    pub ruler: OverlayRuler,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hud {
    pub font: String,
    pub accent: [f64; 3],
    pub ally: [f64; 3],
    pub enemy: [f64; 3],
    pub text: [f64; 3],
    pub dim: [f64; 3],
    pub warn: [f64; 3],
    pub pinned: [f64; 3],
    pub good: [f64; 3],
    pub bad: [f64; 3],
    pub credits: [f64; 3],
    pub film: [f64; 3],
    pub glass: [f64; 4],
    pub glow_px: f64,
    pub panel_edge_hide_px: f64,
    pub panel_gap_px: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContactsAir {
    pub fill_alpha: f64,
    pub hatch_alpha: f64,
    pub glow_alpha: f64,
    pub hatch_px: f64,
    pub rim_px: f64,
    pub keyline_px: f64,
    pub keyline_alpha: f64,
    pub min_radius_px: f64,
    pub max_radius_px: f64,
    pub stem_px: f64,
    pub stem_alpha: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contacts {
    pub fade_s: f64,
    pub hatch_spacing_m: f64,
    pub hatch_width_m: f64,
    pub hatch_angle_deg: f64,
    pub outline_width_m: f64,
    pub outline_color: [f64; 3],
    pub hatch_alpha: f64,
    pub glow_alpha: f64,
    pub fill_alpha: f64,
    pub lift_m: f64,
    pub air: ContactsAir,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapBorder {
    pub color: [f64; 4],
    pub width_px: f64,
    pub min_width_m: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ground {
    pub backdrop_reach_m: f64,
}

impl Overlay {
    pub(super) fn check(&self) -> Result<(), String> {
        use super::validate::range;
        let bounded =
            |key: &str, value, lo, hi| range(&format!("presentation.overlay.{key}"), value, lo, hi);
        let positive = |key: &str, value| bounded(key, value, f64::MIN_POSITIVE, f64::MAX);
        for (key, value, lo, hi) in [
            ("paint.albedo", self.paint.albedo, f64::MIN_POSITIVE, 1.0),
            ("paint.fog_keep", self.paint.fog_keep, 0.0, 1.0),
            ("paint.grass_glow", self.paint.grass_glow, 0.0, 8.0),
            (
                "paint.grass_falloff_m",
                self.paint.grass_falloff_m,
                f64::MIN_POSITIVE,
                4.0,
            ),
            ("paint.saturation", self.paint.saturation, 0.0, 3.0),
            (
                "orders.queued_alpha",
                self.orders.queued_alpha,
                f64::MIN_POSITIVE,
                1.0,
            ),
            (
                "orders.current_alpha",
                self.orders.current_alpha,
                f64::MIN_POSITIVE,
                1.0,
            ),
            (
                "orders.march.amplitude",
                self.orders.march.amplitude,
                0.0,
                1.0,
            ),
            (
                "orders.area_draw_scale",
                self.orders.area_draw_scale,
                f64::MIN_POSITIVE,
                1.0,
            ),
            (
                "orders.glow.order",
                self.orders.glow.order,
                f64::MIN_POSITIVE,
                2.0,
            ),
            (
                "orders.glow.selected",
                self.orders.glow.selected,
                f64::MIN_POSITIVE,
                2.0,
            ),
            ("orders.cover_glow", self.orders.cover_glow, 1.0, 2.0),
            (
                "orders.drop_line_alpha",
                self.orders.drop_line_alpha,
                f64::MIN_POSITIVE,
                1.0,
            ),
            (
                "orders.drop_line_reach",
                self.orders.drop_line_reach,
                f64::MIN_POSITIVE,
                1.0,
            ),
            ("xray.own[3]", self.xray.own[3], f64::MIN_POSITIVE, 1.0),
            (
                "xray.selected_alpha",
                self.xray.selected_alpha,
                f64::MIN_POSITIVE,
                1.0,
            ),
            (
                "xray.ally_alpha",
                self.xray.ally_alpha,
                f64::MIN_POSITIVE,
                1.0,
            ),
        ] {
            bounded(key, value, lo, hi)?;
        }
        for (key, value) in [
            ("orders.line_px", self.orders.line_px),
            ("orders.mark_px", self.orders.mark_px),
            ("orders.soldier_mark_px", self.orders.soldier_mark_px),
            ("orders.soldier_line_px", self.orders.soldier_line_px),
            ("orders.cover_pip_m", self.orders.cover_pip_m),
            (
                "orders.vehicle_marker_margin_m",
                self.orders.vehicle_marker_margin_m,
            ),
            (
                "orders.building_marker_margin_m",
                self.orders.building_marker_margin_m,
            ),
            ("orders.drop_line_px", self.orders.drop_line_px),
            ("ruler.line_px", self.ruler.line_px),
            ("ruler.tick_px", self.ruler.tick_px),
            ("ruler.tick_line_px", self.ruler.tick_line_px),
            ("ruler.end_px", self.ruler.end_px),
        ] {
            positive(key, value)?;
        }
        if self.orders.drop_line_spacing <= 1.0 {
            return Err("presentation.overlay.orders.drop_line_spacing: must exceed 1".into());
        }
        if self.orders.drop_line_hold >= self.orders.drop_line_reach {
            return Err(
                "presentation.overlay.orders.drop_line_hold: must be below drop_line_reach".into(),
            );
        }
        Ok(())
    }
}
