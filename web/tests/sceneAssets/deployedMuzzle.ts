import type { Finding } from "@packages/scene-assets/src/finding.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import type { MountRow } from "@packages/scene-assets/src/units.ts";
import { AUTHORITY, TOLERANCES, syntheticUnits, truckGlb } from "./synthetic";

const launcher: MountRow = {
  id: "launcher",
  name: "Launcher",
  weapons: ["launcher"],
  squad: false,
  special: false,
  turret: false,
  on: null,
  pivot_m: [0, 0, 0],
  muzzle_m: [100, 0, 0],
};

export async function deployedMuzzleGolden(): Promise<Finding[]> {
  const result = await validateAppearance(
    {
      name: "truck",
      entry: {
        unit: "vehicle",
        source: "truck.glb",
        basis_yaw_deg: 0,
        mounts: { launcher: "hull" },
      },
      files: { "truck.glb": truckGlb() },
    },
    {
      tolerances: TOLERANCES,
      authority: { ...AUTHORITY, units: syntheticUnits({}, { mounts: [launcher] }) },
    },
  );
  return result.findings;
}
