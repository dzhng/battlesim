//! The biome's paved and worn kinds: each road look (`Road`), its
//! shoulder, ruts, walk, kerb, markings and slabs, and their checks.
use super::{bad, within, Biome, Range, AREA_KINDS};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// One kind of paving's surface, drawn exactly where the simulation's rule
/// holds.
#[derive(Clone, Deserialize)]
pub struct Road {
    /// Two colours: the surface, and what its patches wear toward (a change
    /// of hue, never of brightness).
    pub palette: String,
    /// Width of the blend across the road's edge, centred on it.
    pub feather_m: f64,
    /// How far its edge wanders either side of the stroke's: its surface
    /// spills onto the shoulder and is bitten back by it, never a ruled line.
    pub ragged_m: f64,
    /// How far a patch goes to the second colour, and a patch's size.
    pub mottle: f64,
    pub patch_m: f64,
    /// Stones and clods: their strength and the largest lumps' size.
    pub grain: f64,
    pub grain_m: f64,
    /// Where this kind runs under a road drawn over it, how far its surface
    /// is carried onto that road; 0 ends it at that road's edge.
    pub join_m: f64,
    pub roughness: f64,
    pub shoulder: Shoulder,
    pub ruts: Ruts,
    pub centre_strip: CentreStrip,
    /// Where two kinds overlap, the higher layer is drawn on top; left out,
    /// the simulation's own order (4 road, 3 country road, 2 dirt track,
    /// 1 paving).
    pub layer: Option<f64>,
    /// A road through a town, drawn by another kind's row (`Town`).
    pub town: Option<Town>,
    pub walk: Option<Walk>,
    pub curb: Option<Curb>,
    pub markings: Option<Markings>,
    /// The joints of the slabs it is laid in as an area, square to its
    /// outline's dominant bearing.
    pub slabs: Option<Slabs>,
    /// A region's own finish of this row: per regional family, the fields
    /// that differ (`Biome::load`).
    #[serde(default)]
    pub families: BTreeMap<String, Value>,
}

/// Where a settlement's own ground lies within `beside_m` of a stroke's
/// edge on both sides, the stroke is drawn by `kind`'s row, and on across
/// any gap in that ground shorter than `gap_m`.
#[derive(Clone, Deserialize)]
pub struct Town {
    pub kind: String,
    pub beside_m: f64,
    pub gap_m: f64,
}

/// The worn ground beside a road: its colour (at the ground's luminance
/// where that is brighter), how far the ground goes to it, its widest width
/// from the road's edge, the share and scale its outer edge wanders in by,
/// and the share of grass that still grows at the road's edge.
#[derive(Clone, Deserialize)]
pub struct Shoulder {
    pub palette: String,
    pub cover: f64,
    pub width_m: f64,
    pub jitter: f64,
    pub jitter_m: f64,
    pub grass: f64,
}

/// Wheel ruts, shading only: each rut's distance from the centreline
/// (mirrored, at most two), its width, the steepest tilt of its sides
/// (degrees) and how far it darkens the surface at its middle.
#[derive(Clone, Deserialize)]
pub struct Ruts {
    pub offsets_m: Vec<f64>,
    pub width_m: f64,
    pub tilt_deg: f64,
    pub tint: f64,
}

/// The grass strip down a narrow track: half its width (0 for none), and
/// the widest stroke that has one.
#[derive(Clone, Deserialize)]
pub struct CentreStrip {
    pub half_width_m: f64,
    pub max_road_width_m: f64,
}

/// The paved walk along a street: the kind whose row draws it, its width,
/// its slabs' length and how far their joint darkens (0 for none).
#[derive(Clone, Deserialize)]
pub struct Walk {
    pub kind: String,
    pub width_m: f64,
    pub slab_m: f64,
    pub joint: f64,
}

/// A curb, shading only: the stones' colour, width, length and joint, and
/// the step's face: how far inside the edge it starts and its tilt
/// (degrees, at most 40).
#[derive(Clone, Deserialize)]
pub struct Curb {
    pub palette: String,
    pub width_m: f64,
    pub stone_m: f64,
    pub joint: f64,
    pub face_m: f64,
    pub tilt_deg: f64,
}

/// A road's painted lines: the paint, how much it hides whole and the share
/// it loses worn, the centre line's width, a dash and its gap, and the
/// crossings' bars.
#[derive(Clone, Deserialize)]
pub struct Markings {
    pub palette: String,
    pub cover: f64,
    pub wear: f64,
    pub line_m: f64,
    pub dash_m: Range,
    pub crossing: Crossing,
}

/// A crossing's bars: a bar's width (and the gap between two), its length
/// along the road, how far from the crossing road's edge they start, and
/// how far they keep from their own road's edge.
#[derive(Clone, Deserialize)]
pub struct Crossing {
    pub bar_m: f64,
    pub length_m: f64,
    pub gap_m: f64,
    pub inset_m: f64,
}

/// An area's slabs: a slab's side, and how far a joint darkens the surface.
#[derive(Clone, Deserialize)]
pub struct Slabs {
    pub slab_m: f64,
    pub joint: f64,
}

impl Biome {
    pub(super) fn check_road(&self, at: &str, kind: &str, road: &Road) -> Result<(), String> {
        let at = |f: &str| format!("{at}.{f}");
        if kind != "default" && !AREA_KINDS.contains(&kind) {
            bad(&at(""), "names no paved kind")?;
        }
        let paved = |path: &str, row: &str| match AREA_KINDS.contains(&row) {
            true => Ok(()),
            false => bad(path, format!("names no paved kind \"{row}\"")),
        };
        self.has_palette(&at("palette"), &road.palette, 2)?;
        within(&at("feather_m"), road.feather_m, 0.0, 5.0)?;
        within(
            &at("ragged_m"),
            road.ragged_m,
            0.0,
            road.shoulder.width_m.max(0.0),
        )?;
        within(&at("mottle"), road.mottle, 0.0, 1.0)?;
        within(&at("patch_m"), road.patch_m, 0.1, 1000.0)?;
        within(&at("grain"), road.grain, 0.0, 1.0)?;
        within(&at("grain_m"), road.grain_m, 0.01, 100.0)?;
        within(&at("join_m"), road.join_m, 0.0, 20.0)?;
        within(&at("roughness"), road.roughness, 0.0, 1.0)?;
        let s = &road.shoulder;
        self.has_palette(&at("shoulder.palette"), &s.palette, 1)?;
        within(&at("shoulder.cover"), s.cover, 0.0, 1.0)?;
        within(&at("shoulder.width_m"), s.width_m, 0.0, 8.0)?;
        within(&at("shoulder.jitter"), s.jitter, 0.0, 1.0)?;
        within(&at("shoulder.jitter_m"), s.jitter_m, 0.1, 1000.0)?;
        within(&at("shoulder.grass"), s.grass, 0.0, 1.0)?;
        let ruts = &road.ruts;
        if ruts.offsets_m.len() > 2 {
            bad(&at("ruts.offsets_m"), "must list at most two distances")?;
        }
        for (i, &o) in ruts.offsets_m.iter().enumerate() {
            within(&at(&format!("ruts.offsets_m[{i}]")), o, 0.05, 50.0)?;
        }
        within(&at("ruts.width_m"), ruts.width_m, 0.0, 3.0)?;
        within(&at("ruts.tilt_deg"), ruts.tilt_deg, 0.0, 15.0)?;
        within(&at("ruts.tint"), ruts.tint, 0.0, 0.5)?;
        within(
            &at("centre_strip.half_width_m"),
            road.centre_strip.half_width_m,
            0.0,
            5.0,
        )?;
        within(
            &at("centre_strip.max_road_width_m"),
            road.centre_strip.max_road_width_m,
            0.0,
            100.0,
        )?;
        if let Some(layer) = road.layer {
            within(&at("layer"), layer, 0.0, 100.0)?;
        }
        if let Some(t) = &road.town {
            paved(&at("town.kind"), &t.kind)?;
            within(&at("town.beside_m"), t.beside_m, 0.0, 100.0)?;
            within(&at("town.gap_m"), t.gap_m, 0.0, 10000.0)?;
        }
        if let Some(w) = &road.walk {
            paved(&at("walk.kind"), &w.kind)?;
            within(&at("walk.width_m"), w.width_m, 0.0, 8.0)?;
            within(&at("walk.slab_m"), w.slab_m, 0.0, 100.0)?;
            within(&at("walk.joint"), w.joint, 0.0, 0.5)?;
        }
        if let Some(c) = &road.curb {
            self.has_palette(&at("curb.palette"), &c.palette, 1)?;
            within(&at("curb.width_m"), c.width_m, 0.05, 2.0)?;
            within(&at("curb.stone_m"), c.stone_m, 0.1, 100.0)?;
            within(&at("curb.joint"), c.joint, 0.0, 0.5)?;
            within(&at("curb.face_m"), c.face_m, 0.02, 2.0)?;
            within(&at("curb.tilt_deg"), c.tilt_deg, 0.0, 40.0)?;
        }
        if let Some(s) = &road.slabs {
            within(&at("slabs.slab_m"), s.slab_m, 0.2, 20.0)?;
            within(&at("slabs.joint"), s.joint, 0.0, 0.5)?;
        }
        if let Some(m) = &road.markings {
            self.has_palette(&at("markings.palette"), &m.palette, 1)?;
            within(&at("markings.cover"), m.cover, 0.0, 1.0)?;
            within(&at("markings.wear"), m.wear, 0.0, 1.0)?;
            within(&at("markings.line_m"), m.line_m, 0.05, 1.0)?;
            within(&at("markings.dash_m[0]"), m.dash_m[0], 0.5, 50.0)?;
            within(&at("markings.dash_m[1]"), m.dash_m[1], 0.0, 50.0)?;
            let c = &m.crossing;
            within(&at("markings.crossing.bar_m"), c.bar_m, 0.1, 3.0)?;
            within(&at("markings.crossing.length_m"), c.length_m, 0.0, 8.0)?;
            within(&at("markings.crossing.gap_m"), c.gap_m, 0.0, 8.0)?;
            within(&at("markings.crossing.inset_m"), c.inset_m, 0.0, 5.0)?;
        }
        Ok(())
    }
}
