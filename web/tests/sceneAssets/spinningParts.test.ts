// Validation of parts that spin: what rolls on the ground turns under its own
// node, and a helicopter's airframe is sized without its rotor disc.
import { expect, test } from "vitest";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import { AUTHORITY, HELI_AUTHORITY, TOLERANCES } from "./synthetic";
import { cartGlb, heliGlb, skidGlb } from "./syntheticSpinning";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };

test("what rolls on the ground must spin: tyres under wheel nodes; skids or a belly rest as they are", async () => {
  const judged = async (glb: Uint8Array) =>
    (
      await validateAppearance(
        {
          name: "model",
          entry: { unit: "vehicle", source: "model.glb", basis_yaw_deg: 0 },
          files: { "model.glb": glb },
        },
        context,
      )
    ).findings.map((f) => f.code);
  expect(await judged(skidGlb())).not.toContain("nodes.missing");
  expect(await judged(cartGlb(true))).not.toContain("nodes.missing");
  expect(await judged(cartGlb(false))).toContain("nodes.missing");
});

test("a helicopter fits its hull without its rotors: their disc is not its airframe's size", async () => {
  const findings = (
    await validateAppearance(
      {
        name: "heli",
        entry: { unit: "vehicle", source: "heli.glb", basis_yaw_deg: 0 },
        files: { "heli.glb": heliGlb() },
      },
      { ...context, authority: HELI_AUTHORITY },
    )
  ).findings;
  expect(findings.map((f) => f.message)).toEqual([]);
});
