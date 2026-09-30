"""Reproduce proof-only instrumentation; never edit frozen or production owners."""
from pathlib import Path
import sys,hashlib
root=Path(sys.argv[1]).resolve()
proof=root/'specs/city-maps/assets/navigation-proof/uniform-parent'
out=root/'throwaway/navigation-uniform-proof'
out.mkdir(parents=True,exist_ok=True)
base=root/'specs/city-maps/assets/navigation-proof/dense-navigation.rs'
text=base.read_text()
assert hashlib.sha256(base.read_bytes()).hexdigest()=='e387c1bcb173380360cc1db315c395bfdc3fb6569e43e8b1ddf614da02e86356'
def replace(a,b):
    global text
    assert text.count(a)==1,(a,text.count(a))
    text=text.replace(a,b)
replace('s.g[start] = 0.0;', 's.g[start] = proof_incoming();')
replace('f: h(start, self.nx),','f: s.g[start] + h(start, self.nx),')
replace('for (di, dj) in STEPS {','proof_expand();\n            for (direction, (di, dj)) in STEPS.into_iter().enumerate() {')
replace('s.parent[next] = cell as u32;','s.parent[next] = cell as u32;\n                    proof_accept(s, self.nx, cell, next, f, direction);')
text+=proof.joinpath('trace-support.rs').read_text()
out.joinpath('traced.rs').write_text(text)
out.joinpath('main.rs').write_text(proof.joinpath('runner.rs').read_text().replace('@DENSE@',str(base)))
out.joinpath('Cargo.toml').write_text(f'''[package]
name="navigation-uniform-proof"
version="0.0.0"
edition="2021"
[workspace]
[dependencies]
sim={{path="{root}/crates/sim"}}
contract={{path="{root}/crates/contract"}}
serde_json="1"
[[bin]]
name="navigation-uniform-proof"
path="main.rs"
''')
print(out/'Cargo.toml')
