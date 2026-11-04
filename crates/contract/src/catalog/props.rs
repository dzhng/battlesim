//! Prop types: what a body of each blocks, stops, hides and weighs, what it
//! becomes when destroyed, and what draws it, by rank in the catalog.
use super::*;

/// A prop type's place in the catalog: its rank among the prop ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PropKind(pub u16);

/// One prop type (Q19, Q28): what a body of it blocks, stops, hides and
/// weighs, what it becomes when destroyed, and what draws it. The rules read
/// its body's columns, never its id; a new obstacle is a new entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropType {
    /// It is what a unit leaves when destroyed: each placement names the
    /// unit type it was (`wreck_of`), and only a wreck's does.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wreck: bool,
    pub body: PropBody,
    /// What a destroyed body becomes; with `body.hp`, and only with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<Destroyed>,
    pub appearance: PropAppearance,
}

/// A prop type's body row. Each column is independent and has its own
/// readers: `blocks` navigation and collision; `stops_rounds` flight;
/// `occludes` the fog sweep, sensing and the renderer's sight-light
/// occluders; `weight_class` pushing; `cover_tier` cover. A row that blocks
/// nobody, stops no rounds and gives no cover but occludes for a
/// `lifetime_s` is a smoke screen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropBody {
    pub blocks: Blocks,
    pub stops_rounds: bool,
    /// Hides what lies behind it from sight (Q25: only big static bodies).
    pub occludes: bool,
    pub weight_class: WeightClass,
    /// The cover it gives infantry (Q4, Q24): from behind a body that blocks
    /// infantry, or from inside a ground body (rubble) that does not.
    #[serde(default)]
    pub cover_tier: Option<CoverTier>,
    /// A transient body: it goes this long after it appears.
    #[serde(default)]
    pub lifetime_s: Option<f64>,
    /// How much of a fog cell one such body conceals, in [0, 1) (Q21): a
    /// cell's foliage is `1 − Π(1 − conceals)` over the concealing bodies
    /// whose canopy covers it.
    #[serde(default)]
    pub conceals: f64,
    /// Integrity (Q17, 34c): the structural damage it takes to destroy one
    /// such body; with `building_floor_bands` this is the coefficient per
    /// square metre and floor band. None: ordinary fire never destroys it.
    #[serde(default)]
    pub hp: Option<f64>,
    #[serde(default, skip_serializing_if = "HpScale::is_fixed")]
    pub hp_scale: HpScale,
    /// The share of a direct round's structural damage it takes (blast is
    /// not scaled); 1 when absent.
    #[serde(default = "one")]
    pub armor: f64,
    /// It falls rather than slides (Q16): a vehicle that can shove it knocks
    /// it down, and a vehicle that can knock it down clears a lane of the
    /// forest ground it stood on. A tree.
    #[serde(default)]
    pub topples: bool,
    /// A squad can garrison it: a fighting position its soldiers hold from
    /// inside (Q22, the named garrison mechanic).
    #[serde(default)]
    pub garrison: bool,
    /// Its outside is metal: a hull grinding it as it shoves it, or a tracer
    /// striking it, throws sparks.
    #[serde(default)]
    pub metal: bool,
}

/// A fixed integrity value, or a coefficient per square metre and usable floor band.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HpScale {
    #[default]
    Fixed,
    BuildingFloorBands,
}
impl HpScale {
    fn is_fixed(&self) -> bool {
        *self == Self::Fixed
    }
}

fn one() -> f64 {
    1.0
}

/// A destroyed body's state (Q17): gone, gone and its ground cleared (a
/// tree: open ground, like a lane a tank knocks through), or another prop
/// type on the same plan at `height_m` (a building's ruin, sandbags'
/// rubble, a lighter wreck).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Destroyed {
    Removed,
    Cleared,
    Into {
        prop: String,
        height_m: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        building: Option<BuildingRemains>,
    },
}

/// A building's data-driven collapse height and tall terminal shell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRemains {
    pub height_fraction: f64,
    pub max_height_m: f64,
    pub collapse_max_floors: usize,
    pub gutted_prop: String,
}

/// What draws a prop type (presentation reads it; the rules never do).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropAppearance {
    /// Physical catalog entry whose fitted art is still awaiting acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PropAppearanceStatus>,
    /// What draws it: the asset catalog's scenery kind whose appearances are
    /// fitted to its box, `building` for a part its building draws from its
    /// template's art, `forest` for the trees a forest draws itself.
    pub drawn_by: String,
    /// Authored sRGB colour when this prop needs a stand-in box.
    pub stand_in_tint: [f64; 3],
    /// Drawn by repeating one module along the box's long side (a wall, a
    /// fence), not stretched.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub modular: bool,
    /// Only a map places one: a battle never leaves or drops one, so only the
    /// appearances a map uses are loaded.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub map_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropAppearanceStatus {
    SystemsOnly,
}

/// The physical owner that will place a prop and its later destruction states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropPlacement {
    Ordinary,
    Aggregate,
}

/// The prop types, in id order: [`PropKind`] is a rank.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropCatalog {
    pub(super) ids: Vec<String>,
    pub(super) types: Vec<PropType>,
}

impl PropCatalog {
    /// Validate every state against its placement owner. Catalog resolution
    /// already proves the chain acyclic and each gutted alternative terminal.
    pub fn check_placement(&self, kind: PropKind, placement: PropPlacement) -> Result<(), String> {
        let check = |kind| {
            let body = self.get(kind).body;
            match placement {
                PropPlacement::Ordinary
                    if body.garrison || body.hp_scale == HpScale::BuildingFloorBands =>
                {
                    Err(format!("prop {:?} requires a placed aggregate for garrison or building-scaled integrity", self.id(kind)))
                }
                PropPlacement::Aggregate
                    if body.weight_class != crate::scenario::WeightClass::Immovable =>
                {
                    Err(format!("aggregate state {:?} requires an immovable body until composite motion exists", self.id(kind)))
                }
                _ => Ok(()),
            }
        };
        let mut kind = kind;
        loop {
            check(kind)?;
            let Some(Destroyed::Into { prop, building, .. }) = &self.get(kind).destroyed else {
                return Ok(());
            };
            if let Some(policy) = building {
                check(self.kind(&policy.gutted_prop))?;
            }
            kind = self.kind(prop);
        }
    }

    /// [`Self::check_placement`] for the prop type named `id`, which may name
    /// no type this catalog holds.
    pub fn check_placement_of(&self, id: &str, placement: PropPlacement) -> Result<(), String> {
        let kind = self
            .index(id)
            .ok_or_else(|| format!("unknown prop type {id:?}"))?;
        self.check_placement(kind, placement)
    }

    /// Every prop type's index, in id order.
    pub fn kinds(&self) -> impl Iterator<Item = PropKind> {
        (0..self.types.len() as u16).map(PropKind)
    }

    pub fn index(&self, id: &str) -> Option<PropKind> {
        self.ids
            .binary_search_by(|k| k.as_str().cmp(id))
            .ok()
            .map(|i| PropKind(i as u16))
    }

    /// The prop type named `id`, which a caller holds from this catalog.
    pub fn kind(&self, id: &str) -> PropKind {
        self.index(id)
            .unwrap_or_else(|| panic!("no prop type {id:?}"))
    }

    pub fn id(&self, k: PropKind) -> &str {
        &self.ids[k.0 as usize]
    }

    /// The ids in index order: the publications' prop type table.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    pub fn get(&self, k: PropKind) -> &PropType {
        &self.types[k.0 as usize]
    }

    /// The prop type named `id`, which a caller holds from this catalog.
    pub fn by_id(&self, id: &str) -> &PropType {
        self.get(self.kind(id))
    }
}
