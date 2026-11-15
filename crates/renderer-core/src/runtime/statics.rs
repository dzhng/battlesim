use crate::bundle::BundleSource;
use contract::identity::ContentId;
use presentation::presentation::Presentation;

#[derive(Clone)]
pub(super) struct StaticIdentity {
    ground: u32,
    placed: bool,
    scenery: Vec<(String, ContentId)>,
    grass: Vec<(String, ContentId)>,
    density: Option<u64>,
}

impl StaticIdentity {
    pub(super) fn new<S: BundleSource>(
        presentation: &Presentation,
        library: &S,
        density: Option<f64>,
    ) -> Result<Self, String> {
        let ground = presentation
            .ground()
            .ok_or("renderer world has no presentation ground")?;
        let routes = |names: Vec<String>| {
            names
                .into_iter()
                .map(|name| library.content_id(&name).map(|id| (name, id)))
                .collect::<Result<Vec<_>, String>>()
        };
        Ok(Self {
            ground: presentation.ground_revision(),
            placed: presentation.scenery_is_placed(),
            scenery: routes(if presentation.scenery_is_placed() {
                ground.scenery_kinds()
            } else {
                Vec::new()
            })?,
            grass: routes(if density.is_some() {
                ground.grass_kinds()
            } else {
                Vec::new()
            })?,
            density: density.map(f64::to_bits),
        })
    }

    fn same_routes(a: &[(String, ContentId)], b: &[(String, ContentId)]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|((an, ai), (bn, bi))| an == bn && ai == bi)
    }

    pub(super) fn scenery_same(&self, other: &Self) -> bool {
        self.ground == other.ground
            && self.placed == other.placed
            && Self::same_routes(&self.scenery, &other.scenery)
    }

    pub(super) fn placed(&self) -> bool {
        self.placed
    }

    pub(super) fn grass_same(&self, other: &Self) -> bool {
        self.ground == other.ground
            && self.density == other.density
            && Self::same_routes(&self.grass, &other.grass)
    }
}
