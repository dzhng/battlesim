// @vitest-environment node
// The npm `math` package is the one owner of TypeScript vector and matrix math
// (the user's call, battle-look decisions.md): no source file hand-rolls a vector or
// matrix type or helper beside it.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { basename, join, relative } from "node:path";
import { expect, test } from "vitest";

const ROOT = new URL("../../", import.meta.url).pathname;
const SCANNED = ["packages", "apps", "web/src", "web/tests"];
const THIS_FILE = "web/tests/mathOwner.test.ts";

/** Vector and matrix helpers `math` owns (`vec3.cross`, `mat4.lookAt`, …),
 *  optionally suffixed with a dimension (`dot3`, `lerp3`). */
const HELPER =
  "(?:cross|dot|normali[sz]e|lerp|length|distance|invert|multiply|transpose|lookAt|identity|perspective\\w*|ortho\\w*|transform(?:Point|Vec)\\w*|clamp(?:01)?)[234]?";
/** A declaration of one: a function, or a const/let arrow function. */
const DECLARATION = new RegExp(
  `(?:function\\s+(${HELPER})\\s*[<(]|(?:const|let)\\s+(${HELPER})\\s*=\\s*(?:\\([^)]*\\)|\\w+)\\s*(?::[^=]+)?=>)`,
  "g",
);
/** A vector, matrix or quaternion type declared outside `math`. */
const TYPE = /\btype\s+(?:Vec[234]|Mat[234]|Quat)\s*=/g;
/** Files named for a vector or matrix module. */
const MODULE_FILE = /^(?:vec[234]|mat[234]|quat|vector|matrix)\.tsx?$/;

/** Helpers `math` lacks, kept with a reason. */
const ALLOWED = new Set([
  // Reverse-Z projections: `math` builds only forward-Z ones.
  "packages/renderer-core/src/camera3d.ts:perspectiveReverseZ",
  "packages/renderer-core/src/camera3d.ts:orthographicReverseZ",
]);

function sources(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "wasm" || name.startsWith(".")) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...sources(path));
    else if (/\.tsx?$/.test(name)) out.push(path);
  }
  return out;
}

/** Every hand-rolled vector or matrix declaration in `text`, by name. */
function handRolled(text: string): string[] {
  const names = [...text.matchAll(DECLARATION)].map((m) => m[1] ?? m[2]);
  return [...names, ...[...text.matchAll(TYPE)].map((m) => m[0])];
}

test("the detector recognises the migrated helpers", () => {
  // The shapes the old hand-rolled owners took, so the grep cannot rot silent.
  expect(handRolled("function cross(a: readonly number[], b: readonly number[]) {")).toEqual([
    "cross",
  ]);
  expect(handRolled("export function lookAt(eye: Vec3, target: Vec3): Mat4 {")).toEqual(["lookAt"]);
  expect(handRolled("function lerp3(a: Vec3, b: Vec3, t: number): Vec3 {")).toEqual(["lerp3"]);
  expect(handRolled("const clamp = (v: number, lo: number, hi: number) => v")).toEqual(["clamp"]);
  expect(handRolled("export type Mat4 = Float32Array;")).toEqual(["type Mat4 ="]);
  expect(handRolled("const length = Math.hypot(dx, dy);")).toEqual([]);
});

test("no vector or matrix math is hand-rolled outside the math package", () => {
  const offenders: string[] = [];
  for (const path of SCANNED.flatMap((dir) => sources(join(ROOT, dir)))) {
    const file = relative(ROOT, path);
    if (file === THIS_FILE) continue;
    if (MODULE_FILE.test(basename(path))) offenders.push(`${file}: module file`);
    for (const name of handRolled(readFileSync(path, "utf8")))
      if (!ALLOWED.has(`${file}:${name}`)) offenders.push(`${file}: ${name}`);
  }
  expect(offenders).toEqual([]);
});
