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
/// One stretch of a stroke between two rounded samples: end a, end b, half
/// width, its area's kind (`surface_area_tag`), and which of its ends are cut square
/// (`contract::ground::CUT_A | CUT_B`, from `contract::ground::stretches`).
pub const SURFACE_STROKE_STRIDE: usize = 7;
/// A forest stroke's stretch: as a surface's, with the forest's id for the tag.
pub const FOREST_STROKE_STRIDE: usize = 7;
pub const SURFACE_TRIANGLE_STRIDE: usize = 7;
pub const SURFACE_BOUNDARY_STRIDE: usize = 5;
/// One stretch of a river between two rounded samples: end a, end b, the
/// water's half width at each, the bank's grade at each, how high the land
/// stands above the water at the edge at each, the surface height.
pub const RIVER_STRIDE: usize = 11;

fn surface_tag(kind: SurfaceKind) -> u8 {
    SURFACE_KINDS.iter().position(|k| *k == kind).unwrap() as u8
}

/// The tag a paved area's stretches, triangles and boundary edges carry: its
/// own kind as the map names it, an index into the layout's
/// `surfaceAreaKinds`. Each kind is drawn as itself; what a unit finds there
/// is still `SurfaceKind::of` it.
pub fn surface_area_tag(kind: contract::map::SurfaceKind) -> f32 {
    contract::map::SurfaceKind::ALL
        .iter()
        .position(|k| *k == kind)
        .unwrap() as f32
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
    let area_kinds = |keep: &dyn Fn(contract::map::SurfaceKind) -> bool| {
        contract::map::SurfaceKind::ALL
            .iter()
            .filter(|&&k| keep(k))
            .map(|k| serde_json::json!(k))
            .collect::<Vec<_>>()
    };
    let mut layout = serde_json::json!({
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
        "surfaceStrokeFields": ["ax", "ay", "bx", "by", "halfWidth", "kind", "cuts"],
        "strokeCuts": { "a": contract::ground::CUT_A, "b": contract::ground::CUT_B },
        "surfaceRunStride": 4,
        "surfaceRunFields": ["ax", "ay", "bx", "by"],
        "surfaceTriangleStride": SURFACE_TRIANGLE_STRIDE,
        "surfaceTriangleFields": ["ax", "ay", "bx", "by", "cx", "cy", "kind"],
        "surfaceBoundaryStride": SURFACE_BOUNDARY_STRIDE,
        "surfaceBoundaryFields": ["ax", "ay", "bx", "by", "kind"],
        "riverStride": RIVER_STRIDE,
        "riverFields": ["ax", "ay", "bx", "by", "halfA", "halfB", "gradeA", "gradeB", "bankA", "bankB", "surfaceZ"],
        "riverRunStride": 4,
        "riverRunFields": ["ax", "ay", "bx", "by"],
        "forestTrunkRangeStride": 2,
        "forestTrunkRangeFields": ["firstProp", "onePastProp"],
        "forestMetadataStride": 3,
        "forestMetadataFields": ["id", "canopy", "shapeKind"],
        "forestShapeKinds": ["rectangle", "stroke", "polygon"],
        "forestStrokeStride": FOREST_STROKE_STRIDE,
        "forestStrokeFields": ["ax", "ay", "bx", "by", "halfWidth", "id", "cuts"],
        "forestTriangleStride": 7,
        "forestTriangleFields": ["ax", "ay", "bx", "by", "cx", "cy", "id"],
        "forestBoundaryStride": 5,
        "forestBoundaryFields": ["ax", "ay", "bx", "by", "id"],
    });
    // What a paved stretch, triangle or boundary edge's `kind` names, and
    // which of those are carriageways. (Set here: the object above is as
    // long as the `json!` macro expands.)
    layout["surfaceAreaKinds"] = area_kinds(&|_| true).into();
    layout["roadAreaKinds"] = area_kinds(&|k| k.is_road()).into();
    layout.to_string()
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

    /// Immutable building references: each building's template and the frame
    /// it was placed at, which is where its template's art is drawn. Physical
    /// prop geometry stays in props().
    pub fn export_buildings(&self) -> String {
        serde_json::json!({"catalogueHash":self.template_catalog_hash,"buildings":self.buildings.definitions().map(|b|serde_json::json!({
            "owner":b.owner,"kind":b.kind,"templateId":b.geometry.template_id,"category":b.category,
            "regionalFamily":b.regional_family,"frame":b.geometry.frame,"parts":b.parts
        })).collect::<Vec<_>>()}).to_string()
    }

    /// Every river's rounded stretches: ax, ay, bx, by, the water's half
    /// width at each end, the bank's grade at each end, the height of the
    /// land above the water at the edge at each end (the lower of the two
    /// banks), the surface height.
    /// A point's distance inside the water is the half width at the closest
    /// point of a stretch less its distance to it, the deepest over stretches
    /// (`contract::river::section`): the drawing's edge is the rule's. The
    /// grade and the bank's height say how the ground was cut, for shading
    /// that follows the cross-section and not the grid's triangles.
    pub fn export_rivers(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for river in self.rivers() {
            for p in river.samples().windows(2) {
                let bank = |t: f64| {
                    contract::river::edges(&p[0], &p[1], t)
                        .map(|edge| contract::map::relief_height(&self.relief, edge[0], edge[1]))
                        .into_iter()
                        .fold(f64::INFINITY, f64::min)
                        - river.surface_z()
                };
                out.extend(
                    [
                        p[0].xy[0],
                        p[0].xy[1],
                        p[1].xy[0],
                        p[1].xy[1],
                        p[0].half_width_m,
                        p[1].half_width_m,
                        p[0].depth_m / p[0].half_width_m,
                        p[1].depth_m / p[1].half_width_m,
                        bank(0.0),
                        bank(1.0),
                        river.surface_z(),
                    ]
                    .map(|v| v as f32),
                );
            }
        }
        out
    }

    /// The long runs of every river: ax, ay, bx, by. A field is cut along
    /// these, as along a road's. They join authored points, leaving out
    /// those that stray from the run by less than half the river's narrowest
    /// water: a meander authored point by point then cuts fields along a few
    /// long chords that stay in its water, not into a fan of slivers.
    pub fn export_river_runs(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for river in self.rivers() {
            let points: Vec<[f64; 2]> = river.points().iter().map(|p| p.xy).collect();
            for p in long_runs(&points, river.narrowest_width_m() / 2.0).windows(2) {
                out.extend([p[0][0], p[0][1], p[1][0], p[1][1]].map(|v| v as f32));
            }
        }
        out
    }

    /// The authored control runs of every road stroke: ax, ay, bx, by. A
    /// field is cut along these, not along a bend's short samples.
    pub fn export_surface_runs(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for area in self.surfaces.areas() {
            if let contract::ground::GroundShape::Stroke { centerline, .. } = &area.shape {
                if area.kind.is_road() {
                    for p in centerline.control_points().windows(2) {
                        out.extend([p[0][0], p[0][1], p[1][0], p[1][1]].map(|v| v as f32));
                    }
                }
            }
        }
        out
    }

    /// The rounded strokes every consumer samples: ax, ay, bx, by, half width,
    /// surface kind, cut ends. A point is the stroke's when it is within the
    /// half width of a stretch and not past an end that stretch is cut at
    /// (`contract::ground::stretch_contains`).
    pub fn export_surface_strokes(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for area in self.surfaces.areas() {
            if let contract::ground::GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            {
                let kind = surface_area_tag(area.kind) as f64;
                for (a, b, cuts) in contract::ground::stretches(centerline.samples(), width_m / 2.0)
                {
                    out.extend(
                        [a[0], a[1], b[0], b[1], width_m / 2.0, kind, cuts as f64]
                            .map(|v| v as f32),
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
                for (a, b, cuts) in contract::ground::stretches(centerline.samples(), width_m / 2.0)
                {
                    out.extend(
                        [
                            a[0],
                            a[1],
                            b[0],
                            b[1],
                            width_m / 2.0,
                            id as f64,
                            cuts as f64,
                        ]
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

/// `line` without the points that lie within `tolerance` of the run joining
/// the points kept either side (Ramer–Douglas–Peucker): its ends stay.
fn long_runs(line: &[[f64; 2]], tolerance: f64) -> Vec<[f64; 2]> {
    let mut keep = vec![false; line.len()];
    keep[0] = true;
    keep[line.len() - 1] = true;
    let mut spans = vec![(0, line.len() - 1)];
    while let Some((first, last)) = spans.pop() {
        let farthest = (first + 1..last)
            .map(|i| {
                (
                    i,
                    contract::ground::segment_distance(line[first], line[last], line[i]),
                )
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
        if let Some((i, distance)) = farthest {
            if distance > tolerance {
                keep[i] = true;
                spans.push((first, i));
                spans.push((i, last));
            }
        }
    }
    line.iter()
        .zip(keep)
        .filter_map(|(p, kept)| kept.then_some(*p))
        .collect()
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
