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
