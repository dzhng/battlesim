//! Map and side-known prop bodies decoded for presentation.
use super::{PropBox, PropLayout};
use crate::static_world::fallen::Known;
use sim::publication::reader::KnownProp;
use sim::world::export::PROP_STRIDE;

/// A prop authored in the static map: its id and box.
#[derive(Clone, Debug)]
pub struct MapProp {
    pub id: f64,
    pub body: PropBox,
}

/// The map's props, from the world's exported records (`prop_record`).
pub fn map_props(records: &[f32], layout: &PropLayout) -> Vec<MapProp> {
    records
        .as_chunks::<PROP_STRIDE>()
        .0
        .iter()
        .map(|p| {
            let f = |i: usize| f64::from(p[i]);
            let wreck = p[10];
            MapProp {
                id: f(0) + f(1) * 65536.0,
                body: PropBox {
                    kind: layout.prop_kinds[p[2] as usize].clone(),
                    center: [f(3), f(4)],
                    yaw: f(5),
                    half: [f(6), f(7), f(8)],
                    base_z: f(9),
                    wreck_of: (wreck >= 0.0).then(|| layout.unit_kinds[wreck as usize].clone()),
                },
            }
        })
        .collect()
}

/// A prop a side knows: the map prop whose place it took (none for a prop
/// that is no map prop's, a wreck), whether it was destroyed with nothing
/// left, its id (the cook-off names a wreck by it), and its box where it
/// stands (none: a lab's stand-in that names only what the buildings read).
#[derive(Clone, Debug, PartialEq)]
pub struct KnownBody {
    pub authored: Option<f64>,
    pub id: Option<f64>,
    pub kind: String,
    pub destroyed: bool,
    pub body: Option<PropBox>,
}

impl KnownBody {
    /// The publication's known prop.
    pub fn published(k: &KnownProp) -> Self {
        KnownBody {
            authored: k.authored_prop,
            id: k.id,
            kind: k.kind.to_string(),
            destroyed: k.destroyed,
            body: Some(PropBox {
                kind: k.kind.to_string(),
                center: k.center,
                yaw: k.yaw,
                half: k.half,
                base_z: k.base_z,
                wreck_of: k.wreck_of.as_deref().map(str::to_owned),
            }),
        }
    }

    /// A lab's stand-in for one.
    pub fn lab(k: &Known) -> Self {
        KnownBody {
            authored: k.authored_prop,
            id: k.id,
            kind: k.kind.clone(),
            destroyed: k.destroyed,
            body: k.center.map(|center| PropBox {
                kind: k.kind.clone(),
                center,
                yaw: k.yaw,
                half: k.half,
                base_z: k.base_z,
                wreck_of: k.wreck_of.clone(),
            }),
        }
    }
}
