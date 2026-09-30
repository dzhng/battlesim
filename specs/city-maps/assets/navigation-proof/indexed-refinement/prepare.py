from pathlib import Path
import hashlib
r=Path(__file__).resolve().parents[2];p=Path(__file__).parent
s=(r/'crates/sim/src/navigation.rs').read_text()
assert hashlib.sha256(s.encode()).hexdigest()=='c3e6b96bdbcc5bce518bb7c7904b5ae50bfa3c53ab8f2ec06628bd372b15ace4'
s+='\ninclude!("physical.rs");\nimpl NavGrid{pub fn reference_segment_cost(&self,a:V2,b:V2,m:&Mobility,p:RoutePolicy)->Option<f64>{self.segment_cost(a,b,m,p)}}\n'
p.joinpath('candidate.rs').write_text(s)
p.joinpath('Cargo.toml').write_text(f'''[package]
name="navigation-indexed-proof"
version="0.0.0"
edition="2021"
[workspace]
[dependencies]
sim={{path="{r}/crates/sim"}}
contract={{path="{r}/crates/contract"}}
serde_json="1"
serde={{version="1",features=["derive"]}}
[[bin]]
name="navigation-indexed-proof"
path="main.rs"
[profile.release]
opt-level=3
lto="thin"
''')
