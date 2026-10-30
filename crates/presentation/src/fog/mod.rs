//! What unseen looks like (`presentation.fog`): named styles, one selected,
//! each resolved into the [`FogLook`] uniform the fog mask passes read. Seen
//! pixels are the real world, untouched; an unseen pixel is dimmed,
//! desaturated, pulled toward a night tint and ruled with fine screen-space
//! lines, and the boundary carries a rim on its seen side. Both clients
//! upload the record as it is.
pub mod sight;
pub mod upload;

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::records::FogLook;

/// The widest rim or soft edge a style may ask for, device pixels. A
/// style's search reaches one pixel past it, which the shaders' distance cap
/// (16 pixels in `fog_masks.wgsl`) stays above.
pub const FOG_EDGE_REACH_PX: f64 = 8.0;

use crate::color::luminance;

/// One unseen look, as the fixture writes it.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FogStyle {
    /// Brightness kept, 0–1.
    pub dim: f64,
    /// How far the colour moves to the night tint at its own luminance, 0–1.
    pub cool: f64,
    /// The night tint's hue; rescaled to unit luminance, so it tints
    /// without brightening.
    pub tint: [f64; 3],
    /// Saturation kept before cooling, 0–1.
    pub saturation: f64,
    /// A glow of the night tint over all unseen, at this HDR luminance.
    pub veil: f64,
    pub lines: FogLines,
    /// Device pixels over which the unseen side fades in from the boundary.
    pub edge_softness: f64,
    pub rim: FogRim,
}

/// Screen-space lines ruled across unseen pixels.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FogLines {
    /// Brightness change on a line; 0 off.
    pub strength: f64,
    /// A line's least brightening, in the night tint at this HDR luminance.
    pub floor: f64,
    pub spacing_px: f64,
    pub width_px: f64,
    /// Degrees counter-clockwise from screen horizontal.
    pub angle_deg: f64,
}

/// A line along the boundary, on the seen side, drawn after post.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FogRim {
    pub width_px: f64,
    /// Display sRGB, 0–1.
    pub color: [f64; 3],
    pub alpha: f64,
}

/// `presentation.fog`: named looks and the one drawn.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FogPresentation {
    pub style: String,
    pub styles: BTreeMap<String, FogStyle>,
}

impl FogPresentation {
    /// The selected style, every named one checked.
    pub fn selected(&self) -> Result<&FogStyle, String> {
        if self.styles.is_empty() {
            return Err("presentation.fog.styles must name at least one style".into());
        }
        for (name, style) in &self.styles {
            style.check(&format!("presentation.fog.styles.{name}"))?;
        }
        self.styles.get(&self.style).ok_or_else(|| {
            let names: Vec<&str> = self.styles.keys().map(String::as_str).collect();
            format!(
                "presentation.fog.style \"{}\" is not one of [{}]",
                self.style,
                names.join(", ")
            )
        })
    }
}

impl FogStyle {
    /// Errs naming the first field out of its range, under `path`.
    pub fn check(&self, path: &str) -> Result<(), String> {
        let within = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!(
                    "{path}.{name} must be within [{lo}, {hi}], got {v}"
                ))
            }
        };
        within("dim", self.dim, 0.0, 1.0)?;
        within("cool", self.cool, 0.0, 1.0)?;
        within("saturation", self.saturation, 0.0, 1.0)?;
        within("veil", self.veil, 0.0, 1.0)?;
        for (i, c) in self.tint.iter().enumerate() {
            within(&format!("tint[{i}]"), *c, 0.0, 4.0)?;
        }
        if luminance(self.tint) <= 0.0 {
            return Err(format!("{path}.tint must not be black"));
        }
        let l = &self.lines;
        within("lines.strength", l.strength, -1.0, 4.0)?;
        within("lines.floor", l.floor, 0.0, 1.0)?;
        within("lines.spacing_px", l.spacing_px, 1.0, 256.0)?;
        within("lines.width_px", l.width_px, 0.0, 256.0)?;
        within("lines.angle_deg", l.angle_deg, -360.0, 360.0)?;
        within("edge_softness", self.edge_softness, 0.0, FOG_EDGE_REACH_PX)?;
        within("rim.width_px", self.rim.width_px, 0.0, FOG_EDGE_REACH_PX)?;
        within("rim.alpha", self.rim.alpha, 0.0, 1.0)?;
        for (i, c) in self.rim.color.iter().enumerate() {
            within(&format!("rim.color[{i}]"), *c, 0.0, 1.0)?;
        }
        Ok(())
    }

    /// The uniform, after checking the style.
    pub fn look(&self) -> Result<FogLook, String> {
        self.check("fog style")?;
        let luma = luminance(self.tint);
        let l = &self.lines;
        Ok(FogLook {
            tint: self.tint.map(|c| (c / luma) as f32),
            dim: self.dim as f32,
            cool: self.cool as f32,
            saturation: self.saturation as f32,
            veil: self.veil as f32,
            line_strength: l.strength as f32,
            line_spacing_px: l.spacing_px as f32,
            line_width_px: l.width_px as f32,
            line_angle: l.angle_deg.to_radians() as f32,
            line_floor: l.floor as f32,
            edge_softness_px: self.edge_softness as f32,
            pad0: [0.0; 3],
            rim_color: self.rim.color.map(|c| c as f32),
            rim_alpha: self.rim.alpha as f32,
            rim_width_px: self.rim.width_px as f32,
            reach_px: (self.rim.width_px.max(self.edge_softness).ceil() + 1.0) as f32,
            pad1: [0.0; 2],
        })
    }
}

impl FogLook {
    /// The uniform's words: what the browser's binding hands the page.
    pub fn words(&self) -> Vec<f32> {
        bytemuck::cast_slice(&[*self]).to_vec()
    }

    /// Whether the look draws an edge (a soft side or a rim), which the
    /// distance passes are needed for.
    pub fn has_edge(&self) -> bool {
        self.edge_softness_px > 0.0 || self.has_rim()
    }

    /// Whether the look draws a rim after post.
    pub fn has_rim(&self) -> bool {
        self.rim_width_px > 0.0 && self.rim_alpha > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> FogPresentation {
        let game: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/presentation.json"
        )))
        .unwrap();
        serde_json::from_value(game["fog"].clone()).unwrap()
    }

    fn base() -> FogStyle {
        fixture().selected().unwrap().clone()
    }

    #[test]
    fn the_fixture_selects_a_valid_style_and_refuses_an_unknown_name() {
        assert!(fixture().selected().is_ok());
        let mut fog = fixture();
        fog.style = "no-such-style".into();
        assert!(fog.selected().unwrap_err().contains("style"));
    }

    #[test]
    fn a_tint_changes_hue_never_brightness() {
        for tint in [[0.5, 0.7, 1.4], [3.0, 3.0, 3.0], [0.1, 0.1, 2.0]] {
            let look = FogStyle { tint, ..base() }.look().unwrap();
            let t = look.tint.map(f64::from);
            assert!((luminance(t) - 1.0).abs() < 1e-6, "{tint:?}: {t:?}");
            // The same hue: the channels keep their ratios.
            assert!(
                (t[2] / t[0] - tint[2] / tint[0]).abs() < 1e-5,
                "{tint:?}: {t:?}"
            );
        }
    }

    #[test]
    fn a_style_out_of_range_is_refused_by_name() {
        let s = base();
        let refuse = |style: FogStyle, name: &str| {
            let error = style.look().unwrap_err();
            assert!(error.contains(name), "{error}");
        };
        refuse(
            FogStyle {
                dim: 1.5,
                ..s.clone()
            },
            "dim",
        );
        refuse(
            FogStyle {
                tint: [0.0; 3],
                ..s.clone()
            },
            "tint",
        );
        refuse(
            FogStyle {
                veil: -0.1,
                ..s.clone()
            },
            "veil",
        );
        let mut lines = s.clone();
        lines.lines.spacing_px = 0.0;
        refuse(lines, "lines.spacing_px");
        refuse(
            FogStyle {
                edge_softness: FOG_EDGE_REACH_PX + 1.0,
                ..s.clone()
            },
            "edge_softness",
        );
        let mut rim = s.clone();
        rim.rim.width_px = FOG_EDGE_REACH_PX + 1.0;
        refuse(rim, "rim.width_px");
        let mut alpha = s;
        alpha.rim.alpha = 1.5;
        refuse(alpha, "rim.alpha");
    }

    #[test]
    fn the_search_reaches_one_pixel_past_the_widest_edge() {
        let widest = FOG_EDGE_REACH_PX;
        for (rim, soft) in [
            (0.0, 0.0),
            (1.5, 0.0),
            (0.0, 3.2),
            (2.0, 6.0),
            (widest, widest),
        ] {
            let mut s = base();
            s.rim.width_px = rim;
            s.edge_softness = soft;
            let reach = f64::from(s.look().unwrap().reach_px);
            // A rim pixel lies within width + 1 pixel centres of the unseen
            // side; a softened unseen pixel within softness + 1 of the seen.
            assert!(reach >= rim.max(soft).ceil() + 1.0, "{rim} {soft}: {reach}");
            assert!(reach <= FOG_EDGE_REACH_PX + 1.0, "{rim} {soft}: {reach}");
        }
    }

    #[test]
    fn a_look_with_no_rim_and_a_hard_edge_needs_no_distances() {
        let mut s = base();
        s.edge_softness = 0.0;
        s.rim.alpha = 0.0;
        let look = s.look().unwrap();
        assert!(!look.has_edge() && !look.has_rim());
        assert!(base().look().unwrap().has_edge());
    }
}
