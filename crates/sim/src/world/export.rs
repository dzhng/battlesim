//! Flat, f32 exports of the authoritative geometry for presentation and lab
//! probes. Layout (strides, offsets, enum tags) is described by
//! [`layout_json`] so consumers never hardcode it.
use super::{Collider, Hit, Surface, SurfaceKind, WorldGeometry};
use contract::catalog::{PropBody, PropCatalog};
use contract::map::MoverClass;

pub const SURFACE_KINDS: [SurfaceKind; 4] = [
    SurfaceKind::Ground,
    SurfaceKind::Road,
    SurfaceKind::Water,
    SurfaceKind::Bridge,
];
/// Per-vertex surface flags alongside the kind tag.
pub const FLAG_FOREST: u8 = 1;
pub const FLAG_BLOCKED: u8 = 2;
/// id, kind, center x, center y, yaw, half x, half y, half z, base z.
pub const PROP_STRIDE: usize = 9;
/// min x, min y, width, height, z.
pub const AREA_STRIDE: usize = 5;
/// One road segment: end a, end b, half width. A point is road where it lies
/// within half width of a segment (and is not water).
pub const ROAD_STRIDE: usize = 5;

fn surface_tag(kind: SurfaceKind) -> u8 {
    SURFACE_KINDS.iter().position(|k| *k == kind).unwrap() as u8
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
        "areaStride": AREA_STRIDE,
        "propFields": ["id", "kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ"],
        "areaFields": ["x", "y", "w", "h", "z"],
        "roadStride": ROAD_STRIDE,
        "roadFields": ["ax", "ay", "bx", "by", "halfWidth"],
    })
    .to_string()
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
        self.props()
            .flat_map(|p| {
                [
                    p.id as f32,
                    p.kind.0 as f32,
                    p.center.x as f32,
                    p.center.y as f32,
                    p.yaw as f32,
                    p.half.x as f32,
                    p.half.y as f32,
                    p.half.z as f32,
                    p.base_z as f32,
                ]
            })
            .collect()
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

    /// Every road as its segments, each with the road's half width: the
    /// geometry the road rule measures against.
    pub fn export_roads(&self) -> Vec<f32> {
        self.roads
            .iter()
            .flat_map(|(points, width)| {
                points
                    .windows(2)
                    .flat_map(move |w| [w[0].x, w[0].y, w[1].x, w[1].y, width / 2.0])
            })
            .map(|v| v as f32)
            .collect()
    }

    /// Forest rects with their canopy height.
    pub fn export_forests(&self) -> Vec<f32> {
        self.forests()
            .iter()
            .flat_map(|f| {
                [
                    f.rect[0],
                    f.rect[1],
                    f.rect[2],
                    f.rect[3],
                    f.canopy_height_m,
                ]
                .map(|v| v as f32)
            })
            .collect()
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
