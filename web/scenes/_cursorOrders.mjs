// Player gestures through the shared session and real worker authority.
import { lab, obs, advance, until, openBattle, snapshot, presented } from "./_lab.mjs";
import { decode, writeCrop } from "./_png.mjs";

const base = (ctx) => new URL(ctx.url).origin;
const select = async (page, ids) => {
  await lab(page, (ids) => window.__lab.route.select(ids), ids);
  await page.waitForFunction(
    (ids) => JSON.stringify(window.__lab.route.selected()) === JSON.stringify(ids),
    ids,
  );
};
const project = (page, at) => lab(page, (p) => window.__lab.projectToCss(...p), at);
const action = (page) => page.locator('[data-testid="game-cursor"]').getAttribute("data-action");
async function expects(page, expected) {
  await page.waitForFunction(
    (expected) => {
      const cursor = document.querySelector('[data-testid="game-cursor"]');
      return cursor && !cursor.hidden && cursor.dataset.action === expected;
    },
    expected,
    { timeout: 10000 },
  );
}
async function capture(ctx, page, name, pointer, wide = false) {
  const size = page.viewportSize();
  const png = decode(await snapshot(ctx, page, `cursor-${name}-${size.width}x${size.height}.png`));
  let bounds = {
    left: pointer[0] - 42,
    top: pointer[1] - 28,
    right: pointer[0] + 78,
    bottom: pointer[1] + 64,
  };
  if (wide) {
    const message = await page.getByTestId("rejected-order").boundingBox();
    bounds = {
      left: Math.min(pointer[0] - 12, message.x - 12),
      top: Math.min(pointer[1] - 12, message.y - 12),
      right: Math.max(pointer[0] + 53, message.x + message.width + 12),
      bottom: Math.max(pointer[1] + 50, message.y + message.height + 12),
    };
  }
  bounds.left = Math.max(0, bounds.left);
  bounds.top = Math.max(0, bounds.top);
  bounds.right = Math.min(size.width, bounds.right);
  bounds.bottom = Math.min(size.height, bounds.bottom);
  for (const scale of [1, 3])
    await writeCrop(
      png,
      ctx.evidencePath(`cursor-${name}-${scale}x.png`),
      (bounds.left + bounds.right) / 2,
      (bounds.top + bounds.bottom) / 2,
      (bounds.right - bounds.left) / 2,
      (bounds.bottom - bounds.top) / 2,
      scale,
    );
}

async function onBuilding(page, center, height) {
  const p = await project(page, [...center, height]);
  await page.mouse.move(...p);
  await page.waitForFunction(() => window.__lab.route.pointerIntent().kind === "occupy_building");
  try {
    await expects(page, "garrison");
  } catch (error) {
    console.log(
      "BUILDING-DIAGNOSTIC",
      await lab(page, () => ({
        intent: window.__lab.route.pointerIntent(),
        resolution: window.__lab.route.pointerResolution(),
        own: window.__lab.route.observation().own,
        known: window.__lab.route.observation().knownProps,
      })),
    );
    throw error;
  }
  return p;
}

async function infoCardCursor(ctx, page) {
  await page.evaluate(() => {
    const samples = [];
    const record = (event) => {
      if (!(event.target instanceof Element) || !event.target.closest(".ro-layer .ro-unit")) return;
      samples.push({
        event: event.type,
        native: getComputedStyle(event.target).cursor,
        visible: !document.querySelector('[data-testid="game-cursor"]').hidden,
      });
    };
    window.__cardCursorProbe = { samples, record };
    window.addEventListener("pointerover", record, true);
    window.addEventListener("pointermove", record, true);
  });
  try {
    for (const id of [0, 2]) {
      await page.locator(`.ro-unit[data-unit="${id}"] .ro-name-word`).hover();
      await page.evaluate(async () => {
        for (let i = 0; i < 12; i++) {
          await new Promise(requestAnimationFrame);
          const cursor = document.querySelector('[data-testid="game-cursor"]');
          const matrix = new DOMMatrix(getComputedStyle(cursor).transform);
          const hit = document.elementFromPoint(matrix.e, matrix.f);
          window.__cardCursorProbe.samples.push({
            event: "frame",
            native: hit && getComputedStyle(hit).cursor,
            visible: !cursor.hidden,
          });
        }
      });
    }
    const samples = await page.evaluate(() => window.__cardCursorProbe.samples);
    ctx.check(
      "info-card handoffs and resting hover keep only the game cursor visible",
      samples.length >= 24 && samples.every((s) => s.visible && s.native === "none"),
      JSON.stringify(samples),
    );
    await page.screenshot({ path: ctx.evidencePath("cursor-info-card-hover.png") });
    await ctx.writeEvidence("cursor-info-card-hover.json", samples);
    await page.mouse.down({ button: "middle" });
    try {
      await page.waitForFunction(
        () => document.querySelector('[data-testid="game-cursor"]').hidden,
      );
      const fallback = await page
        .locator('.ro-unit[data-unit="2"] .ro-name-word')
        .evaluate((card) => getComputedStyle(card).cursor);
      ctx.check(
        "info cards do not request a native hand during camera control",
        fallback !== "pointer",
        fallback,
      );
    } finally {
      await page.mouse.up({ button: "middle" });
    }

    await page.getByRole("button", { name: "Reset", exact: true }).hover();
    await page.waitForFunction(() => document.querySelector('[data-testid="game-cursor"]').hidden);
    ctx.check(
      "HUD buttons regain their native hand when the game cursor is hidden",
      (await page
        .getByRole("button", { name: "Reset", exact: true })
        .evaluate((button) => getComputedStyle(button).cursor)) === "pointer",
    );
  } finally {
    await page.evaluate(() => {
      const { record } = window.__cardCursorProbe;
      window.removeEventListener("pointerover", record, true);
      window.removeEventListener("pointermove", record, true);
      delete window.__cardCursorProbe;
    });
  }
}

export async function garrisonCursor(ctx) {
  const page = await openBattle(
    {
      ...ctx,
      newPage: async (options) => {
        const page = await ctx.newPage(options);
        await page.addInitScript(delayedAdmission);
        return page;
      },
    },
    { url: `${base(ctx)}/lab/garrison`, tick: 40 },
  );
  await lab(page, () =>
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [346, 252, 0],
      distance: 130,
      pitch: 0.95,
      yaw: -0.75,
    }),
  );
  await select(page, [0, 2]);
  await presented(page);
  const digest = await lab(page, () => window.__lab.route.digest());
  const p = await onBuilding(page, [360, 250], 8);
  ctx.check(
    "hover queries preserve the identical baseline battle",
    (await obs(page)).tick === 40 &&
      (await lab(page, () => window.__lab.route.digest())) === digest,
    digest,
  );
  await ctx.writeEvidence(
    "cursor-baseline-state.json",
    await lab(page, () => ({
      tick: window.__lab.route.tick(),
      digest: window.__lab.route.digest(),
      camera: window.__lab.camera(),
      selected: window.__lab.route.selected(),
      intent: window.__lab.route.pointerIntent(),
      resolution: window.__lab.route.pointerResolution(),
    })),
  );
  await capture(ctx, page, "garrison-hover", p);
  await infoCardCursor(ctx, page);
  await onBuilding(page, [360, 250], 8);

  // Recompute without a pointer event, while paused.
  await select(page, []);
  await expects(page, "default");
  await select(page, [0, 2]);
  await expects(page, "garrison");
  await page.keyboard.down("Control");
  await expects(page, "attack_move");
  await capture(ctx, page, "ctrl-attack-move", p);
  await page.mouse.down({ button: "right" });
  await page.waitForFunction(() => window.__lab.route.pointerResolution().state === "ready");
  await page.evaluate(() => window.__lab.frame());
  ctx.check(
    "held attack-move queries availability without movement marks",
    (await lab(page, () => window.__lab.route.movePreview())).length === 0,
  );
  await page.keyboard.press("Escape");
  await page.mouse.up({ button: "right" });
  await page.keyboard.up("Control");
  await page.mouse.move(...p);
  await expects(page, "garrison");
  await page.keyboard.down("Shift");
  await page.waitForFunction(() => window.__lab.route.pointerIntent().queued === true);
  await page.keyboard.up("Shift");
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [406, 252, 0] }),
  );
  await expects(page, "default");
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [346, 252, 0] }),
  );
  await expects(page, "garrison");
  ctx.check(
    "stationary pointer follows selection, modifiers and camera",
    (await action(page)) === "garrison",
  );

  // A real refused acknowledgement beside the composite cursor.
  const rejected = await lab(page, () =>
    window.__lab.route.command({ kind: "garrison", units: [0, 2], building: 0 }),
  );
  ctx.check(
    "direct multi-squad entry still refuses through native admission",
    !!rejected.error,
    JSON.stringify(rejected),
  );
  await page.locator('[data-testid="rejected-order"]').waitFor();
  await capture(ctx, page, "refused-callout", p, true);

  await page.getByTestId("rejected-order").waitFor({ state: "hidden" });
  // Hold, drag facing, then retain released marks across actual worker latency.
  await page.evaluate(() => {
    window.__delayCommandAcks = 800;
  });
  await page.mouse.down({ button: "right" });
  await page.mouse.move(p[0] + 34, p[1] + 14);
  await page.waitForFunction(() => window.__lab.route.movePreview().length > 0);
  const held = await lab(page, () => ({
    intent: window.__lab.route.pointerIntent(),
    marks: window.__lab.route.movePreview(),
  }));
  await capture(ctx, page, "held-building", [p[0] + 34, p[1] + 14]);
  await page.mouse.up({ button: "right" });
  await page.waitForTimeout(100);
  const released = await lab(page, () => window.__lab.route.movePreview());
  ctx.check(
    "released facing preview remains visible until delayed acknowledgement",
    released.length > 0 && JSON.stringify(released) === JSON.stringify(held.marks),
    JSON.stringify(released),
  );
  await page.waitForFunction(() =>
    window.__lab.route.acks().some(({ order }) => order.kind === "occupy_building"),
  );
  const accepted = await lab(page, () =>
    window.__lab.route.acks().find(({ order }) => order.kind === "occupy_building"),
  );
  ctx.check(
    "building release sends the captured combined selection and facing",
    !accepted.ack.error &&
      accepted.order.units.join() === "0,2" &&
      accepted.order.building === 0 &&
      Math.abs(accepted.order.facing - held.intent.facing) < 1e-6 &&
      !accepted.queued,
    JSON.stringify(accepted),
  );
  await page.mouse.move(...p);
  await expects(page, "garrison");
  await capture(ctx, page, "accepted-building", p);
  await advance(page, accepted.ack.applied_tick - (await obs(page)).tick);
  const arrived = await until(
    page,
    (o) =>
      o.own.some((u) => u.garrison?.phase === "inside") &&
      accepted.ack.building.destinations
        .filter((d) => d.placed)
        .every((d) => {
          const u = o.own.find((u) => u.id === d.unit);
          return (
            u.route.length === 0 &&
            Math.hypot(u.position[0] - d.goal[0], u.position[1] - d.goal[1]) < 1
          );
        }),
    900,
  );
  ctx.check(
    "the combined order actually enters one squad and gathers its companion",
    !!arrived &&
      arrived.own.filter((u) => u.garrison?.phase === "inside").length === 1 &&
      arrived.own.find((u) => u.id === accepted.ack.building.destinations[0]?.unit)?.garrison ==
        null,
    JSON.stringify(
      arrived?.own.map((u) => ({
        id: u.id,
        position: u.position,
        garrison: u.garrison,
        goal: u.goal,
      })),
    ),
  );
  await capture(ctx, page, "building-arrived", p);
  await page.close();
}

/** Delay only admission delivery, at the real worker edge; no production hook. */
export function delayedAdmission() {
  const NativeWorker = window.Worker;
  window.Worker = class extends NativeWorker {
    constructor(...args) {
      super(...args);
      window.__cursorWorkerEpoch = (window.__cursorWorkerEpoch ?? 0) + 1;
    }
    postMessage(message, ...args) {
      if (message.type === "command")
        window.__cursorCommands = [...(window.__cursorCommands ?? []), message];
      super.postMessage(message, ...args);
    }
    set onmessage(handler) {
      super.onmessage = (event) => {
        const delay = event.data.type === "ack" ? window.__delayCommandAcks : 0;
        if (delay) setTimeout(() => handler?.call(this, event), delay);
        else handler?.call(this, event);
      };
    }
  };
}

export async function movementCursor(ctx) {
  const page = await openBattle(ctx, { url: `${base(ctx)}/lab/movement`, tick: 40 });
  await lab(page, () =>
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [265, 270, 0],
      distance: 230,
      pitch: 1.1,
      yaw: -0.7,
    }),
  );
  const staged = await lab(page, () =>
    window.__lab.route.command({
      kind: "move",
      units: [0, 2],
      gesture: 8400,
      goal: [300, 260],
      route: "shortest",
    }),
  );
  ctx.check(
    "mixed cursor proof stages an admitted approach",
    !staged.error && staged.placement.destinations.every((d) => d.placed),
    JSON.stringify(staged),
  );
  if (staged.error) throw Error("mixed approach refused");
  const close = await until(
    page,
    (o) =>
      staged.placement.destinations.every((d) => {
        const u = o.own.find((u) => u.id === d.unit);
        return (
          u.route.length === 0 &&
          Math.hypot(u.position[0] - d.goal[0], u.position[1] - d.goal[1]) < 1
        );
      }),
    4500,
  );
  ctx.check(
    "mixed selection physically reaches the approach",
    !!close,
    JSON.stringify(
      (await obs(page)).own
        .filter((u) => [0, 2].includes(u.id))
        .map((u) => ({
          id: u.id,
          position: u.position,
          goal: u.goal,
          state: u.state,
          route: u.route,
          blocker: u.blocker,
        })),
    ),
  );
  if (!close) throw Error("mixed approach did not arrive");
  await select(page, [0, 2]);
  const p = await onBuilding(page, [320, 300], 4);
  await capture(ctx, page, "mixed-building-hover", p);
  await page.mouse.down({ button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route.movePreview().some((mark) => mark.unit === 0),
  );
  await capture(ctx, page, "mixed-building-held", p);
  await page.mouse.up({ button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route.acks().some(({ order }) => order.kind === "occupy_building"),
  );
  const accepted = await lab(page, () =>
    window.__lab.route.acks().find(({ order }) => order.kind === "occupy_building"),
  );
  ctx.check(
    "mixed squad and vehicle pointer order admits entry and gathering",
    !accepted.ack.error &&
      accepted.order.units.join() === "0,2" &&
      accepted.ack.building.entrant?.unit === 2 &&
      accepted.ack.building.destinations.some((mark) => mark.unit === 0 && mark.placed),
    JSON.stringify(accepted),
  );
  await capture(ctx, page, "mixed-building-accepted", p);
  const tankGoal = accepted.ack.building.destinations.find((mark) => mark.unit === 0)?.goal;
  const arrived = await until(
    page,
    (o) =>
      o.own.find((u) => u.id === 2)?.garrison?.phase === "inside" &&
      o.own.find((u) => u.id === 0)?.route.length === 0 &&
      Math.hypot(
        ...o.own
          .find((u) => u.id === 0)
          .position.slice(0, 2)
          .map((v, i) => v - tankGoal[i]),
      ) < 1,
    1600,
  );
  ctx.check(
    "vehicle actually gathers while the selected squad enters",
    !!arrived,
    JSON.stringify(arrived?.own.filter((u) => [0, 2].includes(u.id))),
  );
  await capture(ctx, page, "mixed-building-arrived", p);
  // A fresh own entry claim changes availability immediately, even paused.
  await select(page, [3]);
  await page.mouse.move(...p);
  await expects(page, "default");
  ctx.check(
    "known occupied building falls back to the plain movement cursor",
    (await action(page)) === "default",
  );
  await capture(ctx, page, "building-fallback", p);
  await page.close();
}

export async function battleCursor(ctx) {
  const page = await openBattle(
    {
      ...ctx,
      newPage: async (options) => {
        const page = await ctx.newPage(options);
        await page.addInitScript(delayedAdmission);
        return page;
      },
    },
    { url: `${base(ctx)}/battle/village`, tick: 40 },
  );
  const o = await obs(page);
  const tank = o.own.find((u) => u.kind === "tank");
  const squad = o.own.find((u) => u.kind === "rifle");
  await select(page, [tank.id]);
  await lab(
    page,
    (at) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [...at, 0],
        distance: 200,
        pitch: 1.15,
      }),
    tank.position.slice(0, 2),
  );
  const goal = [tank.position[0] + 24, tank.position[1] + 15, 0];
  const p = await project(page, goal);
  await page.mouse.move(...p);
  await expects(page, "default");
  await capture(ctx, page, "default", p);
  const own = page.locator(`.ro-unit[data-unit="${tank.id}"]`);
  await own.hover();
  await expects(page, "default");
  ctx.check("own unit hover is a plain arrow", (await action(page)) === "default");
  for (const [label, expected] of [
    ["Attack-move", "attack_move"],
    ["Attack ground", "attack_ground"],
    ["Fast move", "fast_move"],
    ["Reverse", "reverse_move"],
  ]) {
    await page.getByRole("button", { name: new RegExp(`^${label} `) }).click();
    await page.mouse.move(...p);
    await expects(page, expected);
    await capture(ctx, page, expected, p);
  }
  await page.keyboard.press("Escape");
  // Automatic reverse uses the same existing vehicle command resolver.
  const rear = [
    tank.position[0] - Math.cos(tank.yaw) * 14,
    tank.position[1] - Math.sin(tank.yaw) * 14,
    0,
  ];
  const rearCss = await project(page, rear);
  await page.mouse.move(...rearCss);
  await expects(page, "reverse_move");
  ctx.check(
    "automatic reverse hover matches the vehicle's rear zone",
    (await action(page)) === "reverse_move",
  );
  // Unarmed units still advance on attack-move.
  const supply = o.own.find((u) => u.kind === "supply");
  await select(page, [supply.id]);
  await page.mouse.move(...p);
  await page.keyboard.down("Control");
  await expects(page, "attack_move");
  await capture(ctx, page, "unarmed-attack-move", p);
  await page.keyboard.up("Control");
  await select(page, [squad.id]);
  // Panels retain their own cursor even when a captured drag crosses them.
  await page.mouse.move(...p);
  await page.mouse.down({ button: "right" });
  await page.getByRole("button", { name: "Menu", exact: true }).hover();
  await page.evaluate(() => window.__lab.frame());
  ctx.check(
    "a held drag over HUD controls suppresses the action overlay",
    await page.locator('[data-testid="game-cursor"]').evaluate((e) => e.hidden),
  );
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await page.mouse.up({ button: "right" });
  // Panels and the pause menu leave their native pointer intact.
  await page.getByRole("button", { name: "Menu", exact: true }).hover();
  await page.waitForFunction(() => document.querySelector('[data-testid="game-cursor"]').hidden);
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("dialog", { name: "Paused" }).waitFor();
  await snapshot(ctx, page, "cursor-menu-hidden-1280x800.png");
  ctx.check(
    "the cursor overlay is absent over the pause menu",
    await page.locator('[data-testid="game-cursor"]').evaluate((e) => e.hidden),
  );
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await lab(page, () => window.__lab.route.pause());
  await page.mouse.move(...p);
  await page.mouse.down({ button: "middle" });
  await page.mouse.move(p[0] + 20, p[1] + 10);
  await page.waitForFunction(() => document.querySelector('[data-testid="game-cursor"]').hidden);
  await snapshot(ctx, page, "cursor-orbit-hidden-1280x800.png");
  ctx.check(
    "camera orbit suppresses the action overlay",
    await page.locator('[data-testid="game-cursor"]').evaluate((e) => e.hidden),
  );
  await page.mouse.up({ button: "middle" });
  // A stale release after reset must not reinterpret itself as a fresh order.
  await page.mouse.move(...p);
  await page.mouse.down({ button: "right" });
  await resetWorkerBattle(page);
  await select(page, [tank.id]);
  await page.mouse.up({ button: "right" });
  await page.waitForTimeout(150);
  const after = await lab(page, () => window.__lab.route.acks());
  ctx.check(
    "reset cancels a held press without a new-battle order",
    after.length === 0,
    JSON.stringify(after),
  );
  await select(page, [tank.id]);
  await page.evaluate(() => {
    window.__delayCommandAcks = 800;
  });
  const pendingPoint = await project(page, [tank.position[0] + 20, tank.position[1] + 20, 0]);
  await page.mouse.click(...pendingPoint, { button: "right" });
  await page.waitForFunction(() => window.__cursorCommands?.length > 0);
  await page.waitForTimeout(100);
  await resetWorkerBattle(page);
  await page.waitForTimeout(900);
  ctx.check(
    "reset with admission outstanding leaves the new log empty",
    (await lab(page, () => window.__lab.route.acks())).length === 0,
  );
  await page.evaluate(() => {
    window.__delayCommandAcks = 0;
  });
  await select(page, [tank.id]);
  await page.getByRole("button", { name: /^Stop / }).click();
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  ctx.check(
    "new-battle acknowledgements remain current after the old reply",
    (await lab(page, () => window.__lab.route.acks())).every(({ order }) => order.kind === "stop"),
  );
  await page.close();
}

export async function queuedBuildingCursor(ctx) {
  const page = await openBattle(ctx, { url: `${base(ctx)}/lab/garrison`, tick: 40 });
  await select(page, [0, 2]);
  const first = await lab(page, () =>
    window.__lab.route.command({
      kind: "move",
      units: [0, 2],
      gesture: 8402,
      goal: [290, 240],
      route: "shortest",
    }),
  );
  if (first.error) throw Error("queue staging refused");
  await advance(page, first.applied_tick - (await obs(page)).tick);
  const before = await obs(page);
  const p = await onBuilding(page, [360, 250], 8);
  await page.keyboard.down("Shift");
  await page.mouse.down({ button: "right" });
  // A held press retains Shift even if it is released before the mouse.
  await page.keyboard.up("Shift");
  await page.mouse.up({ button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route.acks().some(({ order }) => order.kind === "occupy_building"),
  );
  const accepted = await lab(page, () =>
    window.__lab.route.acks().find(({ order }) => order.kind === "occupy_building"),
  );
  await advance(page, accepted.ack.applied_tick - (await obs(page)).tick);
  const after = await obs(page);
  ctx.check(
    "Shift captured at press queues entry and gathering behind the current moves",
    !accepted.ack.error &&
      accepted.label.endsWith("(queued)") &&
      [0, 2].every((id) => {
        const a = after.own.find((u) => u.id === id),
          b = before.own.find((u) => u.id === id);
        return JSON.stringify(a.goal) === JSON.stringify(b.goal) && a.queue.length > 0;
      }),
    JSON.stringify({
      accepted,
      units: after.own
        .filter((u) => [0, 2].includes(u.id))
        .map((u) => ({ id: u.id, goal: u.goal, queue: u.queue, garrison: u.garrison })),
    }),
  );
  await capture(ctx, page, "queued-building", p);
  await page.close();
}

export async function attackCursor(ctx) {
  const page = await openBattle(ctx, { url: `${base(ctx)}/lab/weapons`, tick: 40 });
  const observation = await obs(page);
  const enemy = observation.identified.find((u) => u.kind === "tank") ?? observation.identified[0];
  if (!enemy) throw Error("weapons fixture exposes no identified enemy");
  await select(page, [0]);
  await lab(
    page,
    (at) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [...at, 0],
        distance: 170,
        pitch: 1.1,
      }),
    enemy.position.slice(0, 2),
  );
  const at = enemy.members[0] ?? enemy.position;
  const p = await project(page, [at[0], at[1], at[2] + 1]);
  await page.mouse.move(...p);
  await expects(page, "attack");
  await capture(ctx, page, "attack", p);
  await page.mouse.down({ button: "right" });
  await page.evaluate(() => window.__lab.frame());
  ctx.check(
    "an identified enemy shows attack without movement destination marks",
    (await action(page)) === "attack" &&
      (await lab(page, () => window.__lab.route.movePreview())).length === 0,
  );
  await page.mouse.up({ button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route.acks().some(({ order }) => order.kind === "attack"),
  );
  const accepted = await lab(page, () =>
    window.__lab.route.acks().find(({ order }) => order.kind === "attack"),
  );
  ctx.check(
    "attack pointer dispatch uses the shown side-scoped handle",
    !accepted.ack.error &&
      accepted.order.target.kind === "identified" &&
      accepted.order.target.id === enemy.id,
    JSON.stringify(accepted),
  );
  await page.close();
}

export async function partialBuildingCursor(ctx) {
  for (const viewport of [
    { width: 1280, height: 800 },
    { width: 430, height: 800 },
  ]) {
    const page = await openBattle(ctx, {
      url: `${base(ctx)}/lab/cursor-orders`,
      tick: 40,
      viewport,
    });
    await select(page, [0, 1]);
    const p = await onBuilding(page, [360, 250], 8);
    await page.mouse.click(...p, { button: "right" });
    await page.waitForFunction(() =>
      window.__lab.route.acks().some(({ order }) => order.kind === "occupy_building"),
    );
    const accepted = await lab(page, () =>
      window.__lab.route.acks().find(({ order }) => order.kind === "occupy_building"),
    );
    ctx.check(
      "partial entry succeeds while the isolated vehicle holds",
      !accepted.ack.error &&
        accepted.ack.building.entrant?.unit === 1 &&
        accepted.ack.building.destinations.some((d) => d.unit === 0 && !d.placed),
      JSON.stringify(accepted),
    );
    await page.getByTestId("rejected-order").waitFor();
    await capture(
      ctx,
      page,
      viewport.width === 430 ? "partial-callout-narrow" : "partial-callout",
      p,
      true,
    );
    ctx.check(
      "partial refusal stays fully visible and clear of the composite cursor",
      await calloutFits(page),
    );
    const arrived = await until(
      page,
      (o) => o.own.find((u) => u.id === 1)?.garrison?.phase === "inside",
      600,
    );
    const heldVehicle = arrived?.own.find((u) => u.id === 0);
    ctx.check(
      "successful partial entry applies while the failed vehicle holds",
      !!arrived &&
        heldVehicle.goal === null &&
        heldVehicle.route.length === 0 &&
        Math.hypot(heldVehicle.position[0] - 160, heldVehicle.position[1] - 160) < 0.01,
    );
    await capture(
      ctx,
      page,
      viewport.width === 430 ? "partial-arrived-narrow" : "partial-arrived",
      p,
    );
    await page.close();
  }
}

async function calloutFits(page) {
  return page.getByTestId("rejected-order").evaluate((el) => {
    const r = el.getBoundingClientRect();
    const c = document.querySelector('[data-testid="game-cursor"]').getBoundingClientRect();
    return (
      r.left >= 8 &&
      r.right <= innerWidth - 8 &&
      r.top >= 8 &&
      r.bottom <= innerHeight - 8 &&
      (r.right <= c.left || r.left >= c.right || r.bottom <= c.top || r.top >= c.bottom)
    );
  });
}

export async function narrowRefusal(ctx) {
  const page = await openBattle(ctx, {
    url: `${base(ctx)}/lab/cursor-orders`,
    tick: 40,
    viewport: { width: 430, height: 800 },
  });
  await select(page, [0, 1]);
  const p = await onBuilding(page, [360, 250], 8);
  await lab(page, () =>
    window.__lab.route.command({ kind: "garrison", units: [1, 2], building: 0 }),
  );
  await page.getByTestId("rejected-order").waitFor();
  await capture(ctx, page, "refused-callout-narrow", p, true);
  ctx.check(
    "the complete refusal stays inside the narrow viewport and clear of the cursor",
    await calloutFits(page),
  );
  await page.close();
}

async function resetWorkerBattle(page) {
  const epoch = await page.evaluate(() => window.__cursorWorkerEpoch);
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction((epoch) => window.__cursorWorkerEpoch > epoch, epoch);
  await page.waitForFunction(() => window.__lab.route.tick() > 3);
  await lab(page, () => window.__lab.route.pause());
}

export async function readoutEdge(ctx) {
  const page = await openBattle(ctx, {
    url: `${base(ctx)}/lab/cursor-orders`,
    tick: 40,
    viewport: { width: 430, height: 800 },
  });
  await select(page, [0]);
  await lab(page, () => {
    const yaw = -0.75;
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [160, 160, 0],
      distance: 200,
      pitch: 1.15,
      yaw,
    });
  });
  await page.keyboard.down("Space");
  await page.waitForFunction(
    () => document.querySelector('.ro-unit[data-unit="0"]')?.dataset.zoom === "default",
  );
  const box = await page.locator('.ro-unit[data-unit="0"]').boundingBox();
  if (!box || box.x + box.width < 416)
    throw Error(`own readout is not at the screen edge: ${JSON.stringify(box)}`);
  await page.mouse.move(box.x + box.width - 2, box.y + box.height / 2);
  await page.waitForFunction(() =>
    document
      .elementFromPoint(
        420,
        document.querySelector('.ro-unit[data-unit="0"]').getBoundingClientRect().y + 10,
      )
      ?.closest(".ro-unit"),
  );
  const before = await lab(page, () => window.__lab.camera());
  await page.waitForTimeout(350);
  const after = await lab(page, () => window.__lab.camera());
  ctx.check(
    "hovering an edge readout leaves the camera stationary",
    JSON.stringify(before.target) === JSON.stringify(after.target),
    JSON.stringify({ before, after }),
  );
  await page.mouse.move(215, 400);
  await snapshot(ctx, page, "cursor-readout-edge-after-430x800.png");
  await page.keyboard.up("Space");
  await page.close();
}
