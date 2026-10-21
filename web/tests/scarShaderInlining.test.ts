// @vitest-environment node
import { describe, expect, it } from "vitest";
import tgpu from "typegpu";
import { groundScars, groundScarsSeen } from "../../packages/battle-renderer/src/frame/groundScars";

/** How many copies of each function holding a data-dependent loop (`loop`,
 *  `while`: a probe or a search, whose trip count no compiler knows) a
 *  compiler that inlines every call, as DXC does for D3D12, makes from
 *  `entry` in resolved WGSL. */
function inlinedCopies(source: string, entry: string) {
  const wgsl = source.replace(/\/\/[^\n]*/g, "");
  const bodies = new Map<string, string>();
  for (const head of wgsl.matchAll(/\bfn\s+(\w+)\s*\(/g)) {
    let at = wgsl.indexOf("{", head.index! + head[0].length);
    const start = at;
    for (let depth = 0; ; at++) {
      if (wgsl[at] === "{") depth++;
      else if (wgsl[at] === "}" && --depth === 0) break;
    }
    bodies.set(head[1], wgsl.slice(start, at + 1));
  }
  const copies = new Map<string, number>();
  const visit = (name: string, times: number) => {
    copies.set(name, (copies.get(name) ?? 0) + times);
    const body = bodies.get(name)!;
    for (const callee of bodies.keys()) {
      const sites = body.match(new RegExp(`\\b${callee}\\s*\\(`, "g"))?.length ?? 0;
      if (sites && callee !== name) visit(callee, times * sites);
    }
  };
  visit(entry, 1);
  return new Map([...copies].filter(([name]) => /\b(loop|while)\b/.test(bodies.get(name)!)));
}

// A data-dependent loop cannot be unrolled or folded away, so every inlined
// copy is code the shader compiler and then the driver optimise again. The
// scar lookup's probe and search, once copied per tap, per corner and per
// view step, took the terrain pipeline 25 s and the grass build 13 s to
// compile on Windows (D3D12, DXC): each compiles once per shader.
describe("the side's scar sample, as a shader compiles it", () => {
  for (const [entry, fn] of [
    ["groundScarsSeen", groundScarsSeen],
    ["groundScars", groundScars],
  ] as const)
    it(`inlines each data-dependent loop of ${entry} once`, () => {
      const wgsl = tgpu.resolve([fn]);
      const loops = inlinedCopies(wgsl, entry);
      expect(loops.size).toBeGreaterThan(0);
      expect(Object.fromEntries([...loops].filter(([, n]) => n > 1))).toEqual({});
    });
});
