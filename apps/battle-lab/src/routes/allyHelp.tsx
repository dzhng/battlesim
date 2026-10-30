import { useSessionCatalog } from "@web/battle/catalog/context";
import { BattleView } from "../BattleView";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { gameCamera } from "../gameCamera";
import { BattleClock } from "../battleStatus";
import { durableSoldiers } from "../scenarios";

// Keep the question local: the human holds the east objective under observed
// pressure, while an allied main force has reserves far from the fight.
const PLAYERS = [
  { id: "blue-1", side: "blue", faction: "us", controller: "human", human_capable: true },
  { id: "blue-2", side: "blue", faction: "us", controller: "active_ai" },
  { id: "red-1", side: "red", faction: "eastern", controller: "active_ai" },
  { id: "red-2", side: "red", faction: "eastern", controller: "active_ai" },
];

export default function AllyHelp() {
  const { rules } = useSessionCatalog();
  const scenario = useBuiltScenario(rules, (wasm, rules) => {
    // Live roster additions must not change the fixture's choices or strength scale.
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
          size: [2400, 1600],
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
          // These are test forces, with a fixed relative strength and holders.
          // Durable bodies keep the pressure in place while help travels.
          overrides: {
            units: {
              test_tank: { cost: 100, roster: roster("veh"), body: { hull: { hp: 1e6 } } },
              test_rifle: { cost: 10, roster: roster("inf") },
            },
          },
        }),
        JSON.stringify(fixedRules),
      ),
    );
    built.players = PLAYERS;
    built.skirmish = JSON.parse(
      wasm.synthetic_skirmish_setup(JSON.stringify([2400, 1600]), JSON.stringify(PLAYERS)),
    );
    // Separate roads place the small fight by the human's objective and the
    // allied reserves by their own. Every force enters through normal purchases.
    built.skirmish.sites.entries = [
      { side: "blue", center: [1800, 760], yaw: 0 },
      { side: "blue", center: [600, 12], yaw: 0 },
      { side: "red", center: [1800, 920], yaw: -Math.PI / 2 },
      { side: "red", center: [600, 1588], yaw: -Math.PI / 2 },
    ];
    Object.assign(built.skirmish.rules, { credits_per_minute: 100, starting_minutes: 10 });
    const tanks = (count: number, destination: number[]) =>
      Array.from({ length: count }, () => ({ variant: "test_tank", destination }));
    built.skirmish.openings = {
      "blue-2": tanks(6, [600, 120]),
      "red-1": tanks(2, [1800, 880]),
      "red-2": tanks(2, [600, 1500]),
    };
    return JSON.stringify(built);
  });
  if (!scenario || buildFailed(scenario)) return null;
  return (
    <BattleView
      fixture="ally-help"
      scenario={scenario}
      seed={20260925}
      player="blue-1"
      status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
      camera={{ ...gameCamera.opening(), target: [1200, 800, 0], distance: 1300 }}
    />
  );
}
