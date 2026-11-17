use super::*;
use sim::publication::reader::FallenBody;

impl PropPopulation {
    pub fn set_fallen(&mut self, fallen: &[FallenBody]) -> bool {
        let next = fallen
            .iter()
            .filter_map(|f| {
                let prop = f.prop?;
                let map = self.by_id.get(&(prop as u64)).copied()?;
                let body = self.bodies[map].0.body.clone();
                Some(KnownBody {
                    authored: Some(prop),
                    id: None,
                    kind: body.kind.clone(),
                    destroyed: false,
                    body: Some(PropBox {
                        half: [body.half[0], body.half[1], body.half[2].min(0.2)],
                        ..body
                    }),
                })
            })
            .collect();
        if next == self.fallen {
            return false;
        }
        self.fallen = next;
        self.refit();
        true
    }
}
