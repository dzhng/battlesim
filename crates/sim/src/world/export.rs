//! Flat, f32 exports of the authoritative geometry for presentation and lab
//! probes. Layout (strides, offsets, enum tags) is described by
//! [`layout_json`] so consumers never hardcode it.
use super::{Collider, Hit, Surface, SurfaceKind, WorldGeometry};
use contract::catalog::{PropBody, PropCatalog};
use contract::map::MoverClass;

pub const SURFACE_KINDS: [SurfaceKind; 5] = [
    SurfaceKind::Ground,
    SurfaceKind::Road,
    SurfaceKind::Water,
    SurfaceKind::Bridge,
    SurfaceKind::Sidewalk,
];
/// Per-vertex surface flags alongside the kind tag.
pub const FLAG_FOREST: u8 = 1;
pub const FLAG_BLOCKED: u8 = 2;
/// idLo, idHi, kind, center x, center y, yaw, half x, half y, half z, base z.
pub const PROP_STRIDE: usize = 10;
/// min x, min y, width, height, z.
pub const AREA_STRIDE: usize = 5;
/// One exact stroke segment: end a, end b, half width and surface tag.
pub const SURFACE_STROKE_STRIDE: usize = 6;
pub const SURFACE_TRIANGLE_STRIDE: usize = 7;
pub const SURFACE_BOUNDARY_STRIDE: usize = 5;

fn surface_tag(kind: SurfaceKind) -> u8 {
    SURFACE_KINDS.iter().position(|k| *k == kind).unwrap() as u8
}

pub(super) fn surface_area_tag(kind: contract::map::SurfaceKind) -> f32 {
    surface_tag(match kind {
        contract::map::SurfaceKind::Road => SurfaceKind::Road,
        contract::map::SurfaceKind::Sidewalk => SurfaceKind::Sidewalk,
    }) as f32
}

/// The layout, with the prop types' `blocks`, `occludes` and weight
/// columns spelled out per type, and what draws each, for presentation.
/// A prop's kind tag is its type's index in `propKinds`.
pub fn layout_json(types: &PropCatalog) -> String {
    let lower = |name: String| name.to_lowercase();
    let kinds = |keep: &dyn Fn(&PropBody) -> bool| {
        types
            .kinds()
            .filter(|&k| keep(&types.get(k).body))
            .map(|k| types.id(k))
            .collect::<Vec<_>>()
    };
    serde_json::json!({
        "surfaceKinds": SURFACE_KINDS.iter().map(|k| lower(format!("{k:?}"))).collect::<Vec<_>>(),
        "propKinds": types.ids(),
        // What draws each prop type (`PropAppearance`).
        "propAppearance": types
            .kinds()
            .map(|k| (types.id(k).to_string(), serde_json::json!(types.get(k).appearance)))
            .collect::<serde_json::Map<_, _>>(),
        // Per mover class, the prop kinds that stop it.
        "blockingPropKinds": MoverClass::ALL
            .iter()
            .map(|&c| (lower(format!("{c:?}")), serde_json::json!(kinds(&|b| b.blocks.class(c)))))
            .collect::<serde_json::Map<_, _>>(),
        // The prop kinds that hide what lies behind them from sight.
        "occludingPropKinds": kinds(&|b| b.occludes),
        // The prop kinds something can shove: drawn apart from the static world.
        "movablePropKinds": kinds(&|b| b.weight_class != contract::scenario::WeightClass::Immovable),
        // The prop kinds fire can destroy (the integrity column, Q19): drawn
        // apart too, so one seen destroyed leaves the world.
        "destroyablePropKinds": kinds(&|b| b.hp.is_some()),
        "flags": { "forest": FLAG_FOREST, "blocked": FLAG_BLOCKED },
        "propStride": PROP_STRIDE,
        "limbBits":16,
        "garrisonPropKinds":kinds(&|b|b.garrison),
        "areaStride": AREA_STRIDE,
        "propFields": ["idLo", "idHi", "kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ"],
        "areaFields": ["x", "y", "w", "h", "z"],
        "surfaceStrokeStride": SURFACE_STROKE_STRIDE,
        "surfaceStrokeFields": ["ax", "ay", "bx", "by", "halfWidth", "kind"],
        "surfaceTriangleStride": SURFACE_TRIANGLE_STRIDE,
        "surfaceTriangleFields": ["ax", "ay", "bx", "by", "cx", "cy", "kind"],
        "surfaceBoundaryStride": SURFACE_BOUNDARY_STRIDE,
        "surfaceBoundaryFields": ["ax", "ay", "bx", "by", "kind"],
        "forestTrunkRangeStride": 2,
        "forestTrunkRangeFields": ["firstProp", "onePastProp"],
        "forestMetadataStride": 3,
        "forestMetadataFields": ["id", "canopy", "shapeKind"],
        "forestShapeKinds": ["rectangle", "stroke", "polygon"],
        "forestStrokeStride": 6,
        "forestStrokeFields": ["ax", "ay", "bx", "by", "halfWidth", "id"],
        "forestTriangleStride": 7,
        "forestTriangleFields": ["ax", "ay", "bx", "by", "cx", "cy", "id"],
        "forestBoundaryStride": 5,
        "forestBoundaryFields": ["ax", "ay", "bx", "by", "id"],
    })
    .to_string()
}

/// One exact integer ID and the presentation's unchanged physical columns.
pub fn prop_record(p: &super::Prop) -> [f32; PROP_STRIDE] {
    let [lo, hi] = crate::publication::limbs(p.id);
    [
        lo,
        hi,
        p.kind.0 as f32,
        p.center.x as f32,
        p.center.y as f32,
        p.yaw as f32,
        p.half.x as f32,
        p.half.y as f32,
        p.half.z as f32,
        p.base_z as f32,
    ]
}

impl WorldGeometry {
    pub fn export_terrain_grid(&self) -> String {
        serde_json::json!({"nx":self.field.nx,"ny":self.field.ny,"spacing":self.field.spacing,
            "pageSize":super::terrain::HEIGHT_PAGE_SIZE,"minHeight":self.field.bottom()})
        .to_string()
    }

    pub fn export_terrain_page_ids(&self) -> Vec<u32> {
        self.field.export_samples().0.clone()
    }

    pub fn export_terrain_heights(&self) -> Vec<f32> {
        self.field.export_samples().1.clone()
    }

    pub fn export_terrain_positions(&self) -> Vec<f32> {
        let (vertices, _) = self.terrain_mesh();
        vertices
            .iter()
            .flat_map(|v| [v.x as f32, v.y as f32, v.z as f32])
            .collect()
    }

    pub fn export_terrain_indices(&self) -> Vec<u32> {
        self.terrain_mesh().1.to_vec()
    }

    /// Two bytes per terrain triangle (index order): ground kind tag, flags.
    /// Classified at each triangle's centroid, so the slope is that triangle's.
    pub fn export_terrain_triangle_surfaces(&self) -> Vec<u8> {
        let (vertices, indices) = self.terrain_mesh();
        indices
            .chunks(3)
            .flat_map(|t| {
                let c =
                    (vertices[t[0] as usize] + vertices[t[1] as usize] + vertices[t[2] as usize])
                        * (1.0 / 3.0);
                let s = self
                    .ground_surface_at(c.x, c.y)
                    .expect("triangle centroid is in bounds");
                let mut flags = 0;
                if s.forest {
                    flags |= FLAG_FOREST;
                }
                if !s.traversable {
                    flags |= FLAG_BLOCKED;
                }
                [surface_tag(s.kind), flags]
            })
            .collect()
    }

    pub fn export_props(&self) -> Vec<f32> {
        self.props().flat_map(prop_record).collect()
    }

    /// Immutable building references; physical prop geometry stays in props().
    pub fn export_buildings(&self) -> String {
        serde_json::json!({"catalogueHash":self.template_catalog_hash,"buildings":self.buildings.definitions().map(|b|serde_json::json!({
            "owner":b.owner,"kind":b.kind,"templateId":b.geometry.template_id,"category":b.category,
            "regionalFamily":b.regional_family,"parts":b.parts
        })).collect::<Vec<_>>()}).to_string()
    }

    /// Water rects with their surface height.
    pub fn export_water(&self) -> Vec<f32> {
        self.water()
            .iter()
            .flat_map(|w| {
                [w.rect[0], w.rect[1], w.rect[2], w.rect[3], w.surface_z].map(|v| v as f32)
            })
            .collect()
    }

    /// Exact authored strokes: ax, ay, bx, by, half width, surface kind.
    pub fn export_surface_strokes(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for area in self.surfaces.areas() {
            if let contract::ground::GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            {
                let kind = surface_area_tag(area.kind) as f64;
                for p in centerline.samples().windows(2) {
                    out.extend(
                        [p[0][0], p[0][1], p[1][0], p[1][1], width_m / 2.0, kind].map(|v| v as f32),
                    );
                }
            }
        }
        out
    }

    /// Native polygon membership triangles: ax, ay, bx, by, cx, cy, kind.
    pub fn export_surface_triangles(&self) -> Vec<f32> {
        self.surfaces.triangles().to_vec()
    }

    /// Exposed polygon-union boundary segments: ax, ay, bx, by, surface kind.
    pub fn export_surface_boundaries(&self) -> Vec<f32> {
        self.surfaces.boundaries().to_vec()
    }

    /// Actual rectangular forests only, preserving the original rectangle arithmetic.
    pub fn export_forests(&self) -> Vec<f32> {
        self.forests()
            .iter()
            .filter_map(|f| {
                f.shape.exact_rectangle().map(|r| {
                    [r[0], r[1], r[2], r[3], self.forest.rule.canopy_height_m].map(|v| v as f32)
                })
            })
            .flatten()
            .collect()
    }

    /// Original source prop ranges by authored forest; immutable across removals.
    pub fn export_forest_trunk_ranges(&self) -> Vec<u32> {
        self.forest
            .trunk_ranges()
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    /// Rect export ordinal -> authored forest ID, for mixed shape maps.
    pub fn export_forest_rect_ids(&self) -> Vec<u32> {
        self.forests()
            .iter()
            .enumerate()
            .filter_map(|(id, f)| f.shape.exact_rectangle().map(|_| id as u32))
            .collect()
    }

    /// Authored order: ID, canopy height, physical shape kind (rect/stroke/polygon).
    pub fn export_forest_metadata(&self) -> Vec<f32> {
        self.forests()
            .iter()
            .enumerate()
            .flat_map(|(id, f)| {
                let kind = if f.shape.exact_rectangle().is_some() {
                    0.0
                } else if matches!(f.shape, contract::ground::GroundShape::Stroke { .. }) {
                    1.0
                } else {
                    2.0
                };
                [id as f32, self.forest.rule.canopy_height_m as f32, kind]
            })
            .collect()
    }

    pub fn export_forest_strokes(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for (id, f) in self.forests().iter().enumerate() {
            if let contract::ground::GroundShape::Stroke {
                centerline,
                width_m,
            } = &f.shape
            {
                for p in centerline.samples().windows(2) {
                    out.extend(
                        [p[0][0], p[0][1], p[1][0], p[1][1], width_m / 2.0, id as f64]
                            .map(|v| v as f32),
                    );
                }
            }
        }
        assert!(
            out.iter().all(|v| v.is_finite()),
            "forest stroke export must fit finite f32 coordinates"
        );
        out
    }

    pub fn export_forest_triangles(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for (id, f) in self.forests().iter().enumerate() {
            if f.shape.exact_rectangle().is_some() {
                continue;
            }
            if let contract::ground::GroundShape::Polygon { ring } = &f.shape {
                for triangle in contract::ground::triangulate(ring).expect("validated forest ring")
                {
                    out.extend(triangle.into_iter().flatten().map(|v| v as f32));
                    out.push(id as f32);
                }
            }
        }
        assert!(
            out.iter().all(|v| v.is_finite()),
            "forest polygon export must fit finite f32 coordinates"
        );
        out
    }

    pub fn export_forest_boundaries(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for (id, f) in self.forests().iter().enumerate() {
            if f.shape.exact_rectangle().is_some() {
                continue;
            }
            if let contract::ground::GroundShape::Polygon { ring } = &f.shape {
                for (a, b) in contract::ground::edges(ring) {
                    out.extend([a[0], a[1], b[0], b[1], id as f64].map(|v| v as f32));
                }
            }
        }
        assert!(
            out.iter().all(|v| v.is_finite()),
            "forest boundary export must fit finite f32 coordinates"
        );
        out
    }
}

/// `[z, nx, ny, nz, slope_deg, kind, forest, traversable]`, or empty out of bounds.
pub fn surface_record(s: Option<Surface>) -> Vec<f64> {
    s.map_or_else(Vec::new, |s| {
        vec![
            s.z,
            s.normal.x,
            s.normal.y,
            s.normal.z,
            s.slope_deg,
            surface_tag(s.kind) as f64,
            s.forest as u8 as f64,
            s.traversable as u8 as f64,
        ]
    })
}

/// `[t, x, y, z, nx, ny, nz, prop_id | -1]`, or empty on a miss.
pub fn hit_record(h: Option<Hit>) -> Vec<f64> {
    h.map_or_else(Vec::new, |h| {
        vec![
            h.t,
            h.point.x,
            h.point.y,
            h.point.z,
            h.normal.x,
            h.normal.y,
            h.normal.z,
            match h.collider {
                Collider::Terrain => -1.0,
                Collider::Prop(id) => id as f64,
            },
        ]
    })
}
