//! Source art admission and observer-state transfer between art generations.
use super::*;

impl PropPopulation {
    /// The map's `props` drawn from `art`.
    pub fn new(art: PropArt, props: Vec<MapProp>) -> Self {
        let mut looks = Looks::default();
        art.looks(
            &props.iter().map(|p| p.body.clone()).collect::<Vec<_>>(),
            &mut looks,
        );
        let mut map = Vec::new();
        let mut bodies = Vec::new();
        for p in props {
            if art.is_tree(&p.body.kind) {
                continue;
            }
            let first = map.len();
            art.fit(&p.body, Some(p.id), &mut looks, &mut map);
            bodies.push((p, first..map.len()));
        }
        let by_id = bodies
            .iter()
            .enumerate()
            .map(|(i, (p, _))| (p.id as u64, i))
            .collect();
        let mut cells: BTreeMap<(i64, i64), Chunk> = BTreeMap::new();
        for (i, m) in map.iter().enumerate() {
            let [x, y] = [m.record[0], m.record[1]].map(f64::from);
            let cell = ((y / CHUNK_M).floor() as i64, (x / CHUNK_M).floor() as i64);
            let b = model_bounds(m);
            let chunk = cells.entry(cell).or_insert(Chunk {
                bounds: b,
                models: Vec::new(),
            });
            for k in 0..3 {
                chunk.bounds[k] = chunk.bounds[k].min(b[k]);
                chunk.bounds[k + 3] = chunk.bounds[k + 3].max(b[k + 3]);
            }
            chunk.models.push(i);
        }
        let chunks: Vec<_> = cells.into_values().collect();
        let lists = vec![Vec::new(); looks.list.len() * TIERS * (1 + CASCADES)];
        PropPopulation {
            hidden: vec![false; map.len()],
            prints: vec![0; lists.len()],
            lists,
            chunk_index: BoundsIndex::new(chunks.iter().map(|c| c.bounds)),
            chunks,
            known_index: BoundsIndex::default(),
            candidates: Vec::new(),
            known_candidates: Vec::new(),
            visited: 0,
            map,
            by_id,
            bodies,
            looks,
            art,
            known: Vec::new(),
            moving: Vec::new(),
            shoved: BTreeMap::new(),
            known_models: Vec::new(),
            known_bodies: 0,
            rows: Vec::new(),
        }
    }

    /// Stage new authored fitting facts without capturing observer knowledge.
    pub fn replace_art(
        &self,
        update: impl FnOnce(&mut PropInput) -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut input = self.art.source.clone();
        update(&mut input)?;
        let art = PropArt::new(self.art.layout.clone(), input)?;
        Ok(Self::new(
            art,
            self.bodies.iter().map(|(p, _)| p.clone()).collect(),
        ))
    }

    /// Latest knowledge and held shove poses outlive asynchronous art work.
    pub fn adopt_knowledge(&mut self, current: &Self) {
        self.known.clone_from(&current.known);
        self.moving.clone_from(&current.moving);
        self.shoved.clone_from(&current.shoved);
        self.refit();
    }
}
