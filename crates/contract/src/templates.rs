//! Physical building templates, independent of appearance and simulation state.
//! XY is ground, +Z up; parts use the existing oriented-box primitive. Placement
//! translates and rotates that frame without scaling. Unknown source facts stay
//! absent; consumers needing them call `require_complete` before selection.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::numbers;

/// Cumulative per-descriptor allocation allowance: at most 1 MiB of f64 XY
/// bay coordinates. This bounds work; it does not choose source dimensions.
pub const MAX_TEMPLATE_BAYS: usize = 65_536;
// Consecutive lattice indices must remain distinct exact f64 integers.
const MAX_EXACT_BAY_INDEX: f64 = ((1u64 << 53) - 1) as f64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingCategory {
    Farmstead,
    DetachedHome,
    AttachedHome,
    UrbanApartment,
    Highrise,
    Industry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingTemplateDescriptor {
    pub id: String,
    pub category: BuildingCategory,
    pub regional_family: String,
    pub parts: Vec<TemplatePart>,
    /// Nonnegative local floor datums. None means the source has no
    /// authoritative floor data; it does not mean a one-floor building.
    #[serde(default, deserialize_with = "numbers::optional_list")]
    pub floor_heights_m: Option<Vec<f64>>,
    pub entrances: Option<Vec<Entrance>>,
    pub edges: Vec<FacadeEdge>,
    pub joins: Vec<SupportedJoin>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplatePart {
    pub id: String,
    #[serde(deserialize_with = "numbers::array")]
    pub center: [f64; 2],
    #[serde(deserialize_with = "numbers::scalar")]
    pub yaw: f64,
    #[serde(deserialize_with = "numbers::array")]
    pub half_extents: [f64; 3],
    #[serde(deserialize_with = "numbers::scalar")]
    pub base_z: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entrance {
    pub id: String,
    pub edge: String,
    #[serde(deserialize_with = "numbers::scalar")]
    pub offset_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facade {
    PositiveX,
    PositiveY,
    NegativeX,
    NegativeY,
}

impl Facade {
    pub const ALL: [Self; 4] = [
        Self::PositiveX,
        Self::PositiveY,
        Self::NegativeX,
        Self::NegativeY,
    ];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeEdge {
    pub id: String,
    pub part: String,
    /// The part-local box face; simulation directional eye groups must use
    /// the materialized normal relative to the building frame.
    pub facade: Facade,
    /// Along-facade metres relative to the part centre, in the current
    /// simulation's +Y, -X, -Y, +X order for +X, +Y, -X, -Y facades.
    #[serde(deserialize_with = "numbers::array")]
    pub span_m: [f64; 2],
    pub exposed: bool,
    pub bays: Option<FacadeBays>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeBays {
    #[serde(deserialize_with = "numbers::scalar")]
    pub pitch_m: f64,
    #[serde(deserialize_with = "numbers::scalar")]
    pub phase_m: f64,
}

struct BayRange {
    first: f64,
    count: usize,
}

impl FacadeBays {
    fn range(&self, span: [f64; 2]) -> Result<BayRange, &'static str> {
        let first = ((span[0] - self.phase_m) / self.pitch_m).floor() + 1.0;
        let last = ((span[1] - self.phase_m) / self.pitch_m).ceil() - 1.0;
        let count = (last - first + 1.0).max(0.0);
        if !first.is_finite()
            || !last.is_finite()
            || !count.is_finite()
            || count > MAX_TEMPLATE_BAYS as f64
        {
            return Err("bay lattice exceeds the descriptor allocation bound");
        }
        if first.abs() > MAX_EXACT_BAY_INDEX || last.abs() > MAX_EXACT_BAY_INDEX {
            return Err("bay lattice indices cannot remain distinct exact integers");
        }
        let count = count as usize;
        let mut previous = span[0];
        for i in 0..count {
            let offset = self.phase_m + (first + i as f64) * self.pitch_m;
            if !offset.is_finite() || offset <= previous || offset >= span[1] {
                return Err("bay lattice cannot represent distinct strictly interior points");
            }
            previous = offset;
        }
        Ok(BayRange { first, count })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedJoin {
    pub id: String,
    pub edges: [String; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementFrame {
    #[serde(deserialize_with = "numbers::array")]
    pub translation: [f64; 3],
    #[serde(deserialize_with = "numbers::scalar")]
    pub yaw: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedPart {
    pub id: String,
    #[serde(deserialize_with = "numbers::array")]
    pub center: [f64; 2],
    #[serde(deserialize_with = "numbers::scalar")]
    pub yaw: f64,
    #[serde(deserialize_with = "numbers::array")]
    pub half_extents: [f64; 3],
    #[serde(deserialize_with = "numbers::scalar")]
    pub base_z: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedEntrance {
    pub id: String,
    #[serde(deserialize_with = "numbers::array")]
    pub position: [f64; 3],
    #[serde(deserialize_with = "numbers::array")]
    pub normal: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedEdge {
    pub id: String,
    pub part: String,
    /// The part-local box face; simulation directional eye groups must use
    /// the materialized normal relative to the building frame.
    pub facade: Facade,
    /// Authoritative part-local interval used by physical consumers.
    #[serde(deserialize_with = "numbers::array")]
    pub span_m: [f64; 2],
    #[serde(deserialize_with = "numbers::span")]
    pub span: [[f64; 2]; 2],
    #[serde(deserialize_with = "numbers::array")]
    pub normal: [f64; 2],
    #[serde(deserialize_with = "numbers::scalar")]
    pub base_z: f64,
    #[serde(deserialize_with = "numbers::scalar")]
    pub top_z: f64,
    pub exposed: bool,
    #[serde(default, deserialize_with = "numbers::optional_points")]
    pub bays: Option<Vec<[f64; 2]>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedBuilding {
    pub template_id: String,
    pub frame: PlacementFrame,
    pub parts: Vec<MaterializedPart>,
    #[serde(deserialize_with = "numbers::scalar")]
    pub height_m: f64,
    #[serde(default, deserialize_with = "numbers::optional_list")]
    pub floor_z: Option<Vec<f64>>,
    pub entrances: Option<Vec<MaterializedEntrance>>,
    pub edges: Vec<MaterializedEdge>,
}

/// Internal placed-geometry admission. This verifies the final record itself;
/// checking its source catalogue identity belongs to the preparation boundary.
impl MaterializedBuilding {
    pub fn validate(&self) -> Result<(), String> {
        let bad = |message: &str| Err(format!("{}: {message}", self.template_id));
        let mut names = BTreeSet::new();
        if self.template_id.is_empty()
            || self.parts.is_empty()
            || !self.frame.yaw.is_finite()
            || self.frame.translation.iter().any(|v| !v.is_finite())
            || !self.height_m.is_finite()
            || self.height_m < 0.0
        {
            return bad("placed building requires finite physical geometry");
        }
        for part in &self.parts {
            if part.id.is_empty()
                || !names.insert(&part.id)
                || part.center.iter().any(|v| !v.is_finite())
                || !part.yaw.is_finite()
                || !part.base_z.is_finite()
                || part
                    .half_extents
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.0)
                || !(part.base_z + 2.0 * part.half_extents[2]).is_finite()
            {
                return bad("placed part frames and names must be finite and unique");
            }
        }
        let top = self
            .parts
            .iter()
            .map(|p| p.base_z + 2.0 * p.half_extents[2])
            .fold(self.frame.translation[2], f64::max);
        if !coordinate_equal(
            top,
            self.frame.translation[2] + self.height_m,
            self.height_m,
        ) {
            return bad("placed height contradicts physical parts");
        }
        if self.floor_z.as_ref().is_some_and(|floors| {
            floors.is_empty()
                || floors
                    .iter()
                    .any(|z| !z.is_finite() || *z < self.frame.translation[2] || *z >= top)
                || floors.windows(2).any(|p| p[0] >= p[1])
        }) {
            return bad("placed floors must rise below the physical top");
        }
        let mut edges = BTreeSet::new();
        let mut bay_count = 0usize;
        for edge in &self.edges {
            let Some(part) = self.parts.iter().find(|p| p.id == edge.part) else {
                return bad("unknown placed edge part");
            };
            let (normal, along, reach, half) = edge.facade.axes(part.half_extents);
            if edge.id.is_empty()
                || !edges.insert(&edge.id)
                || edge.span_m.iter().any(|v| !v.is_finite())
                || edge.span_m[0] >= edge.span_m[1]
                || edge.span_m[0] < -half
                || edge.span_m[1] > half
            {
                return bad("placed edge interval must lie on its physical part");
            }
            let rotation = libm::sincos(part.yaw);
            let expected_normal = rotate(normal, rotation);
            let expected_span = edge.span_m.map(|offset| {
                let local = [
                    normal[0] * reach + along[0] * offset,
                    normal[1] * reach + along[1] * offset,
                ];
                let point = rotate(local, rotation);
                [part.center[0] + point[0], part.center[1] + point[1]]
            });
            if (0..2).any(|i| {
                !coordinate_equal(edge.normal[i], expected_normal[i], 1.0)
                    || (0..2).any(|j| {
                        !coordinate_equal(edge.span[j][i], expected_span[j][i], reach.max(half))
                    })
            }) || !coordinate_equal(edge.base_z, part.base_z, 2.0 * part.half_extents[2])
                || !coordinate_equal(
                    edge.top_z,
                    part.base_z + 2.0 * part.half_extents[2],
                    2.0 * part.half_extents[2],
                )
            {
                return bad("placed local and world facade geometry disagree");
            }
            let bays = edge.bays.as_deref().unwrap_or(&[]);
            bay_count = bay_count
                .checked_add(bays.len())
                .ok_or("placed bay count overflow")?;
            if bay_count > MAX_TEMPLATE_BAYS || (!edge.exposed && edge.bays.is_some()) {
                return bad("placed bays exceed admission or occupy an internal edge");
            }
            validate_bay_points(edge.span, bays.iter().copied())
                .map_err(|e| format!("{}: {e}", self.template_id))?;
            for point in bays {
                let delta = [
                    edge.span[1][0] - edge.span[0][0],
                    edge.span[1][1] - edge.span[0][1],
                ];
                let relative = [point[0] - edge.span[0][0], point[1] - edge.span[0][1]];
                let t = (relative[0] * delta[0] + relative[1] * delta[1])
                    / (delta[0] * delta[0] + delta[1] * delta[1]);
                if !on_facade(*point, edge.span[0], delta, t, reach.max(half)) {
                    return bad("placed bay is off its facade");
                }
            }
        }
        if let Some(entrances) = &self.entrances {
            let mut ids = BTreeSet::new();
            for entrance in entrances {
                if entrance.id.is_empty()
                    || !ids.insert(&entrance.id)
                    || entrance
                        .position
                        .iter()
                        .chain(&entrance.normal)
                        .any(|v| !v.is_finite())
                {
                    return bad("placed entrances require finite unique records");
                }
                if !self.edges.iter().filter(|e| e.exposed).any(|edge| {
                    let delta = [
                        edge.span[1][0] - edge.span[0][0],
                        edge.span[1][1] - edge.span[0][1],
                    ];
                    let length = libm::hypot(delta[0], delta[1]);
                    let relative = [
                        entrance.position[0] - edge.span[0][0],
                        entrance.position[1] - edge.span[0][1],
                    ];
                    let t = (relative[0] * delta[0] + relative[1] * delta[1]) / (length * length);
                    let [x, y, z] = entrance.position;
                    t > 0.0
                        && t < 1.0
                        && on_facade([x, y], edge.span[0], delta, t, length)
                        && (0..2).all(|i| coordinate_equal(entrance.normal[i], edge.normal[i], 1.0))
                        && coordinate_equal(z, edge.base_z, self.height_m)
                }) {
                    return bad("placed entrance must lie inside an exposed facade");
                }
            }
        }
        Ok(())
    }
}

// Different valid transform operation orders may round at a world datum.
// Admit four coordinate ULPs plus local rotation roundoff, not a tolerance
// proportional to the absolute origin. Source catalogue joins remain stricter.
fn coordinate_equal(a: f64, b: f64, local_scale: f64) -> bool {
    equal_at(a, b, a.abs().max(b.abs()), local_scale)
}

/// `a` and `b` agree to the rounding of a world coordinate as large as `datum`.
fn equal_at(a: f64, b: f64, datum: f64, local_scale: f64) -> bool {
    if !a.is_finite() || !b.is_finite() || !datum.is_finite() {
        return false;
    }
    let next = f64::from_bits(datum.to_bits() + 1);
    let ulp = if next.is_finite() {
        next - datum
    } else {
        datum - f64::from_bits(datum.to_bits() - 1)
    };
    (a - b).abs() <= 4.0 * ulp + 64.0 * f64::EPSILON * local_scale.max(1.0)
}

/// Whether `point` is the point `t` of the way along a facade from `origin`.
/// Projecting onto a turned facade mixes the two world coordinates, so each
/// carries the rounding of the larger: a building far along X and near the
/// origin in Y is still on its own walls.
fn on_facade(point: [f64; 2], origin: [f64; 2], delta: [f64; 2], t: f64, local_scale: f64) -> bool {
    let datum = point
        .into_iter()
        .chain(origin)
        .map(f64::abs)
        .fold(0.0, f64::max);
    (0..2).all(|i| equal_at(point[i], origin[i] + delta[i] * t, datum, local_scale))
}

impl Facade {
    pub fn axes(self, half: [f64; 3]) -> ([f64; 2], [f64; 2], f64, f64) {
        match self {
            Self::PositiveX => ([1.0, 0.0], [0.0, 1.0], half[0], half[1]),
            Self::PositiveY => ([0.0, 1.0], [-1.0, 0.0], half[1], half[0]),
            Self::NegativeX => ([-1.0, 0.0], [0.0, -1.0], half[0], half[1]),
            Self::NegativeY => ([0.0, -1.0], [1.0, 0.0], half[1], half[0]),
        }
    }
}

struct LocalEdge {
    center: [f64; 2],
    offset: [f64; 2],
    origin: [f64; 2],
    normal: [f64; 2],
    along: [f64; 2],
    base_z: f64,
    top_z: f64,
}

impl LocalEdge {
    fn validate_points(
        &self,
        edge: &FacadeEdge,
        transform: impl Fn([f64; 2]) -> [f64; 2],
    ) -> Result<(), &'static str> {
        let range = edge
            .bays
            .map(|pattern| pattern.range(edge.span_m))
            .transpose()?;
        let points = (0..range.as_ref().map_or(0, |r| r.count)).map(|i| {
            let pattern = edge.bays.unwrap();
            let range = range.as_ref().unwrap();
            transform(self.point(pattern.phase_m + (range.first + i as f64) * pattern.pitch_m))
        });
        validate_bay_points(
            edge.span_m.map(|offset| transform(self.point(offset))),
            points,
        )
    }

    fn relative_point(&self, offset: f64) -> [f64; 2] {
        [
            self.offset[0] + self.along[0] * offset,
            self.offset[1] + self.along[1] * offset,
        ]
    }

    fn point(&self, offset: f64) -> [f64; 2] {
        [
            self.origin[0] + self.along[0] * offset,
            self.origin[1] + self.along[1] * offset,
        ]
    }
}

// Canonical geometry and admission must share arithmetic across native and wasm.
// Software libm (without architecture features) avoids host-dependent trig/norms.
fn rotate([x, y]: [f64; 2], (sin, cos): (f64, f64)) -> [f64; 2] {
    [cos * x - sin * y, sin * x + cos * y]
}

/// Check derived geometry with bounded iteration and no point allocation.
fn validate_bay_points(
    span: [[f64; 2]; 2],
    points: impl Iterator<Item = [f64; 2]>,
) -> Result<(), &'static str> {
    let delta = [span[1][0] - span[0][0], span[1][1] - span[0][1]];
    let length = libm::hypot(delta[0], delta[1]);
    if span.into_iter().flatten().any(|v| !v.is_finite()) || !length.is_finite() || length <= 0.0 {
        return Err("facade span cannot represent distinct finite endpoints");
    }
    let along = [delta[0] / length, delta[1] / length];
    let mut previous = 0.0;
    for point in points {
        let distance = (point[0] - span[0][0]) * along[0] + (point[1] - span[0][1]) * along[1];
        if point.iter().any(|v| !v.is_finite())
            || !distance.is_finite()
            || distance <= previous
            || distance >= length
        {
            return Err("bay geometry cannot represent distinct strictly interior points");
        }
        previous = distance;
    }
    Ok(())
}

/// A positive edge interval covered by another part that extends outward from
/// this facade is internal geometry. This validates declared exposure; it never
/// removes facades or invents a source join at runtime.
fn covered(edge: &LocalEdge, span: [f64; 2], part: &TemplatePart) -> bool {
    let top = part.base_z + 2.0 * part.half_extents[2];
    let vertical_scale = (edge.top_z - edge.base_z)
        .abs()
        .max(2.0 * part.half_extents[2])
        .max(1.0);
    let vertical_tolerance = 64.0 * f64::EPSILON * vertical_scale;
    if top - edge.base_z <= vertical_tolerance || part.base_z - edge.top_z >= -vertical_tolerance {
        return false;
    }
    let rotation = libm::sincos(-part.yaw);
    let delta = [
        (part.center[0] - edge.center[0]) - edge.offset[0],
        (part.center[1] - edge.center[1]) - edge.offset[1],
    ];
    let local = rotate([-delta[0], -delta[1]], rotation);
    let along = rotate(edge.along, rotation);
    let normal = rotate(edge.normal, rotation);
    let scale = delta
        .into_iter()
        .chain(part.half_extents)
        .chain(span)
        .map(f64::abs)
        .fold(1.0, f64::max);
    let tolerance = 64.0 * f64::EPSILON * scale;
    let radius = normal[0].abs() * part.half_extents[0] + normal[1].abs() * part.half_extents[1];
    if delta[0] * edge.normal[0] + delta[1] * edge.normal[1] + radius <= tolerance {
        return false;
    }
    let [mut lo, mut hi] = span;
    for i in 0..2 {
        let half = part.half_extents[i];
        if along[i].abs() <= 64.0 * f64::EPSILON {
            if local[i].abs() > half + tolerance {
                return false;
            }
        } else {
            let a = (-half - local[i]) / along[i];
            let b = (half - local[i]) / along[i];
            lo = lo.max(a.min(b));
            hi = hi.min(a.max(b));
        }
    }
    hi - lo > tolerance
}

/// Validated physical data in canonical named-row order. Appearance and source
/// readiness have no field here; they cannot change this catalogue's identity.
#[derive(Clone, Debug, Serialize)]
pub struct TemplateGeometryCatalog {
    hash: String,
    templates: Vec<BuildingTemplateDescriptor>,
}

impl TemplateGeometryCatalog {
    pub fn new(mut templates: Vec<BuildingTemplateDescriptor>) -> Result<Self, String> {
        templates.sort_by(|a, b| a.id.cmp(&b.id));
        for template in &mut templates {
            template.validate()?;
            template.canonicalize();
        }
        if templates.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err("duplicate template id".into());
        }
        let hash = crate::identity::json_hash(&templates).map_err(|e| e.to_string())?;
        Ok(Self { hash, templates })
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn templates(&self) -> &[BuildingTemplateDescriptor] {
        &self.templates
    }

    pub fn canonical_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_json(input: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Input {
            hash: String,
            templates: Vec<BuildingTemplateDescriptor>,
        }
        let input: Input = serde_json::from_str(input).map_err(|e| e.to_string())?;
        let catalogue = Self::new(input.templates)?;
        if catalogue.hash != input.hash {
            return Err("physical catalogue hash does not match its geometry".into());
        }
        Ok(catalogue)
    }
}

impl BuildingTemplateDescriptor {
    /// Author a physical box using the same oriented-box primitive as props.
    /// Source floor/entrance/bay facts remain explicitly unresolved.
    pub fn solid_box(
        id: String,
        category: BuildingCategory,
        regional_family: String,
        half_extents: [f64; 3],
    ) -> Self {
        Self {
            id,
            category,
            regional_family,
            parts: vec![TemplatePart {
                id: "body".into(),
                center: [0.0, 0.0],
                yaw: 0.0,
                half_extents,
                base_z: 0.0,
            }],
            floor_heights_m: None,
            entrances: None,
            joins: vec![],
            edges: Facade::ALL
                .into_iter()
                .enumerate()
                .map(|(i, facade)| {
                    let half = facade.axes(half_extents).3;
                    FacadeEdge {
                        id: format!("face-{i}"),
                        part: "body".into(),
                        facade,
                        span_m: [-half, half],
                        exposed: true,
                        bays: None,
                    }
                })
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let bad = |message: &str| Err(format!("{}: {message}", self.id));
        if self.id.is_empty() || self.regional_family.is_empty() || self.parts.is_empty() {
            return bad("template id, regional family and physical parts are required");
        }
        let unique = |names: Vec<&str>| {
            let mut seen = BTreeSet::new();
            names
                .into_iter()
                .all(|name| !name.is_empty() && seen.insert(name))
        };
        if !unique(self.parts.iter().map(|p| p.id.as_str()).collect())
            || !unique(self.edges.iter().map(|e| e.id.as_str()).collect())
            || !unique(self.joins.iter().map(|j| j.id.as_str()).collect())
        {
            return bad("part, edge and join ids must be nonempty and unique");
        }
        for part in &self.parts {
            if part.center.iter().any(|v| !v.is_finite())
                || !part.yaw.is_finite()
                || !part.base_z.is_finite()
                || part
                    .half_extents
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.0)
            {
                return bad("parts require finite frames and positive extents");
            }
        }
        let height = self.height_m();
        if !height.is_finite() {
            return bad("physical height is not finite");
        }
        if let Some(floors) = &self.floor_heights_m {
            if floors.is_empty()
                || floors
                    .iter()
                    .any(|h| !h.is_finite() || *h < 0.0 || *h >= height)
                || floors.windows(2).any(|p| p[0] >= p[1])
            {
                return bad("floor datums must be nonnegative and rise below the physical top");
            }
            let legal = match self.category {
                BuildingCategory::Farmstead | BuildingCategory::DetachedHome => {
                    (1..=2).contains(&floors.len())
                }
                BuildingCategory::AttachedHome => (2..=3).contains(&floors.len()),
                BuildingCategory::UrbanApartment => (4..=8).contains(&floors.len()),
                BuildingCategory::Highrise => floors.len() >= 9,
                BuildingCategory::Industry => !floors.is_empty(),
            };
            if !legal {
                return bad("floor count does not match the accepted category");
            }
        }
        let mut total_bays = 0;
        for edge in &self.edges {
            let part = self
                .parts
                .iter()
                .find(|p| p.id == edge.part)
                .ok_or_else(|| format!("{}: unknown edge part {}", self.id, edge.part))?;
            let half = match edge.facade {
                Facade::PositiveX | Facade::NegativeX => part.half_extents[1],
                Facade::PositiveY | Facade::NegativeY => part.half_extents[0],
            };
            if edge.span_m.iter().any(|v| !v.is_finite())
                || edge.span_m[0] >= edge.span_m[1]
                || edge.span_m[0] < -half
                || edge.span_m[1] > half
            {
                return bad("edge spans must lie on their part's facade");
            }
            if let Some(bays) = edge.bays {
                if !edge.exposed
                    || !bays.pitch_m.is_finite()
                    || bays.pitch_m <= 0.0
                    || !bays.phase_m.is_finite()
                    || bays.phase_m < 0.0
                    || bays.phase_m >= bays.pitch_m
                {
                    return bad(
                        "bay lattice needs an exposed edge, positive pitch and canonical phase",
                    );
                }
                let count = bays
                    .range(edge.span_m)
                    .map_err(|message| format!("{}: {message}", self.id))?
                    .count;
                if count > MAX_TEMPLATE_BAYS - total_bays {
                    return bad("cumulative bays exceed the descriptor allocation bound");
                }
                total_bays += count;
            }
            let local = self.local_edge(edge)?;
            local
                .validate_points(edge, |p| p)
                .map_err(|message| format!("{}: {message}", self.id))?;
            if edge.exposed
                && self
                    .parts
                    .iter()
                    .any(|p| p.id != edge.part && covered(&local, edge.span_m, p))
            {
                return bad("an exposed facade span is covered by another part");
            }
        }
        let mut joined = BTreeSet::new();
        for join in &self.joins {
            let edge = |id: &str| {
                self.edges
                    .iter()
                    .find(|e| e.id == id)
                    .ok_or_else(|| format!("{}: unknown join edge {id}", self.id))
            };
            let a = edge(&join.edges[0])?;
            let b = edge(&join.edges[1])?;
            if a.exposed
                || b.exposed
                || a.part == b.part
                || !joined.insert(&a.id)
                || !joined.insert(&b.id)
            {
                return bad("supported joins pair distinct internal edges exactly once");
            }
            let la = self.local_edge(a)?;
            let lb = self.local_edge(b)?;
            let [a0, a1] = a.span_m.map(|offset| la.relative_point(offset));
            let [b0, b1] = b.span_m.map(|offset| {
                let point = lb.relative_point(offset);
                [
                    (lb.center[0] - la.center[0]) + point[0],
                    (lb.center[1] - la.center[1]) + point[1],
                ]
            });
            // Only floating rotation roundoff is admitted, not a source-fit gap.
            let scale = [a0, a1, b0, b1]
                .into_iter()
                .flatten()
                .map(f64::abs)
                .fold(1.0, f64::max);
            let tolerance = 64.0 * f64::EPSILON * scale;
            let vertical_scale = (la.top_z - la.base_z)
                .abs()
                .max((lb.top_z - lb.base_z).abs())
                .max(1.0);
            let vertical_tolerance = 64.0 * f64::EPSILON * vertical_scale;
            if (0..2).any(|i| {
                (a0[i] - b1[i]).abs() > tolerance
                    || (a1[i] - b0[i]).abs() > tolerance
                    || (la.normal[i] + lb.normal[i]).abs() > 64.0 * f64::EPSILON
            }) || (la.base_z - lb.base_z).abs() > vertical_tolerance
                || (la.top_z - lb.top_z).abs() > vertical_tolerance
            {
                return bad(
                    "supported join faces must coincide at equal base/top with opposite normals",
                );
            }
        }
        if self
            .edges
            .iter()
            .any(|e| !e.exposed && !joined.contains(&e.id))
        {
            return bad("internal edges need an explicit supported join");
        }
        if let Some(entrances) = &self.entrances {
            if !unique(entrances.iter().map(|e| e.id.as_str()).collect()) {
                return bad("entrance ids must be nonempty and unique");
            }
            for entrance in entrances {
                let edge = self
                    .edges
                    .iter()
                    .find(|e| e.id == entrance.edge)
                    .ok_or_else(|| {
                        format!("{}: unknown entrance edge {}", self.id, entrance.edge)
                    })?;
                if !edge.exposed
                    || !entrance.offset_m.is_finite()
                    || entrance.offset_m <= edge.span_m[0]
                    || entrance.offset_m >= edge.span_m[1]
                {
                    return bad("entrances must be inside an exposed span");
                }
            }
        }
        Ok(())
    }

    fn canonicalize(&mut self) {
        let zero = |value: &mut f64| {
            if *value == 0.0 {
                *value = 0.0;
            }
        };
        self.parts.sort_by(|a, b| a.id.cmp(&b.id));
        for part in &mut self.parts {
            part.center.iter_mut().for_each(zero);
            part.half_extents.iter_mut().for_each(zero);
            zero(&mut part.yaw);
            zero(&mut part.base_z);
        }
        if let Some(floors) = &mut self.floor_heights_m {
            floors.iter_mut().for_each(zero);
        }
        if let Some(entrances) = &mut self.entrances {
            entrances.sort_by(|a, b| a.id.cmp(&b.id));
            for entrance in entrances {
                zero(&mut entrance.offset_m);
            }
        }
        self.edges.sort_by(|a, b| a.id.cmp(&b.id));
        for edge in &mut self.edges {
            edge.span_m.iter_mut().for_each(zero);
            if let Some(bays) = &mut edge.bays {
                zero(&mut bays.pitch_m);
                zero(&mut bays.phase_m);
            }
        }
        self.joins.sort_by(|a, b| a.id.cmp(&b.id));
        for join in &mut self.joins {
            join.edges.sort();
        }
    }
    pub fn height_m(&self) -> f64 {
        self.parts
            .iter()
            .map(|p| p.base_z + 2.0 * p.half_extents[2])
            .fold(0.0, f64::max)
    }

    pub fn require_complete(&self) -> Result<(), String> {
        self.validate()?;
        if self.floor_heights_m.is_none()
            || self.entrances.is_none()
            || self.edges.is_empty()
            || self
                .edges
                .iter()
                .any(|edge| edge.exposed && edge.bays.is_none())
        {
            return Err(format!(
                "{}: floor, entrance or bay geometry is unresolved",
                self.id
            ));
        }
        if self.entrances.as_ref().is_some_and(Vec::is_empty) {
            return Err(format!(
                "{}: complete building geometry needs an entrance",
                self.id
            ));
        }
        for part in &self.parts {
            for facade in Facade::ALL {
                let half = match facade {
                    Facade::PositiveX | Facade::NegativeX => part.half_extents[1],
                    Facade::PositiveY | Facade::NegativeY => part.half_extents[0],
                };
                let mut spans: Vec<_> = self
                    .edges
                    .iter()
                    .filter(|e| e.part == part.id && e.facade == facade)
                    .map(|e| e.span_m)
                    .collect();
                spans.sort_by(|a, b| a[0].total_cmp(&b[0]));
                let tolerance = 64.0 * f64::EPSILON * half.max(1.0);
                let mut end = -half;
                for span in spans {
                    if (span[0] - end).abs() > tolerance {
                        return Err(format!(
                            "{}: facade spans must cover each part face without gaps or overlap",
                            self.id
                        ));
                    }
                    end = span[1];
                }
                if (end - half).abs() > tolerance {
                    return Err(format!("{}: facade geometry is unresolved", self.id));
                }
            }
        }
        Ok(())
    }

    /// Count a validated catalogue descriptor's lattice before materializing it.
    /// The physical template owner defines the range; compiler admission never
    /// recreates pitch/span arithmetic or allocates a coordinate vector to count.
    pub fn bay_position_count(&self) -> Result<usize, String> {
        self.edges.iter().try_fold(0usize, |total, edge| {
            let count = edge
                .bays
                .map(|pattern| pattern.range(edge.span_m))
                .transpose()
                .map_err(|message| format!("{}: {message}", self.id))?
                .map_or(0, |range| range.count);
            total
                .checked_add(count)
                .ok_or_else(|| format!("{}: bay count overflow", self.id))
        })
    }

    pub fn join(&self, id: &str) -> Result<&SupportedJoin, String> {
        self.validate()?;
        self.joins
            .iter()
            .find(|join| join.id == id)
            .ok_or_else(|| format!("{}: unsupported join {id}", self.id))
    }

    pub fn materialize(&self, frame: PlacementFrame) -> Result<MaterializedBuilding, String> {
        self.validate()?;
        if frame.translation.iter().any(|v| !v.is_finite()) || !frame.yaw.is_finite() {
            return Err(format!("{}: placement frame must be finite", self.id));
        }
        let rotation = libm::sincos(frame.yaw);
        let position = |point| {
            let [x, y] = rotate(point, rotation);
            [frame.translation[0] + x, frame.translation[1] + y]
        };
        // Validate every transformed lattice before allocating any bay vector.
        for edge in &self.edges {
            let local = self.local_edge(edge)?;
            local
                .validate_points(edge, position)
                .map_err(|message| format!("{}: {message}", self.id))?;
        }
        let mut edges = self
            .edges
            .iter()
            .map(|edge| {
                let local = self.local_edge(edge)?;
                let bays = if let Some(pattern) = edge.bays {
                    let range = pattern
                        .range(edge.span_m)
                        .map_err(|message| format!("{}: {message}", self.id))?;
                    let mut points = Vec::new();
                    points
                        .try_reserve_exact(range.count)
                        .map_err(|e| format!("{}: bay allocation: {e}", self.id))?;
                    for i in 0..range.count {
                        points.push(position(local.point(
                            pattern.phase_m + (range.first + i as f64) * pattern.pitch_m,
                        )));
                    }
                    Some(points)
                } else {
                    None
                };
                Ok(MaterializedEdge {
                    id: edge.id.clone(),
                    part: edge.part.clone(),
                    facade: edge.facade,
                    span_m: edge.span_m,
                    span: edge.span_m.map(|offset| position(local.point(offset))),
                    normal: rotate(local.normal, rotation),
                    base_z: frame.translation[2] + local.base_z,
                    top_z: frame.translation[2] + local.top_z,
                    exposed: edge.exposed,
                    bays,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut entrances = self
            .entrances
            .as_ref()
            .map(|entrances| {
                entrances
                    .iter()
                    .map(|entrance| {
                        let edge = self
                            .edges
                            .iter()
                            .find(|e| e.id == entrance.edge)
                            .ok_or_else(|| {
                                format!("{}: unknown entrance edge {}", self.id, entrance.edge)
                            })?;
                        let local = self.local_edge(edge)?;
                        let [x, y] = position(local.point(entrance.offset_m));
                        Ok(MaterializedEntrance {
                            id: entrance.id.clone(),
                            position: [x, y, frame.translation[2] + local.base_z],
                            normal: rotate(local.normal, rotation),
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?;
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        if let Some(entrances) = &mut entrances {
            entrances.sort_by(|a, b| a.id.cmp(&b.id));
        }
        let mut parts: Vec<_> = self
            .parts
            .iter()
            .map(|part| MaterializedPart {
                id: part.id.clone(),
                center: position(part.center),
                yaw: frame.yaw + part.yaw,
                half_extents: part.half_extents,
                base_z: frame.translation[2] + part.base_z,
            })
            .collect();
        parts.sort_by(|a, b| a.id.cmp(&b.id));
        let geometry = MaterializedBuilding {
            template_id: self.id.clone(),
            frame,
            parts,
            height_m: self.height_m(),
            floor_z: self
                .floor_heights_m
                .as_ref()
                .map(|heights| heights.iter().map(|h| frame.translation[2] + h).collect()),
            entrances,
            edges,
        };
        let finite = geometry.parts.iter().all(|part| {
            part.center
                .into_iter()
                .chain([
                    part.yaw,
                    part.base_z,
                    part.base_z + 2.0 * part.half_extents[2],
                ])
                .all(f64::is_finite)
        }) && geometry.floor_z.iter().flatten().all(|z| z.is_finite())
            && geometry.entrances.iter().flatten().all(|entrance| {
                entrance
                    .position
                    .into_iter()
                    .chain(entrance.normal)
                    .all(f64::is_finite)
            })
            && geometry.edges.iter().all(|edge| {
                edge.span
                    .into_iter()
                    .flatten()
                    .chain(edge.normal)
                    .chain([edge.base_z, edge.top_z])
                    .all(f64::is_finite)
                    && edge.bays.iter().flatten().flatten().all(|v| v.is_finite())
            });
        if !finite {
            return Err(format!(
                "{}: placement overflows finite world geometry",
                self.id
            ));
        }
        geometry.validate().map_err(|message| {
            format!("placement cannot represent physical geometry: {message}")
        })?;
        Ok(geometry)
    }

    fn local_edge(&self, edge: &FacadeEdge) -> Result<LocalEdge, String> {
        let part = self
            .parts
            .iter()
            .find(|p| p.id == edge.part)
            .ok_or_else(|| format!("{}: unknown edge part {}", self.id, edge.part))?;
        let [hx, hy, hz] = part.half_extents;
        let (normal, along, reach) = match edge.facade {
            Facade::PositiveX => ([1.0, 0.0], [0.0, 1.0], hx),
            Facade::PositiveY => ([0.0, 1.0], [-1.0, 0.0], hy),
            Facade::NegativeX => ([-1.0, 0.0], [0.0, -1.0], hx),
            Facade::NegativeY => ([0.0, -1.0], [1.0, 0.0], hy),
        };
        let rotation = libm::sincos(part.yaw);
        let normal = rotate(normal, rotation);
        let along = rotate(along, rotation);
        Ok(LocalEdge {
            center: part.center,
            offset: [normal[0] * reach, normal[1] * reach],
            origin: [
                part.center[0] + normal[0] * reach,
                part.center[1] + normal[1] * reach,
            ],
            normal,
            along,
            base_z: part.base_z,
            top_z: part.base_z + 2.0 * hz,
        })
    }
}
