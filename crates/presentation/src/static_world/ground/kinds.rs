//! What the scenery reads off each kind's finest mesh, which the renderer
//! hands over in its world vertex (`mesh::VERTEX_FLOATS`: position, normal,
//! linear albedo, and 1 in the last float for foliage): its size, a tree's
//! trunk and crown, which a felled tree falls by, and the stump a felled
//! tree leaves standing.
use super::biome::Felled;
use super::js;
use super::mesh::VERTEX_FLOATS;
use super::scenery::Size;
use super::walk::Trunk;

/// A trunk's foot ring lies this close to its base.
const FOOT_M: f64 = 0.05;
/// Facets round a stump.
const STUMP_SIDES: usize = 10;

/// What a kind lends the scenery: its size, a tree's trunk (zeros for a
/// shrub or dressing, which no fall cuts) and a tree's stump mesh.
pub struct Kind {
    pub size: Size,
    pub trunk: Trunk,
    pub stump: Option<Vec<f32>>,
}

fn vertices(tier: &[f32]) -> &[[f32; VERTEX_FLOATS]] {
    tier.as_chunks::<VERTEX_FLOATS>().0
}

fn foliage(v: &[f32; VERTEX_FLOATS]) -> bool {
    v[9] > 0.5
}

/// The kind whose finest tier is `tier`; a tree (`felled`, the biome's
/// rules for one) cut at its stump's top.
pub fn kind(tier: &[f32], felled: Option<&Felled>) -> Result<Kind, String> {
    let size = size(tier);
    let Some(felled) = felled else {
        return Ok(Kind {
            size,
            trunk: Trunk::default(),
            stump: None,
        });
    };
    let hinge = felled.stump_height_m;
    Ok(Kind {
        size,
        trunk: Trunk {
            radius: trunk_at(tier, hinge)?.0,
            crown: crown_base(tier),
        },
        stump: Some(stump_mesh(tier, hinge, felled.cut)?),
    })
}

/// Unscaled height and crown reach from the trunk's axis.
pub fn size(tier: &[f32]) -> Size {
    let (mut height, mut radius) = (0.0, 0.0);
    for v in vertices(tier) {
        height = js::max(height, f64::from(v[2]));
        radius = js::max(radius, js::hypot(f64::from(v[0]), f64::from(v[1])));
    }
    Size { height, radius }
}

/// Where a tree's leaves begin, unscaled: its lowest leaf's height, or its
/// top with none.
pub fn crown_base(tier: &[f32]) -> f64 {
    let (mut low, mut top) = (f64::INFINITY, 0.0);
    for v in vertices(tier) {
        top = js::max(top, f64::from(v[2]));
        if foliage(v) {
            low = js::min(low, f64::from(v[2]));
        }
    }
    js::min(low, top)
}

/// A tree's trunk at `height` above its foot, unscaled: its radius and its
/// bark's mean colour. The trunk is rings of bark (the non-foliage
/// vertices): the foot ring's and the next ring's mean radii, interpolated
/// at `height`.
pub fn trunk_at(tier: &[f32], height: f64) -> Result<(f64, [f64; 3]), String> {
    // The lowest bark ring above the foot.
    let mut next = f64::INFINITY;
    for v in vertices(tier) {
        if !foliage(v) && f64::from(v[2]) > FOOT_M {
            next = js::min(next, f64::from(v[2]));
        }
    }
    // Each ring's summed radius and vertex count.
    let mut rings = [(0.0, 0usize); 2];
    let mut bark = [0.0; 3];
    for v in vertices(tier) {
        let z = f64::from(v[2]);
        if foliage(v) {
            continue;
        }
        let ring = if z <= FOOT_M {
            &mut rings[0]
        } else if (z - next).abs() <= FOOT_M {
            &mut rings[1]
        } else {
            continue;
        };
        ring.0 += js::hypot(f64::from(v[0]), f64::from(v[1]));
        ring.1 += 1;
        for (c, b) in bark.iter_mut().enumerate() {
            *b += f64::from(v[6 + c]);
        }
    }
    let [foot, ring] = rings;
    if foot.1 == 0 || ring.1 == 0 {
        return Err("scenery: a tree appearance has no bark ring at its foot".into());
    }
    let n = (foot.1 + ring.1) as f64;
    let bark = bark.map(|b| b / n);
    let (r0, r1) = (foot.0 / foot.1 as f64, ring.0 / ring.1 as f64);
    Ok((r0 + (r1 - r0) * js::min(1.0, height / next), bark))
}

/// A tree's stump: a short prism as wide as its trunk at `height`
/// (`trunk_at`), in its bark's colour, its top the cut face, coloured `cut`.
pub fn stump_mesh(tier: &[f32], height: f64, cut: [f64; 3]) -> Result<Vec<f32>, String> {
    let (radius, bark) = trunk_at(tier, height)?;
    let mut out = Vec::with_capacity(STUMP_SIDES * 9 * VERTEX_FLOATS);
    let mut vertex = |p: [f64; 3], n: [f64; 3], colour: [f64; 3]| {
        out.extend(p.into_iter().chain(n).chain(colour).map(|x| x as f32));
        out.push(0.0);
    };
    let tau = |k: usize| k as f64 / STUMP_SIDES as f64 * std::f64::consts::PI * 2.0;
    for k in 0..STUMP_SIDES {
        let (a0, a1) = (tau(k), tau(k + 1));
        let am = (a0 + a1) / 2.0;
        let (c0, s0, c1, s1) = (js::cos(a0), js::sin(a0), js::cos(a1), js::sin(a1));
        let (nx, ny) = (js::cos(am), js::sin(am));
        let (x0, y0, x1, y1) = (radius * c0, radius * s0, radius * c1, radius * s1);
        let side = [nx, ny, 0.0];
        // The side, counter-clockwise seen from outside.
        vertex([x0, y0, 0.0], side, bark);
        vertex([x1, y1, 0.0], side, bark);
        vertex([x1, y1, height], side, bark);
        vertex([x0, y0, 0.0], side, bark);
        vertex([x1, y1, height], side, bark);
        vertex([x0, y0, height], side, bark);
        // The cut face, seen from above.
        let up = [0.0, 0.0, 1.0];
        vertex([0.0, 0.0, height], up, cut);
        vertex([x0, y0, height], up, cut);
        vertex([x1, y1, height], up, cut);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUT: [f64; 3] = [0.4, 0.3, 0.2];

    /// A low-poly trunk as the tree appearances are: a ring at the foot
    /// 0.5 m out, the next ring 1 m up 0.3 m out, a narrower ring at 4 m,
    /// and a leaf 2.5 m out at 0.2 m.
    fn trunk() -> Vec<f32> {
        let mut v = vec![];
        let mut push = |x: f64, y: f64, z: f64, rgb: [f64; 3], foliage: f64| {
            v.extend([x, y, z, 0.0, 0.0, 1.0, rgb[0], rgb[1], rgb[2], foliage].map(|f| f as f32));
        };
        for k in 0..6 {
            let a = k as f64 / 6.0 * std::f64::consts::TAU;
            let (c, s) = (a.cos(), a.sin());
            push(0.5 * c, 0.5 * s, 0.0, [0.2, 0.1, 0.05], 0.0);
            push(0.3 * c, 0.3 * s, 1.0, [0.2, 0.1, 0.05], 0.0);
            push(0.2 * c, 0.2 * s, 4.0, [0.9, 0.9, 0.9], 0.0);
        }
        push(2.5, 0.0, 0.2, [0.0, 0.5, 0.0], 1.0);
        v
    }

    #[test]
    fn a_stump_is_the_trunks_width_at_its_height_its_bark_cut_at_its_height() {
        let mesh = stump_mesh(&trunk(), 0.5, CUT).unwrap();
        let (mut top, mut reach) = (0.0f64, 0.0f64);
        let mut colours = std::collections::BTreeSet::new();
        for v in vertices(&mesh) {
            top = top.max(f64::from(v[2]));
            reach = reach.max(f64::from(v[0]).hypot(f64::from(v[1])));
            colours.insert(format!("{:.3},{:.3},{:.3}", v[6], v[7], v[8]));
            assert_eq!(v[9], 0.0, "a stump is bark, never foliage");
        }
        assert!((top - 0.5).abs() < 1e-7);
        // Half way between the rings' radii, at half the ring's height.
        assert!((reach - 0.4).abs() < 1e-6, "{reach}");
        let want = ["0.200,0.100,0.050", "0.400,0.300,0.200"];
        assert_eq!(colours.into_iter().collect::<Vec<_>>(), want);
    }

    #[test]
    fn a_tree_is_as_tall_and_wide_as_its_mesh_its_crown_where_its_leaves_begin() {
        let felled = serde_json::from_value(serde_json::json!({
            "fall_s": 2.6, "settle_s": 0.6, "settle_deg": 4, "rest_height_m": 1,
            "rest_spread": 0.6, "stump_height_m": 0.5, "cut": CUT,
        }))
        .unwrap();
        let tree = kind(&trunk(), Some(&felled)).unwrap();
        assert_eq!((tree.size.height, tree.size.radius), (4.0, 2.5));
        assert!((tree.trunk.radius - 0.4).abs() < 1e-6);
        assert!((tree.trunk.crown - 0.2).abs() < 1e-6);
        assert!(tree.stump.is_some());
        let shrub = kind(&trunk(), None).unwrap();
        assert_eq!((shrub.trunk.radius, shrub.trunk.crown), (0.0, 0.0));
        assert!(shrub.stump.is_none());
    }

    #[test]
    fn a_tree_without_bark_at_its_foot_is_refused() {
        let leaves: Vec<f32> = [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 1.0].to_vec();
        assert!(trunk_at(&leaves, 0.4).is_err());
    }
}
