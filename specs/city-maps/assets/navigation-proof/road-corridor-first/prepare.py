from pathlib import Path
import hashlib
r=Path(__file__).resolve().parents[2];p=Path(__file__).parent
s=(r/'crates/sim/src/navigation.rs').read_text()
assert hashlib.sha256(s.encode()).hexdigest()=='c3e6b96bdbcc5bce518bb7c7904b5ae50bfa3c53ab8f2ec06628bd372b15ace4'
s=s.replace('let samples = ((length / spacing).ceil() as usize).max(1);','let samples = ((length / spacing).ceil() as usize).max(1);\n        REFINEMENT_SAMPLES.with(|v|v.set(v.get()+samples));')
s+='''
thread_local!{static REFINEMENT_SAMPLES:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
impl NavGrid {
    pub fn prototype_segment_cost(&self,a:V2,b:V2,m:&Mobility,policy:RoutePolicy)->Option<f64>{self.segment_cost(a,b,m,policy)}
    /// Experiment only: separator portals choose the corridor, fine cells certify it.
    /// A declined certificate is NOT a production Blocked or a resource verdict.
    pub fn corridor_prototype(&self, from:V2, goal:V2, m:&Mobility, policy:RoutePolicy)
        -> Result<(Vec<V2>,f64,usize),&'static str> {
        REFINEMENT_SAMPLES.with(|v|v.set(0));
        if !self.fits_at(from,m) || !self.fits_at(goal,m) {return Err("unsupported endpoint");}
        let Some(start)=self.nearest_fit(from,m,NAV_CELL_M*2.0) else {return Err("unsupported start");};
        let Some(target)=self.nearest_fit(goal,m,NAV_CELL_M*3.0) else {return Err("unsupported target");};
        let Some(cut)=self.search_cut(start,target,m) else {return Err("no separator certificate");};
        let axis_limit=if cut.vertical {self.nx}else{self.ny};
        let lo=cut.low.saturating_sub(CLEARANCE_HALO+1);
        let hi=(cut.high+CLEARANCE_HALO+1).min(axis_limit-1);
        let forward=if cut.vertical {start%self.nx<cut.at}else{start/self.nx<cut.at};
        let mut best:Option<(f64,usize,Vec<V2>)>=None;
        // No experiment candidate cap: meaningful opening width owns the work.
        for &portal in &cut.portals {
            let other=if cut.vertical {portal/self.nx}else{portal%self.nx};
            let point=|axis|if cut.vertical {cell_center(axis,other)}else{cell_center(other,axis)};
            let (a,b)=if forward {(point(lo),point(hi))}else{(point(hi),point(lo))};
            if !self.fits_at(a,m)||!self.fits_at(b,m){continue;}
            let mut cost=0.0;
            let route=vec![a,b,goal];let mut from=from;let mut valid=true;
            for &to in &route {
                let Some(c)=self.segment_cost(from,to,m,policy) else {valid=false;break;};
                cost+=c;from=to;
            }
            if valid&&cost.is_finite()&&cost>=0.0&&best.as_ref().is_none_or(|(old,id,_)|cost.total_cmp(old).then(portal.cmp(id)).is_lt()) {
                best=Some((cost,portal,route));
            }
        }
        let samples=REFINEMENT_SAMPLES.with(|v|v.get());
        best.map(|(c,_,r)|(r,c,samples)).ok_or("straight portal refinement declined")
    }
}
'''
p.joinpath('candidate.rs').write_text(s)
p.joinpath('Cargo.toml').write_text(f'''[package]
name="navigation-corridor-proof"
version="0.0.0"
edition="2021"
[workspace]
[dependencies]
sim={{path="{r}/crates/sim"}}
contract={{path="{r}/crates/contract"}}
serde_json="1"
serde={{version="1",features=["derive"]}}
[[bin]]
name="navigation-corridor-proof"
path="main.rs"
[profile.release]
opt-level=3
lto="thin"
''')
