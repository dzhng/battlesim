// Cook-offs: a tracked and a wheeled hull die, each wreck stays, and the
// debris each throws lies wider than its wreck for a while, then sinks away.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, until, snapshot, openBattle, aim, presented } from "./_lab.mjs";

const demo = (page, name) => lab(page, (n) => window.__lab.route.demo(n), name);
const pieces = (page) => lab(page, () => window.__lab.route.cookOffPieces());

/** The wreck of `kind` the side knows, or undefined. */
const wreckOf = (o, kind) => o?.knownProps.find((p) => p.wreckOf === kind);

export async function run(ctx) {
  const game = JSON.parse(await readFile(new URL("../../fixtures/game.json", import.meta.url)));
  const { delay_s, debris } = game.presentation.effects.cook_off;
  const perS = game.tick_hz;
  const page = await openBattle(ctx);
  /** Both wrecks framed close, each its own shot, as evidence `stage`. */
  const close = async (stage) => {
    for (const [name, where, distance] of [
      ["tank", [455, 392], 26],
      ["jeep", [425, 395], 20],
    ]) {
      await aim(page, where, { distance, pitch: 0.7, yaw: -2.3 });
      await presented(page);
      await snapshot(ctx, page, `cook-off-${stage}-${name}.png`);
    }
  };
  await advance(page, 2);
  await close("before");

  // Each order kills one hull: the tank, then the jeep.
  await demo(page, "Destroy the red tank");
  let o = await until(page, (f) => !!wreckOf(f, "test_tank"), 900, 3);
  const tankDied = o?.tick ?? null;
  await demo(page, "Destroy the red jeep");
  o = await until(page, (f) => !!wreckOf(f, "test_jeep"), 900, 3);
  const jeepDied = o?.tick ?? null;
  ctx.check(
    "the tank and the jeep each leave a wreck",
    tankDied !== null && jeepDied !== null,
    JSON.stringify({ tankDied, jeepDied, props: o?.knownProps }),
  );
  if (tankDied === null || jeepDied === null) return;

  /** The debris pieces drawn now, by wreck appearance: how far each is sunk. */
  const debrisNow = async () =>
    Object.fromEntries(
      (await pieces(page)).filter((p) => p.state === "debris").map((p) => [p.appearance, p.sunk]),
    );
  const at = async (tick) => {
    const now = (await obs(page)).tick;
    if (tick > now) await advance(page, tick - now);
    await presented(page);
  };

  // The death: the jeep's fire still burning, both debris fields down.
  await at(jeepDied + Math.round((delay_s + 1.5) * perS));
  const death = await debrisNow();
  await close("death");
  ctx.check(
    "each watched death throws its debris, lying on the ground",
    death.test_tank_wreck === 0 && death.test_jeep_wreck === 0,
    JSON.stringify(death),
  );

  // Late in the hold: still lying there.
  await at(tankDied + Math.round((delay_s + debris.hold_s - 1) * perS));
  const held = await debrisNow();
  await close("hold");
  ctx.check(
    "debris lies still through its hold",
    held.test_tank_wreck === 0 && held.test_jeep_wreck === 0,
    JSON.stringify(held),
  );

  // Midway through the tank's fade: sinking, never rising.
  await at(tankDied + Math.round((delay_s + debris.hold_s + debris.fade_s / 2) * perS));
  const fading = await debrisNow();
  await close("fading");
  ctx.check("then it sinks into the ground", fading.test_tank_wreck > 0, JSON.stringify(fading));

  // After both fades: no debris drawn, both wrecks still there.
  await at(jeepDied + Math.round((delay_s + debris.hold_s + debris.fade_s + 1) * perS));
  const gone = await debrisNow();
  o = await obs(page);
  await close("after");
  ctx.check(
    "after the fade the debris is gone and the wrecks stay",
    Object.keys(gone).length === 0 && !!wreckOf(o, "test_tank") && !!wreckOf(o, "test_jeep"),
    JSON.stringify({ gone, wrecks: o.knownProps.filter((p) => p.wreckOf) }),
  );
}
