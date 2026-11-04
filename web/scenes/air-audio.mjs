// The air alarm (slice 11), heard by blue. Blue's marine squad sees no
// aircraft until a J-10 flies into its view: then blue hears one air alarm
// and the jet's approach. (Its eyes are on foot: a tank's crew is blind to
// high air, D48.) On the 3 km jet field the J-10 flies its scripted legs
// rather than circling the small air map's middle. While the J-10 stays seen no second alarm sounds. It flies
// off out of sight, ending the episode, and back: the alarm sounds once more.
// The battle runs live, as a player hears it, after a first click. The tests
// behind the episode rule and the sighting:
//   crates/presentation/tests/sound.rs::first_high_air_entry_raises_one_alarm_until_the_episode_ends
//   crates/sim/tests/radar.rs::each_bands_sight_is_the_units_own_and_zero_is_blind
//
// Not shown: distance attenuation and side-local hearing, which the
// presentation's sound tests hold (crates/presentation/tests/sound.rs::
// a_cannon_sounds_near_close_and_far_away_quieter_but_never_below_the_floor,
// an_unseen_enemy_is_heard_only_by_direction).
import { gpuWarnings, obs, lab } from "./_lab.mjs";
import { openAir, shoot } from "./_air.mjs";
import { unitType, audioMix } from "./_units.mjs";

const ENEMY_JET = "eastern_j_10_vigorous_dragon_j_10c";
const { air_alarm, vehicles } = audioMix;
const ALARM = air_alarm.sound;
/** The engine the J-10 sounds with: its class's row. */
const ENGINE = vehicles[unitType(ENEMY_JET).vehicle_class].engine;

const heard = async (page) => (await lab(page, () => window.__lab.route.sound()))?.bySound ?? {};

/** Wait, live, until the J-10 is identified (`seen`) or not; then a second
 *  for the frame to hear it. Returns the tick. */
async function until(page, seen, timeoutS) {
  const handle = await page.waitForFunction(
    ({ seen, kind }) => {
      const o = window.__lab.route.observation();
      const is = !!o?.identified.some((u) => u.kind === kind);
      return is === seen && o.tick;
    },
    { seen, kind: ENEMY_JET },
    { timeout: timeoutS * 1000, polling: 50 },
  );
  const tick = await handle.jsonValue();
  await page.waitForTimeout(1500);
  return tick;
}

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  ctx.check(
    "the J-10 starts unseen",
    !(await obs(page)).identified.some((u) => u.kind === ENEMY_JET),
  );
  // The first click is the gesture the sound waits for.
  await page.click('[data-testid="battle-panel"] header');
  await lab(page, () => window.__lab.route.resume());
  await page
    .waitForFunction(() => window.__lab.route.sound()?.running, undefined, { timeout: 30000 })
    .catch(() => {});
  const before = await heard(page);
  ctx.check("no alarm before any aircraft is seen", !before[ALARM], JSON.stringify(before));

  const first = await until(page, true, 60);
  const entry = await heard(page);
  ctx.check(
    "one alarm and the jet's approach when the J-10 comes into view",
    entry[ALARM] === 1 && entry[ENGINE] > 0,
    JSON.stringify({ tick: first, entry }),
  );
  // A picture waits for the frame to settle on the latest tick: held still for it.
  await lab(page, () => window.__lab.route.pause());
  await shoot(ctx, page, "j-10-seen");
  await lab(page, () => window.__lab.route.resume());
  const lost = await until(page, false, 60);
  const during = await heard(page);
  ctx.check(
    "no second alarm while it stays seen",
    during[ALARM] === 1,
    JSON.stringify({ lost, during }),
  );
  const again = await until(page, true, 60);
  const back = await heard(page);
  ctx.check(
    "seen again after it was lost, a new episode raises the alarm once more",
    back[ALARM] === 2,
    JSON.stringify({ again, back }),
  );
  await lab(page, () => window.__lab.route.pause());
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
