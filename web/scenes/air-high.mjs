// HighAir motion and the vision boundary (slice 02): an F-16 flies across
// the air field over the village, the tower and the wood. Blue's fog never
// clears (HighAir never reveals Ground) and the T-72 in the village is never
// identified, though the jet identifies the enemy Mi-8 (LowAir) and J-10
// (HighAir); it holds the air height and stays on the map.
//
// Not shown, because the simulation does not do it: fixed-wing aircraft fly
// at the helicopters' cruise height (`air.cruise_agl_m`), not a separate
// ~1,000 m HighAir height, and stop and hover at a move's goal instead of
// circling it.
import { gpuWarnings, aim } from './_lab.mjs';
import {
  encounter,
  openAir,
  play,
  own,
  onMap,
  surface,
  CRUISE,
  shoot,
  visibleCells,
  field,
} from './_air.mjs';

const [JET, TANK, HELI, ENEMY_JET] = encounter('high-air', 'air-high').units.map(u => u.kind);
const goal = encounter('high-air', 'air-high').scripts[0].order.goal;

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let fog = 0;
  let tankSeen = false;
  const seen = new Set();
  const heights = [];
  let offMap = null;
  let k = 0;
  // Until the jet reaches its goal across the field, with a generous cap.
  const last = await play(page, 3600, o => {
    fog = Math.max(fog, visibleCells(o));
    for (const u of o.identified) seen.add(u.kind);
    if (o.identified.some(u => u.kind === TANK)) tankSeen = true;
    const jet = own(o, JET)[0];
    if (jet && !onMap(jet.position, field('air-high'))) offMap = jet.position;
    if (jet && k++ % 30 === 0) heights.push(jet.position);
    return !!jet && Math.hypot(jet.position[0] - goal[0], jet.position[1] - goal[1]) < 10;
  });
  ctx.check("blue's fog never clears while only the jet flies", fog === 0, `${fog} cells`);
  ctx.check('the jet never identifies the tank on the ground', !tankSeen);
  ctx.check(
    'the jet identifies the enemy aircraft in both air layers',
    seen.has(HELI) && seen.has(ENEMY_JET),
    JSON.stringify([...seen]),
  );
  ctx.check('the jet stays on the map', offMap === null, JSON.stringify(offMap));
  const lifts = [];
  for (const p of heights) lifts.push(p[2] - (await surface(page, p)));
  ctx.check(
    'it holds the air height over the ground all the way',
    lifts.length > 5 && lifts.every(l => Math.abs(l - CRUISE) < 3),
    JSON.stringify(lifts.map(l => l.toFixed(1))),
  );
  const jet = own(last, JET)[0];
  ctx.check(
    'it reaches its goal past the village',
    !!jet && Math.hypot(jet.position[0] - goal[0], jet.position[1] - goal[1]) < 10,
    JSON.stringify(jet?.position),
  );
  // The jet over the wood at its goal, the village and the tower behind it.
  if (jet)
    await aim(
      page,
      jet.position,
      { distance: 120, pitch: 0.3, yaw: -Math.PI / 2 - 0.5 },
      { onGround: false },
    );
  await shoot(ctx, page, 'jet-over-the-village');
  ctx.check('no GPU validation warnings', warnings.length === 0, warnings.join('\n'));
}
