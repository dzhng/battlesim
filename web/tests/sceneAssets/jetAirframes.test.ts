import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { deriveRosterJets, withAirframes } from "@packages/scene-assets/src/jetAirframes.ts";

const repo = join(import.meta.dirname, "../../..");
const LFS = new TextEncoder().encode("version https://git-lfs");

/** A repo file; an unpulled LFS pointer fails by name, not as bad art. */
function read(path: string): Uint8Array {
  const bytes = readFileSync(join(repo, path));
  if (LFS.every((b, i) => bytes[i] === b))
    throw new Error(`${path} is an unpulled LFS pointer: run git lfs pull`);
  return bytes;
}

const presentation = () => readFileSync(join(repo, "fixtures/presentation.json"), "utf8");

test("the shipped airframes are what the roster's source art gives (rerun `bun run jet-airframes`)", () => {
  const derived = deriveRosterJets(read, (dir) => readdirSync(join(repo, dir)));
  expect(Object.keys(derived).length).toBeGreaterThan(1);
  const text = presentation();
  expect(withAirframes(text, derived)).toBe(text);
}, 120_000);

test("every shipped airframe names an engine look, and only afterburning engines have a flame", () => {
  const style = JSON.parse(presentation()).effects.jet_exhaust;
  const airframes = Object.entries(style.airframes) as [string, { engine: string }][];
  for (const [name, a] of airframes) expect(style.engines[a.engine], name).toBeDefined();
  const dry = airframes.filter(([, a]) => !style.engines[a.engine].flame).map(([n]) => n);
  // The A-10's TF34s and the Su-25's R-195s have no afterburner.
  expect(dry.some((n) => n.includes("a_10"))).toBe(true);
  expect(dry.some((n) => n.includes("su_25"))).toBe(true);
  expect(dry.length).toBeLessThan(airframes.length);
});
