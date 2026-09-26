import os
from collections import defaultdict
HERE = os.path.dirname(os.path.abspath(__file__))
PORT = os.path.join(HERE, "port")
G = "/Users/david/dev/game/"
files = []
for root, _, fs in os.walk(os.path.join(PORT, "packages")):
    for f in fs:
        if f.endswith(".ts"):
            files.append(os.path.relpath(os.path.join(root, f), PORT))
def n(p):
    return sum(1 for _ in open(p))
pillar = {
    "frame": ["frameCamera", "world/camera", "cameraUniform", "worldDepth", "depthContract", "gpuAdmission", "gpuScope"],
    "sky+pmrem+env": ["world/sky", "physicalSky", "skyParameters", "world/pmrem.ts", "pmremSampling", "shaders/pmrem", "world/environment", "shaders/environment", "standardPbr", "dfgLut", "aerial", "physicalEnvironment", "environment/environment", "textureUpload", "math.ts"],
    "shadows": ["world/shadow", "shaders/shadow", "shadowData", "cascadePolicy", "shadowPolicy", "camera3d", "mat4"],
    "post+agx": ["world/post", "shaders/post", "postParameters"],
}
pk = defaultdict(lambda: [0, 0, 0])
pl = defaultdict(lambda: [0, 0, 0])
tot = [0, 0, 0]
for f in sorted(files):
    o = n(G + f)
    m = n(os.path.join(PORT, f))
    pkg = f.split("/")[1]
    hit = [k for k, v in pillar.items() if any(s in f for s in v)]
    assert len(hit) == 1, (f, hit)
    for acc in (pk[pkg], pl[hit[0]], tot):
        acc[0] += 1
        acc[1] += o
        acc[2] += m
for k, v in pk.items():
    print("pkg", k, "files", v[0], "source lines", v[1], "after prune", v[2])
for k, v in pl.items():
    print("pillar", k, "files", v[0], "source lines", v[1], "after prune", v[2])
print("total files", tot[0], "source lines", tot[1], "after prune", tot[2])
