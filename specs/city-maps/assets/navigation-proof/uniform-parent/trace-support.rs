#[derive(Default)]
struct ProofTrace {
    incoming: f64,
    rectangle: [usize; 4],
    expansions: usize,
    events: Vec<serde_json::Value>,
}
thread_local! { static PROOF_TRACE: std::cell::RefCell<ProofTrace> = Default::default(); }
fn proof_incoming() -> f64 {
    PROOF_TRACE.with(|p| p.borrow().incoming)
}
fn proof_expand() {
    PROOF_TRACE.with(|p| p.borrow_mut().expansions += 1);
}
fn proof_chain(s: &Scratch, mut cell: usize) -> Vec<usize> {
    let mut path = vec![cell];
    while s.parent[cell] as usize != cell {
        cell = s.parent[cell] as usize;
        path.push(cell);
        assert!(path.len() <= s.parent.len(), "tracer saw parent cycle");
    }
    path.reverse();
    path
}
fn proof_accept(s: &Scratch, nx: usize, cell: usize, next: usize, f: f64, direction: usize) {
    PROOF_TRACE.with(|p| {
        let mut p = p.borrow_mut();
        let [x0, y0, x1, y1] = p.rectangle;
        let (x, y) = (next % nx, next / nx);
        if x >= x0 && x <= x1 && y >= y0 && y <= y1 && (x == x0 || x == x1 || y == y0 || y == y1) {
            let event = serde_json::json!({"expansion":p.expansions,"predecessor":cell,
                "predecessor_queue_f_bits":format!("{:016x}",f.to_bits()),
                "predecessor_current_g_bits":format!("{:016x}",s.g[cell].to_bits()),
                "neighbor_ordinal":direction,"cell":next,
                "accepted_g_bits":format!("{:016x}",s.g[next].to_bits()),
                "parent_chain":proof_chain(s,next)});
            p.events.push(event);
        }
    });
}
impl NavGrid {
    pub fn proof_run(
        &mut self,
        from: V2,
        goal: V2,
        m: &Mobility,
        policy: RoutePolicy,
        incoming: f64,
        rectangle: [usize; 4],
    ) -> (Plan, f64, Vec<usize>, Vec<serde_json::Value>) {
        PROOF_TRACE.with(|p| {
            *p.borrow_mut() = ProofTrace {
                incoming,
                rectangle,
                ..Default::default()
            }
        });
        let plan = self.plan(from, goal, m, policy);
        let target = self.nearest_fit(goal, m, NAV_CELL_M * 3.0).unwrap();
        let g = self.scratch.g[target];
        let chain = proof_chain(&self.scratch, target);
        let events = PROOF_TRACE.with(|p| std::mem::take(&mut p.borrow_mut().events));
        (plan, g, chain, events)
    }
}
