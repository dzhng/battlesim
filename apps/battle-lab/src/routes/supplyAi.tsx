import { useSessionCatalog } from "@web/battle/catalog/context";
import { BattleView } from "../BattleView";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { gameCamera } from "../gameCamera";
import { BattleClock } from "../battleStatus";
import { durableSoldiers } from "../scenarios";

const PLAYERS = [
  { id: "blue-1", side: "blue", faction: "us", controller: "active_ai" },
  // An empty opponent lets authored wear measure the complete supply cycle.
  { id: "red-1", side: "red", faction: "eastern", controller: "passive_ai" },
];

/** A small, carded-only skirmish showing the emergency supplier path. */
export default function SupplyAi() {
  const { rules } = useSessionCatalog();
  const scenario = useBuiltScenario(rules, (wasm, rules) => {
    const fixedRules = durableSoldiers(rules);
    for (const document of fixedRules.catalog as { units?: Record<string, unknown> }[]) {
      if (document.units)
        document.units = Object.fromEntries(
          Object.entries(document.units).filter(([id]) => id.startsWith("test_")),
        );
    }
    const roster = (category: string) => ({
      factions: ["us", "eastern"],
      category,
      family_name: "Test",
      variant: "Test",
    });
    const built = JSON.parse(
      wasm.encounter_scenario(
        JSON.stringify({
          size: [2400, 900],
          fog_cell_m: 8,
          height_grid_m: 4,
          slope_cutoff_deg: 35,
          props: [],
          buildings: [],
        }),
        JSON.stringify({
          units: [],
          events: [],
          scripts: [],
          overrides: {
            units: {
              test_supply: {
                roster: roster("sup"),
                mobility: { wheeled: { offroad_kmh: 1, road_kmh: 1 } },
                capabilities: { supply: { stock: 0 } },
              },
              test_supply_heli: { roster: roster("sup") },
              test_tank: {
                roster: { ...roster("veh"), factions: ["us"] },
                body: { hull: { hp: 100 } },
                mobility: { tracked: { offroad_kmh: 130, road_kmh: 130 } },
              },
              test_rifle: { roster: { ...roster("inf"), factions: ["us"] }, mounts: [] },
            },
            weapons: {
              rifle: { damage: 0, near_miss_suppression: 0 },
              grenade: { damage: 0, near_miss_suppression: 0 },
            },
          },
        }),
        JSON.stringify(fixedRules),
      ),
    );
    built.players = PLAYERS;
    built.skirmish = JSON.parse(
      wasm.synthetic_skirmish_setup(JSON.stringify([2400, 900]), JSON.stringify(PLAYERS)),
    );
    built.skirmish.sites.entries = [
      { side: "blue", center: [50, 450], yaw: 0 },
      { side: "red", center: [350, 450], yaw: Math.PI },
    ];
    for (const objective of built.skirmish.sites.objectives) objective.center = [1800, 450];
    built.skirmish.openings = {
      "blue-1": [
        { variant: "test_supply", destination: [150, 450] },
        {
          variant: "test_tank",
          destination: [1800, 450],
          condition: { hp: 40 },
        },
      ],
      "red-1": [],
    };
    Object.assign(built.skirmish.rules, {
      credits_per_minute: 110,
      starting_minutes: 4,
      max_units: 4,
      capture_s: 1_000_000,
    });
    return JSON.stringify(built);
  });
  if (!scenario || buildFailed(scenario)) return null;
  return (
    <BattleView
      fixture="supply-ai"
      scenario={scenario}
      seed={20261117}
      player="blue-1"
      status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
      camera={{ ...gameCamera.opening(), target: [1200, 800, 0], distance: 1400 }}
    />
  );
}
