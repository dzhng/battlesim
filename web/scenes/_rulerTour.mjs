import {
  lab,
  obs,
  until,
  snapshot,
  openBattle,
  aim,
  groundCss,
  pointerOffCanvas,
} from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { unitType, weapons, streetMap, visuals } from "./_units.mjs";
import { stageStreetMoves } from "./_streetStaging.mjs";
const CAMERA = visuals.camera;
/** The range ruler (Space held with a selection) and right-clicking a
 *  contact's area to attack it. */
export async function rulerTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  let o = await obs(page);
  const rifle = o.own.find((u) => u.kind === "test_rifle");
  const tank = o.own.find((u) => u.kind === "test_tank");
  const ruler = () => lab(page, () => window.__lab.route.ruler());
  /** The ruler's text as shown: the distance, and each tick's label. */
  const shownText = () =>
    lab(page, () => {
      const layer = document.querySelector("[data-testid=range-ruler]");
      const visible = (e) => e && getComputedStyle(e).display !== "none";
      return {
        shown: visible(layer),
        distance: layer?.querySelector(".rr-distance")?.textContent ?? null,
        ticks: [...(layer?.querySelectorAll(".rr-tick") ?? [])]
          .filter(visible)
          .map((t) => t.textContent),
      };
    });
  // Read the ruler against the carried weapon rows, without recomputing its
  // muzzle selection or constructing a second ruler. Equal ranges share a mark.
  const marksAgree = (r, unit, text) => {
    if (!r || r.unit !== unit.id) return false;
    const mounts = unitType(unit.kind).mounts;
    const rows = mounts
      .flatMap((mount) =>
        mount.weapons.map((id) => ({
          ...weapons[id],
          caption: mount.weapons.length === 1 ? weapons[id].name : mount.name,
        })),
      )
      .filter((row) => row.range_m > 0);
    const reaches = new Set(rows.map((row) => row.range_m));
    return (
      r.marks.length === reaches.size &&
      r.marks.every((mark, i) => {
        const captions = new Set(
          rows.filter((row) => row.range_m === mark.range_m).map((row) => row.caption),
        );
        return (
          captions.size > 0 &&
          mark.names.length === captions.size &&
          mark.names.every((name) => captions.has(name)) &&
          new Set(mark.names).size === mark.names.length &&
          (i === 0 || r.marks[i - 1].range_m < mark.range_m) &&
          mark.inRange === r.distance_m <= mark.range_m &&
          (mark.inRange
            ? mark.along_m === null
            : mark.along_m > 0 &&
              mark.along_m < Math.hypot(r.to[0] - r.from[0], r.to[1] - r.from[1]))
        );
      }) &&
      text.ticks.length === r.marks.filter((mark) => !mark.inRange).length &&
      r.marks
        .filter((mark) => !mark.inRange)
        .every((mark) =>
          text.ticks.includes(`${mark.names.join(" · ").toUpperCase()} ${mark.range_m} m`),
        )
    );
  };
  const frames = async () => {
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
  };

  // Default camera: the rifle squad and the tank selected, the cursor on the
  // ground 40 m past the squad, away from the tank.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const away = Math.atan2(
    rifle.position[1] - tank.position[1],
    rifle.position[0] - tank.position[0],
  );
  const near = [rifle.position[0] + Math.cos(away) * 40, rifle.position[1] + Math.sin(away) * 40];
  const mid = [(rifle.position[0] + near[0]) / 2, (rifle.position[1] + near[1]) / 2];
  await aim(page, mid, { distance: CAMERA.default.distance, pitch: 0.85, yaw: CAMERA.default.yaw });
  await frames();
  let css = await groundCss(page, near);
  await page.mouse.move(css[0], css[1]);
  await frames();
  ctx.check("without Space, no ruler", (await ruler()) === null && !(await shownText()).shown);
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await frames();
  let r = await ruler();
  let text = await shownText();
  ctx.check(
    "with Space held, the ruler runs from the selected unit nearest the cursor, measuring muzzle to aim point",
    r?.unit === rifle.id &&
      Math.hypot(r.to[0] - near[0], r.to[1] - near[1]) < 0.5 &&
      Math.abs(r.distance_m - 40) < 0.5,
    JSON.stringify(r),
  );
  ctx.check(
    "the ruler's readout is its distance in metres alone, with ticks for carried weapons that fall short",
    text.shown && text.distance === `${Math.round(r.distance_m)} m` && marksAgree(r, rifle, text),
    JSON.stringify(text),
  );
  await snapshot(ctx, page, "ruler-default-1920x1080.png");
  // The ruler is paint: the frame with it less the frame with the pointer
  // off the canvas (no ruler) is lit along the line.
  const withRuler = decode(await snapshot(ctx, page, "ruler-default-on.png"));
  await pointerOffCanvas(page);
  await frames();
  const offCanvas = await ruler();
  const without = decode(await snapshot(ctx, page, "ruler-default-off.png"));
  const quarter = await groundCss(page, [
    rifle.position[0] + Math.cos(away) * 30,
    rifle.position[1] + Math.sin(away) * 30,
  ]);
  let lit = 0;
  for (let dy = -4; dy <= 4; dy++)
    for (let dx = -4; dx <= 4; dx++) {
      const a = pixel(withRuler, quarter[0] + dx, quarter[1] + dy);
      const b = pixel(without, quarter[0] + dx, quarter[1] + dy);
      if (Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2]) > 40) lit++;
    }
  ctx.check(
    "the ruler is painted along its line, and goes with the pointer off the ground",
    offCanvas === null && lit >= 4,
    JSON.stringify({ lit, offCanvas }),
  );

  // The tank alone, the cursor 40 m off it: the line leaves its marker's
  // circle at the border, as it leaves the squad's (evidence only).
  await lab(page, (ids) => window.__lab.route.select(ids), [tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const tankNear = [tank.position[0] - Math.cos(away) * 40, tank.position[1] - Math.sin(away) * 40];
  await aim(page, [(tank.position[0] + tankNear[0]) / 2, (tank.position[1] + tankNear[1]) / 2], {
    distance: CAMERA.default.distance,
    pitch: 0.85,
    yaw: CAMERA.default.yaw,
  });
  await frames();
  css = await groundCss(page, tankNear);
  await page.mouse.move(css[0], css[1]);
  await frames();
  r = await ruler();
  ctx.check("with the tank alone selected, the ruler runs from the tank", r?.unit === tank.id);
  await snapshot(ctx, page, "ruler-tank-near-1920x1080.png");

  // Far camera: the squad alone, the cursor 750 m off.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  // Toward the map's middle, so the far points stay on the map.
  const [mx, my] = streetMap.size.map((v) => v / 2);
  const inward = (u) => Math.atan2(my - u.position[1], mx - u.position[0]);
  const far = [
    rifle.position[0] + Math.cos(inward(rifle)) * 750,
    rifle.position[1] + Math.sin(inward(rifle)) * 750,
  ];
  const farMid = [(rifle.position[0] + far[0]) / 2, (rifle.position[1] + far[1]) / 2];
  await aim(page, farMid, { distance: 1100, pitch: 0.85, yaw: CAMERA.default.yaw });
  await frames();
  css = await groundCss(page, far);
  await page.mouse.move(css[0], css[1]);
  await frames();
  r = await ruler();
  text = await shownText();
  ctx.check(
    "at a far cursor, every carried reach has its range and label",
    r?.unit === rifle.id &&
      marksAgree(r, rifle, text) &&
      Math.abs(r.distance_m - 750) < 2 &&
      text.ticks.every((t) => / \d+ m$/.test(t)),
    JSON.stringify({ r, text }),
  );
  await snapshot(ctx, page, "ruler-far-1920x1080.png");
  // Both again, the cursor 750 m out from the tank: the tank is nearer.
  // Every carried reach is represented, independent of its current loadout.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const tankFar = [
    tank.position[0] + Math.cos(inward(tank)) * 750,
    tank.position[1] + Math.sin(inward(tank)) * 750,
  ];
  await aim(page, [(tank.position[0] + tankFar[0]) / 2, (tank.position[1] + tankFar[1]) / 2], {
    distance: 1400,
    pitch: 0.85,
    yaw: CAMERA.default.yaw,
  });
  await frames();
  css = await groundCss(page, tankFar);
  if (css) await page.mouse.move(css[0], css[1]);
  await frames();
  r = await ruler();
  text = await shownText();
  ctx.check(
    "from the tank at 750 m, the ruler groups all carried weapons by reach",
    marksAgree(r, tank, text),
    JSON.stringify({ r, text }),
  );
  await snapshot(ctx, page, "ruler-tank-far-1920x1080.png");
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  await frames();
  ctx.check(
    "releasing Space removes the ruler",
    (await ruler()) === null && !(await shownText()).shown,
  );

  // A contact's area: right-click it with an armed unit selected. Scout
  // along the southern forest edge until their own sight identifies an enemy,
  // then withdraw beyond that sight. Admission alone proves neither journey.
  o = await obs(page);
  const scoutId = o.own.find((u) => u.kind === "test_recon").id;
  await stageStreetMoves(
    ctx,
    page,
    [
      {
        kind: "move",
        units: [scoutId],
        gesture: 2900,
        goal: [450, 620],
        route: "shortest",
      },
      {
        order: {
          kind: "move",
          units: [scoutId],
          gesture: 2901,
          goal: [550, 910],
          route: "shortest",
        },
        queued: true,
      },
    ],
    "ruler scout approach",
  );
  o = await until(
    page,
    (x) => {
      const scouts = x.own.find((u) => u.id === scoutId);
      return scouts && x.identified.some((e) => scouts.sees.includes(e.id));
    },
    30 * 180,
    15,
  );
  const scouts = o?.own.find((u) => u.id === scoutId);
  const from = (e) =>
    Math.hypot(e.position[0] - scouts.position[0], e.position[1] - scouts.position[1]);
  const seen = o?.identified.filter((e) => scouts.sees.includes(e.id)) ?? [];
  const farthest = seen.reduce((a, b) => (!a || from(b) > from(a) ? b : a), null);
  const back = farthest ? unitType("test_recon").sensors.ground_m - from(farthest) + 40 : 0;
  ctx.check(
    "staging: the scouts hold an enemy near the edge of their sight to lose",
    back > 0 && back < 120,
    JSON.stringify({
      tick: o?.tick,
      scouts: scouts?.position,
      identified: seen.map((e) => [e.kind, Math.round(from(e))]),
    }),
  );
  if (!farthest) throw new Error("ruler scouts never identified an enemy");
  await stageStreetMoves(
    ctx,
    page,
    [
      {
        kind: "move",
        units: [scoutId],
        gesture: 2902,
        goal: [
          scouts.position[0] -
            ((farthest.position[0] - scouts.position[0]) / from(farthest)) * back,
          scouts.position[1] -
            ((farthest.position[1] - scouts.position[1]) / from(farthest)) * back,
        ],
        route: "shortest",
      },
    ],
    "ruler scout withdrawal",
  );
  // Losing it leaves an area that remembers the type seen there. The side
  // keeps one area per cause, so the tank firing on after it is lost renews
  // that same area as firing evidence, still remembering the tank.
  const lostArea = (x) => x.contacts.find((c) => c.kind === farthest.kind);
  o = await until(
    page,
    (x) => !x.identified.some((e) => e.id === farthest.id) && !!lostArea(x),
    30 * 180,
    15,
  );
  const contact = o && lostArea(o);
  if (contact) {
    const armed =
      o.own.find((u) => u.kind === "test_tank") ?? o.own.find((u) => u.kind === "test_rifle");
    await lab(page, (ids) => window.__lab.route.select(ids), [armed.id]);
    await page.waitForFunction(() => window.__lab.route.selected().length === 1);
    await aim(page, contact.center, { distance: 120, pitch: 0.85, yaw: CAMERA.default.yaw });
    await frames();
    const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
    css = await groundCss(page, contact.center);
    await page.mouse.click(css[0], css[1], { button: "right" });
    await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
    const [ack] = await lab(page, () => window.__lab.route.acks());
    ctx.check(
      "right-clicking a contact's area attacks it",
      ack.ack.error === null && ack.label.startsWith(`attack area ${contact.id} `),
      JSON.stringify({ ack, contact }),
    );
    await snapshot(ctx, page, "contact-attack-1920x1080.png");
  }
  ctx.check("a contact appears to right-click", !!contact, `${o?.tick}`);
  await page.close();
}
