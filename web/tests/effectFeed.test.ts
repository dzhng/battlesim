// @vitest-environment node
import { expect, test } from "vitest";
import { effectPublication } from "@apps/battle-lab/src/effectFeed";
import { LaunchTracker } from "@packages/battle-renderer/src/effects/launches";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { ObservationView } from "@web/battle/sim/observation";

test("the production observed feed launches own and enemy rounds by shooter and weapon row", () => {
  const observation = (tick: number): ObservationView =>
    ({
      tick,
      own: [
        {
          id: 1,
          kind: "rifle",
          position: [0, 0, 0],
          yaw: 0,
          memberIds: [20],
          weaponPoses: [
            { mount: 0, shots: tick >= 2 ? 1 : 0, bearing: 0, elevation: 0, operator: 20 },
          ],
        },
      ],
      identified: [
        {
          id: 7,
          kind: "tank",
          position: [100, 0, 0],
          yaw: Math.PI,
          memberIds: [],
          weaponPoses: [
            { mount: 0, shots: 0, bearing: Math.PI, elevation: 0, operator: null },
            { mount: 1, shots: tick >= 2 ? 1 : 0, bearing: Math.PI, elevation: 0, operator: null },
          ],
        },
      ],
      projectiles:
        tick === 3
          ? [
              {
                path: [
                  [0, 0, 1],
                  [1, 0, 1],
                ],
                ricochets: [],
                kind: "rifle",
                shooterMember: 20,
                hit: "none",
                impactNormal: null,
              },
            ]
          : [],
      blasts: [],
      knownProps: [],
    }) as unknown as ObservationView;
  const launches = new LaunchTracker();
  expect(launches.note(effectPublication(observation(1), "blue", UNITS), false)).toEqual([]);
  const shots = [
    ...launches.note(effectPublication(observation(2), "blue", UNITS), false),
    ...launches.note(effectPublication(observation(3), "blue", UNITS), false),
  ];
  expect(shots.map((shot) => [shot.shooter, shot.kind])).toEqual([
    [15, UNITS.type("tank").mounts[1].weapons[0]],
    [2, UNITS.type("rifle").mounts[0].weapons[0]],
  ]);
});
