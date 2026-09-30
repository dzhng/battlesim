from pathlib import Path
import hashlib
root=Path(__file__).resolve().parents[2]
source=(root/'crates/sim/src/navigation.rs').read_text()
assert hashlib.sha256(source.encode()).hexdigest()=='c3e6b96bdbcc5bce518bb7c7904b5ae50bfa3c53ab8f2ec06628bd372b15ace4'
Path(__file__).with_name('candidate.rs').write_text(source+'\ninclude!("physical.rs");\nimpl NavGrid {pub fn reference_segment_cost(&self,a:V2,b:V2,m:&Mobility,p:RoutePolicy)->Option<f64>{self.segment_cost(a,b,m,p)}}\n')
