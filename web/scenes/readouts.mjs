// Slice 14: every displayed timer is the published one; completed timers
// vanish; one ring per weapon; ∞ for unlimited; guidance icon; no enemy
// readiness; the panel keeps details when zoomed out; commands and keys.
import { decode, pixel, writeCrop } from "./_png.mjs";
import { lab, obs, advance, snapshot, until, openBattle } from "./_lab.mjs";
import { paintOnly } from "./_overlays.mjs";
import { hull, game } from "./_units.mjs";

/** The rings on screen, read back from the DOM. */
const rings = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("[data-testid=readouts] .ro-unit")].map((u) => ({
      unit: Number(u.dataset.unit),
      shown: u.style.display !== "none",
      mounts: [...u.querySelectorAll(".ro-weapon[data-reason]")].map((s) => ({
        reason: s.dataset.reason,
        aim: s.dataset.aim === "" ? null : Number(s.dataset.aim),
        reload: s.dataset.reload === "" ? null : Number(s.dataset.reload),
        aimArc: !!s.querySelector(".ro-aim"),
        reloadArc: !!s.querySelector(".ro-reload"),
        text: s.dataset.ammo,
        guide: !!s.querySelector(".ro-guide"),
      })),
      deploy: [...u.querySelectorAll(".ro-state")].some((e) =>
        ["deploying", "packing", "deployed"].includes(e.dataset.state),
      ),
    })),
  );

async function shots(ctx, page, name, focus) {
  const png = await snapshot(ctx, page, `frame-${name}.png`);
  if (focus) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), focus);
    for (const scale of [2, 4]) {
      const half = scale === 2 ? [150, 80] : [75, 40];
      await writeCrop(
        decode(png),
        ctx.evidencePath(`crop-${name}-${scale}x.png`),
        at[0],
        at[1] - 30,
        half[0],
        half[1],
        scale,
      );
    }
  }
}

export async function run(ctx) {
  const page = await openBattle(ctx);
  await lab(page, () => window.__lab.route.select([0, 1, 2, 3]));
  await page.waitForFunction(() => window.__lab.route.selected().length === 4);

  // Panels use the same selection path as bodies, including drag gestures.
  const truckPanel = page.locator('[data-testid=readouts] .ro-unit[data-unit="3"]');
  const panelBox = await truckPanel.boundingBox();
  await page.mouse.click(panelBox.x + panelBox.width / 2, panelBox.y + panelBox.height / 2);
  await page.waitForFunction(() => window.__lab.route.selected().join(",") === "3");
  ctx.check("clicking a floating panel selects its unit", true);
  await lab(page, () => window.__lab.route.select([]));
  await page.mouse.move(panelBox.x - 2, panelBox.y - 2);
  await page.mouse.down();
  await page.mouse.move(panelBox.x + panelBox.width + 2, panelBox.y + panelBox.height + 2, {
    steps: 4,
  });
  await page.mouse.up();
  await page.waitForFunction(() => window.__lab.route.selected().includes(3));
  ctx.check("dragging a rectangle over a floating panel selects its unit", true);
  await lab(page, () => window.__lab.route.select([]));
  await page.mouse.move(panelBox.x + 4, panelBox.y + 4);
  await page.mouse.down();
  await page.mouse.move(panelBox.x + panelBox.width + 2, panelBox.y + panelBox.height + 2, {
    steps: 4,
  });
  await page.mouse.up();
  await page.waitForFunction(() => window.__lab.route.selected().includes(3));
  ctx.check("a selection drag can start inside a floating panel", true);

  // Pick empty ground inside the rifle squad's circle, away from each body and panel.
  const interior = await lab(
    page,
    (scale) => {
      const u = window.__lab.route.observation().own.find((u) => u.id === 2);
      const boxes = [...document.querySelectorAll(".ro-unit")]
        .filter((n) => n.style.display !== "none")
        .map((n) => n.getBoundingClientRect());
      for (let k = 0; k < 32; k++) {
        const angle = (k * Math.PI) / 16;
        const radius = u.area.radius * scale * 0.75;
        const p = [
          u.area.anchor[0] + Math.cos(angle) * radius,
          u.area.anchor[1] + Math.sin(angle) * radius,
        ];
        if (u.members.some((m) => Math.hypot(m[0] - p[0], m[1] - p[1]) < 2)) continue;
        const q = window.__lab.projectToCss(...p, window.__lab.route.surfaceZ(...p));
        if (
          q &&
          q[0] > 10 &&
          q[1] > 10 &&
          q[0] < innerWidth - 10 &&
          q[1] < innerHeight - 10 &&
          !boxes.some((b) => q[0] >= b.left && q[0] <= b.right && q[1] >= b.top && q[1] <= b.bottom)
        )
          return q;
      }
      return null;
    },
    game.presentation.overlay.orders.area_draw_scale,
  );
  ctx.check(
    "the infantry picking probe is inside its circle and clear of bodies and panels",
    !!interior,
    JSON.stringify(interior),
  );
  if (interior) {
    await page.mouse.click(...interior);
    await page.waitForFunction(() => window.__lab.route.selected().join(",") === "2");
    ctx.check("clicking empty ground inside the infantry circle selects the squad", true);
  }
  await lab(page, () => window.__lab.route.select([0, 1, 2, 3]));
  await page.waitForFunction(() => window.__lab.route.selected().length === 4);

  // Right-click an identified enemy: attack it.
  let o0 = await obs(page);
  const enemy = o0.identified[0];
  let ack;
  if (enemy) {
    const at = await lab(
      page,
      (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1),
      enemy.position,
    );
    const before = (await lab(page, () => window.__lab.route.acks())).length;
    await page.mouse.click(at[0], at[1], { button: "right" });
    await page.waitForFunction((n) => window.__lab.route.acks().length > n, before);
    ack = (await lab(page, () => window.__lab.route.acks()))[0];
  }
  ctx.check(
    "right-clicking an identified enemy attacks it",
    !!enemy && /attack enemy/.test(ack.label) && ack.ack.error === null,
    JSON.stringify(ack),
  );
  await page.keyboard.down("Space");
  await page.waitForFunction(() => document.querySelector(".ro-layer").dataset.zoom === "default");
  // Mid-engagement: aim, reload and deployment timers all running.
  let matched = true;
  let priorityMatched = true;
  let guided = false;
  let aimShot = false;
  let o;
  for (let step = 0; step < 12; step++) {
    await advance(page, 23);
    await page.evaluate(() => window.__lab.frame());
    o = await obs(page);
    const drawn = await rings(page);
    for (const u of o.own) {
      const r = drawn.find((d) => d.unit === u.id);
      if (!r) continue;
      u.mounts.forEach((m, k) => {
        const d = r.mounts[k];
        const aim = m.target && m.aim < 1 ? m.aim : null;
        const reload = m.loaded === null && m.reload > 0 ? m.reload : null;
        matched &&= d.reason === m.reason && d.aim === aim && d.reload === reload;
        priorityMatched &&=
          d.aimArc === (aim !== null) && d.reloadArc === (aim === null && reload !== null);
        guided ||= m.guiding && d.guide;
      });
    }
    if (!aimShot && (await rings(page)).some((r) => r.mounts.some((m) => m.aimArc))) {
      aimShot = true;
      await shots(ctx, page, "aiming-1280x800", [200, 220, 0]);
    }
    if (step === 3) {
      await shots(ctx, page, "engaged-1280x800", [200, 220, 0]);
      // The same camera and tick without the readouts, for comparison.
      await page.evaluate(() => {
        document.querySelector("[data-testid=readouts]").style.visibility = "hidden";
      });
      await shots(ctx, page, "engaged-no-readouts-1280x800");
      await page.evaluate(() => {
        document.querySelector("[data-testid=readouts]").style.visibility = "";
      });
    }
  }
  ctx.check("every displayed timer is the published one", matched);
  ctx.check(
    "one progress ring prioritizes aiming, then reload, and hides completed timers",
    priorityMatched,
  );
  ctx.check("a guiding launcher shows the guidance icon", guided);
  ctx.check("an aim ring was captured", aimShot);
  const drawn = await rings(page);
  const tank = drawn.find((d) => d.unit === 0);
  const rifle = drawn.find((d) => d.unit === 2);
  ctx.check(
    "the cannon's AP and HE are one weapon row, and unlimited reads ∞",
    tank.mounts.length === 2 &&
      /^AP \d+ · HE \d+$/.test(tank.mounts[0].text) &&
      rifle.mounts[0].text === "∞",
    JSON.stringify({
      tank: tank.mounts.map((m) => m.text),
      rifle: rifle.mounts.map((m) => m.text),
    }),
  );
  ctx.check(
    "the supply truck's panel carries its deployment row",
    drawn.find((d) => d.unit === 3)?.deploy === true,
  );
  const ids = new Set(o.own.map((u) => u.id));
  ctx.check(
    "no enemy readiness is shown",
    drawn.every((d) => ids.has(d.unit)),
    JSON.stringify(drawn.map((d) => d.unit)),
  );

  // A smaller viewport, then zoomed out: rings only for the selection, the
  // panel keeps every detail.
  await page.setViewportSize({ width: 900, height: 600 });
  await shots(ctx, page, "engaged-900x600", [200, 220, 0]);
  // Preserve the last row's dark support and the line's light, with only a faint tail below.
  await page.locator('.ro-unit[data-unit="1"] .ro-name').hover();
  await page.waitForFunction(
    () => document.querySelector('.ro-unit[data-unit="1"]').dataset.zoom === "default",
  );
  // A backing is translucent: absolute pixel losses over a changing
  // battlefield are not a stable contrast experiment. Keep the real callout
  // and its layout, and measure its core and tail over one neutral field.
  await snapshot(ctx, page, "backing-world.png");
  const neutralField = await page.addStyleTag({
    content:
      "div:has(> canvas) { background: rgb(100,100,100) !important; } canvas { visibility: hidden !important; }",
  });
  const atPanel = await page.locator('.ro-unit[data-unit="1"]').boundingBox();
  const backed = decode(await snapshot(ctx, page, "backing-on.png"));
  const noBacking = await page.addStyleTag({
    content: '.ro-unit[data-unit="1"]::before { display: none !important; }',
  });
  const bare = decode(await snapshot(ctx, page, "backing-off.png"));
  await noBacking.evaluate((node) => node.remove());
  let lineLoss = 0;
  let spill = 0;
  let shadeAboveLine = Infinity;
  for (let x = Math.ceil(atPanel.x + 20); x < atPanel.x + atPanel.width - 20; x++) {
    const y = Math.round(atPanel.y + atPanel.height);
    const light = (png) =>
      [0.2126, 0.7152, 0.0722].reduce(
        (sum, weight, c) => sum + weight * pixel(png, x, y - 3)[c],
        0,
      );
    shadeAboveLine = Math.min(shadeAboveLine, light(bare) - light(backed));
    for (const c of [1, 2]) {
      const peak = (png) => Math.max(...[-1, 0, 1].map((dy) => pixel(png, x, y + dy)[c]));
      lineLoss = Math.max(lineLoss, peak(bare) - peak(backed));
      spill = Math.max(spill, Math.abs(pixel(bare, x, y + 8)[c] - pixel(backed, x, y + 8)[c]));
    }
  }
  ctx.check(
    "the backing supports the bright underline without a heavy shadow below it",
    lineLoss <= 8 && spill <= 8 && shadeAboveLine >= 15,
    JSON.stringify({ lineLoss, spill, shadeAboveLine, atPanel }),
  );
  await neutralField.evaluate((node) => node.remove());
  const smallLayout = await page.locator("[data-testid=readouts] .ro-unit").evaluateAll((nodes) => {
    const boxes = nodes
      .filter((n) => n.style.display !== "none")
      .map((n) => n.getBoundingClientRect().toJSON());
    return {
      boxes,
      inside: boxes.every(
        (b) => b.left >= 0 && b.top >= 0 && b.right <= innerWidth && b.bottom <= innerHeight,
      ),
      apart: boxes.every((a, i) =>
        boxes
          .slice(i + 1)
          .every(
            (b) => a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top,
          ),
      ),
    };
  });
  ctx.check(
    "four readouts fit the smaller viewport without hiding or overprinting",
    smallLayout.boxes.length === 4 && smallLayout.inside && smallLayout.apart,
    JSON.stringify(smallLayout),
  );
  const ordered = await lab(page, () =>
    window.__lab.route
      .observation()
      .own.filter((u) => u.kind === "at" || u.kind === "tank")
      .map((u) => ({
        kind: u.kind,
        anchorY: window.__lab.projectToCss(...u.position)[1],
        panelY: document
          .querySelector(`[data-testid=readouts] .ro-unit[data-unit="${u.id}"]`)
          .getBoundingClientRect().top,
      }))
      .sort((a, b) => a.anchorY - b.anchorY),
  );
  ctx.check(
    "the AT and tank panels follow their units' vertical screen order",
    ordered.length === 2 &&
      ordered[0].anchorY < ordered[1].anchorY &&
      ordered[0].panelY < ordered[1].panelY,
    JSON.stringify(ordered),
  );
  await page.keyboard.up("Space");
  await page.mouse.move(20, 20);
  await page.setViewportSize({ width: 1280, height: 800 });
  await lab(page, () => window.__lab.route.select([0]));
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const priority = await page.locator("[data-testid=readouts] .ro-callout").evaluateAll((nodes) =>
    nodes.map((n) => ({
      selected: n.classList.contains("ro-selected"),
      layer: Number(getComputedStyle(n).zIndex) || 0,
    })),
  );
  ctx.check(
    "the selected readout paints above the other readouts",
    priority.length > 1 &&
      priority.filter((n) => n.selected).length === 1 &&
      priority.every(
        (n) =>
          !n.selected ||
          priority.filter((other) => !other.selected).every((other) => n.layer > other.layer),
      ),
    JSON.stringify(priority),
  );
  await lab(page, () => window.__lab.setCamera({ ...window.__lab.camera(), distance: 900 }));
  await page.evaluate(() => window.__lab.frame());
  const far = await rings(page);
  const panel = await page.getByTestId("selection-card").innerText();
  ctx.check(
    "zoomed out, rings stay only for the selection and the panel keeps the details",
    far
      .filter((d) => d.shown)
      .map((d) => d.unit)
      .join() === "0" &&
      /CANNON/.test(panel) &&
      /HMG/.test(panel),
    JSON.stringify({ shown: far.filter((d) => d.shown).map((d) => d.unit), panel }),
  );
  await shots(ctx, page, "far-1280x800");
  await lab(page, () => window.__lab.reset());

  // Keys (CommandBindings): F toggles the fire policy; X arms attack-move,
  // R a reverse move.
  await page.keyboard.press("f");
  await advance(page, 2);
  o = await obs(page);
  ctx.check(
    "F switches the selection's fire policy",
    o.own[0].engagement === "return_fire_only",
    o.own[0].engagement,
  );
  await page.keyboard.press("x");
  ctx.check(
    "X arms attack-move",
    (await lab(page, () => window.__lab.route.mode())) === "attack_move",
  );
  await page.keyboard.press("r");
  ctx.check(
    "R arms a reverse move",
    (await lab(page, () => window.__lab.route.mode())) === "reverse_move",
  );

  const mode = () => lab(page, () => window.__lab.route.mode());
  const lastAck = async () => (await lab(page, () => window.__lab.route.acks()))[0];
  await page.keyboard.press("Escape");
  ctx.check("Escape disarms", (await mode()) === "move");

  // A leader is one line everywhere: solid, one width, in its panel's
  // colour (the HUD accent for own units, selected or not); amber belongs to
  // the ground markers alone. Selection dims or lifts it with its panel.
  const want = game.presentation.hud.accent.map((v) => Math.round(v * 255));
  const leaders = await page.evaluate(() =>
    [...document.querySelectorAll(".ro-leader.ro-own")].map((l) => {
      const cs = getComputedStyle(l);
      return {
        selected: l.classList.contains("ro-selected"),
        stroke: cs.stroke,
        width: cs.strokeWidth,
        dash: cs.strokeDasharray,
        opacity: Number(cs.opacity),
      };
    }),
  );
  const sel = leaders.filter((l) => l.selected);
  const unsel = leaders.filter((l) => !l.selected);
  ctx.check(
    "every own leader line is one style: solid, one width, the callouts' cyan; selection only as the panel's opacity",
    sel.length > 0 &&
      unsel.length > 0 &&
      leaders.every(
        (l) =>
          l.stroke.startsWith(`rgba(${want.join(", ")}`) &&
          l.dash === "none" &&
          l.width === leaders[0].width,
      ) &&
      sel.every((l) => l.opacity === 1) &&
      unsel.every((l) => l.opacity < 1),
    JSON.stringify({ leaders, want }),
  );

  // Keys are ignored while typing in a control.
  await page.evaluate(() => {
    const input = document.createElement("input");
    input.id = "typing-probe";
    document.body.append(input);
    input.focus();
  });
  await page.keyboard.press("g");
  ctx.check("keys do nothing while typing", (await mode()) === "move");
  await page.evaluate(() => document.getElementById("typing-probe")?.remove());

  // G arms attack-ground for one right-click, then movement is the default again.
  await page.keyboard.press("g");
  const count = (await lab(page, () => window.__lab.route.acks())).length;
  const spot = await lab(page, () => window.__lab.projectToCss(300, 300, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, count, {
    timeout: 5000,
  });
  ack = await lastAck();
  ctx.check(
    "G then right-click attacks the ground, and the mode resets",
    /attack ground/.test(ack.label) && ack.ack.error === null && (await mode()) === "move",
    JSON.stringify(ack),
  );

  // Ctrl+right-click attack-moves at once, with nothing armed.
  const ctrlCount = (await lab(page, () => window.__lab.route.acks())).length;
  const there = await lab(page, () => window.__lab.projectToCss(260, 240, 0));
  await page.keyboard.down("Control");
  await page.mouse.click(there[0], there[1], { button: "right" });
  await page.keyboard.up("Control");
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, ctrlCount, {
    timeout: 5000,
  });
  ack = await lastAck();
  ctx.check(
    "Ctrl+right-click attack-moves the selection",
    ack.label.startsWith("attack-move") && ack.ack.error === null && (await mode()) === "move",
    JSON.stringify(ack),
  );

  // Reverse (Q31): R or X then a right-click, and the zone behind a single
  // selected vehicle.
  const rightClickAt = async (x, y, key) => {
    // The log keeps the newest eight: wait on the newest sequence number.
    const seq = (await lastAck())?.seq ?? 0;
    if (key) await page.keyboard.press(key);
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), [x, y]);
    await page.mouse.click(at[0], at[1], { button: "right" });
    await page.waitForFunction((k) => (window.__lab.route.acks()[0]?.seq ?? 0) > k, seq, {
      timeout: 5000,
    });
    // Past the double-click window, so the next right-click is a fresh move.
    await page.waitForTimeout(400);
    return lastAck();
  };
  const selectOnly = async (ids) => {
    await lab(page, (u) => window.__lab.route.select(u), ids);
    await page.waitForFunction((k) => window.__lab.route.selected().length === k, ids.length);
  };
  await selectOnly([0, 1]);
  ack = await rightClickAt(320, 320, "r");
  ctx.check(
    "R then right-click issues a reverse move",
    ack.label.startsWith("reverse move") && ack.ack.error === null && (await mode()) === "move",
    JSON.stringify(ack),
  );
  ack = await rightClickAt(330, 300, "x");
  ctx.check(
    "X then right-click issues an attack-move",
    ack.label.startsWith("attack-move") && ack.ack.error === null && (await mode()) === "move",
    JSON.stringify(ack),
  );
  // A point `back` metres behind the tank's centre along its hull (the
  // hull is 3.5 m long each way, so 12 m back is 8.5 m behind its rear).
  const tankNow = (await obs(page)).own.find((u) => u.id === 0);
  const behind = (back, left = 0) => [
    tankNow.position[0] - back * Math.cos(tankNow.yaw) - left * Math.sin(tankNow.yaw),
    tankNow.position[1] - back * Math.sin(tankNow.yaw) + left * Math.cos(tankNow.yaw),
  ];
  await selectOnly([0]);
  ack = await rightClickAt(...behind(12));
  ctx.check(
    "a right-click behind a single selected tank reverses",
    ack.label.startsWith("reverse move") && ack.ack.error === null,
    JSON.stringify(ack),
  );
  await selectOnly([0, 3]);
  ack = await rightClickAt(...behind(12));
  ctx.check(
    "the same click with two vehicles selected is a normal move",
    ack.label.startsWith("move ") && ack.ack.error === null,
    JSON.stringify(ack),
  );
  await selectOnly([0]);
  ack = await rightClickAt(...behind(12, -10));
  ctx.check(
    "a click outside the zone (10 m beside the strip) is a normal move",
    ack.label.startsWith("move ") && ack.ack.error === null,
    JSON.stringify(ack),
  );
  // Each button shows its key on a chip; its accessible name is the full wording.
  const bar = await page
    .getByRole("toolbar", { name: "Commands" })
    .evaluate((t) => [...t.querySelectorAll("button")].map((b) => b.ariaLabel).join("\n"));
  ctx.check(
    "the command bar names X for attack-move and R for reverse",
    /Attack-move \(X or Ctrl\+right-click\)/.test(bar) && /Reverse \(R,/.test(bar),
    bar,
  );

  const stopSeq = (await lastAck()).seq;
  await page.keyboard.press("Backspace");
  await page.waitForFunction((seq) => window.__lab.route.acks()[0]?.seq > seq, stopSeq);
  const stoppedAck = await lastAck();
  await advance(page, 2);
  const stopped = (await obs(page)).own.find((u) => u.id === tankNow.id);
  ctx.check(
    "Backspace stops the selection",
    stoppedAck.label.startsWith("stop") &&
      stoppedAck.ack.error === null &&
      !!stopped &&
      stopped.goal === null &&
      stopped.queue.length === 0,
    JSON.stringify({ ack: stoppedAck, goal: stopped?.goal, queue: stopped?.queue }),
  );

  // An attack reaches only armed units (`reach("attack")`): with a tank and
  // the unarmed supply truck selected, attack-move and attack-ground go to
  // the tank alone, and the truck keeps the move it was given.
  const truck = (await obs(page)).own.find((u) => u.kind === "supply");
  await selectOnly([truck.id]);
  ack = await rightClickAt(truck.position[0] + 20, truck.position[1], null);
  await advance(page, 2);
  const truckGoal = (await obs(page)).own.find((u) => u.id === truck.id).goal;
  await selectOnly([0, truck.id]);
  const truckName = `supply #${truck.id}`;
  const attackMove = await rightClickAt(330, 300, "x");
  const attackGround = await rightClickAt(300, 300, "g");
  await advance(page, 2);
  const truckAfter = (await obs(page)).own.find((u) => u.id === truck.id);
  ctx.check(
    "attack-move and attack-ground on a tank and a truck reach only the tank, and the truck keeps its move",
    [attackMove, attackGround].every(
      (a) => a.ack.error === null && a.label.includes("tank #0") && !a.label.includes(truckName),
    ) &&
      !!truckGoal &&
      JSON.stringify(truckAfter.goal) === JSON.stringify(truckGoal),
    JSON.stringify({ attackMove, attackGround, truckGoal, after: truckAfter.goal }),
  );

  // T deploys the supply truck, or packs it once it is deployed or deploying.
  await lab(page, (id) => window.__lab.route.select([id]), truck.id);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  // The bar shows only what the selection can do: the unarmed truck has no
  // attack or garrison tile, and one Deploy/Pack toggle; nothing selected,
  // no bar.
  const tiles = async () =>
    (await page.getByRole("toolbar", { name: "Commands" }).count())
      ? await page
          .getByRole("toolbar", { name: "Commands" })
          .evaluate((t) => [...t.querySelectorAll("button")].map((b) => b.ariaLabel.split(" (")[0]))
      : [];
  const truckTiles = await tiles();
  await lab(page, () => window.__lab.route.select([]));
  await page.waitForFunction(() => window.__lab.route.selected().length === 0);
  const none = await tiles();
  await lab(page, (id) => window.__lab.route.select([id]), truck.id);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  ctx.check(
    "the command bar shows only the selection's commands, one deploy toggle, and nothing with nothing selected",
    !truckTiles.some((t) => /Attack|Garrison|Leave/.test(t)) &&
      truckTiles.filter((t) => /^(Deploy|Pack)$/.test(t)).length === 1 &&
      truckTiles.includes("Stop") &&
      none.length === 0,
    JSON.stringify({ truckTiles, none }),
  );
  // Read now: the move above packed it.
  const heading = truckAfter.deployment.target;
  await page.keyboard.press("t");
  await page.waitForFunction(() => /^(deploy|pack) /.test(window.__lab.route.acks()[0].label));
  ack = await lastAck();
  ctx.check(
    "T deploys a packed truck and packs a deployed one",
    ack.label.startsWith(heading === "deployed" ? "pack " : "deploy ") && ack.ack.error === null,
    `${heading}: ${JSON.stringify(ack)}`,
  );
  await page.close();
  await vehicleMarker(ctx);
}

/** A vehicle's marker is painted on the ground, depth-tested
 *  like any ground mark. Framed the same way on open ground and with the
 *  lab's building between the camera and the tank parked behind it: on open
 *  ground the selection's ring shows round the hull; behind the building's
 *  corner it is hidden (it once drew over everything in front). */
async function vehicleMarker(ctx) {
  const page = await openBattle(ctx);
  const TANK = 0;
  const BUILDING = { center: [150, 110], half: 12 };
  // The tank's marker, selected: the selection's amber paint.
  await lab(page, (id) => window.__lab.route.select([id]), TANK);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  await lab(
    page,
    (id) =>
      window.__lab.route.command({
        kind: "set_engagement",
        units: [id],
        policy: "return_fire_only",
      }),
    TANK,
  );
  const radius =
    hull("tank").half_extents_m[0] + game.presentation.overlay.orders.vehicle_marker_margin_m;
  // Ground paint lies on the ground itself.
  const place = (target, yaw) =>
    lab(
      page,
      (c) =>
        window.__lab.setCamera({
          ...window.__lab.camera(),
          target: [c.target[0], c.target[1], 0],
          distance: 65,
          pitch: 0.85,
          yaw: c.yaw,
        }),
      { target, yaw },
    );
  // Frame `target` at the default camera's distance and pitch, with the
  // camera on the side `bearing` points to from it: of four yaws, the one
  // that draws a point 10 m that way lowest on screen.
  const frameFrom = async (target, bearing) => {
    let best = { yaw: 0, y: -Infinity };
    for (const yaw of [0, Math.PI / 2, Math.PI, -Math.PI / 2]) {
      await place(target, yaw);
      const near = [target[0] + Math.cos(bearing) * 10, target[1] + Math.sin(bearing) * 10];
      const p = await lab(page, (q) => window.__lab.projectToCss(q[0], q[1], 0), near);
      if (p && p[1] > best.y) best = { yaw, y: p[1] };
    }
    await place(target, best.yaw);
    await page.evaluate(() => window.__lab.frame());
  };
  // The share of the marker's rim painted, read as the paint's rise over the
  // ground.
  const rimShown = async (name) => {
    const tank = (await obs(page)).own.find((u) => u.id === TANK);
    await snapshot(ctx, page, `marker-${name}.png`);
    // Painted on the ground: the paint's rise over it.
    const png = await paintOnly(ctx, page, `marker-${name}`);
    // The brightest painted pixel round a sample (its light with the paint:
    // the ground under it plus the paint's rise), or 0 where none is.
    const painted = (x, y) => {
      let best = 0;
      for (let dy = -2; dy <= 2; dy++)
        for (let dx = -2; dx <= 2; dx++) {
          const [px, py] = [Math.round(x) + dx, Math.round(y) + dy];
          if (px < 0 || py < 0 || px >= png.width || py >= png.height) continue;
          const i = (py * png.width + px) * 4;
          const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
          const u = png.under.data;
          if (r + g + b > 45) best = Math.max(best, r + g + b + u[i] + u[i + 1] + u[i + 2]);
        }
      return best;
    };
    let shown = 0,
      light = 0;
    const N = 48;
    for (let k = 0; k < N; k++) {
      const a = (k / N) * 2 * Math.PI;
      const q = [tank.position[0] + Math.cos(a) * radius, tank.position[1] + Math.sin(a) * radius];
      const z = await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), q);
      const p = await lab(page, (w) => window.__lab.projectToCss(w[0], w[1], w[2]), [...q, z]);
      const lit = p ? painted(p[0], p[1]) : 0;
      if (lit > 0) [shown, light] = [shown + 1, light + lit];
    }
    return { shown: shown / N, light: light / Math.max(1, shown), at: tank.position };
  };
  // Open ground: the tank where it starts, framed from the south.
  const start = (await obs(page)).own.find((u) => u.id === TANK).position;
  await frameFrom(start, -Math.PI / 2);
  const open = await rimShown("open-ground");
  // Parked hard against the building's north wall, framed across its corner.
  const park = [BUILDING.center[0] + 6, BUILDING.center[1] + BUILDING.half + 3];
  await lab(
    page,
    (c) =>
      window.__lab.route.command({
        kind: "move",
        units: [c.id],
        gesture: 2701,
        goal: c.park,
        route: "shortest",
      }),
    { id: TANK, park },
  );
  const parked = await until(
    page,
    (o) => {
      const t = o.own.find((u) => u.id === TANK);
      return !!t && !t.goal && Math.hypot(t.position[0] - park[0], t.position[1] - park[1]) < 6;
    },
    1500,
    30,
  );
  const tank = parked?.own.find((u) => u.id === TANK);
  if (tank) await frameFrom(tank.position, -Math.PI / 2);
  const behind = tank ? await rimShown("behind-building") : { shown: 1, at: null };
  ctx.check(
    "a vehicle's marker shows round its hull on open ground and is hidden behind a building",
    open.shown >= 0.5 && behind.shown <= open.shown * 0.5,
    JSON.stringify({ open, behind, parked: !!tank }),
  );
  // Parked in the building's cast shadow (the sun is east of south-east,
  // `presentation.light.sun_azimuth`, so it falls west): the painted marker
  // is darker than in the open, lit like the ground, yet still drawn (its
  // emissive).
  const shadowed = [BUILDING.center[0] - BUILDING.half - 5, BUILDING.center[1]];
  await lab(
    page,
    (c) =>
      window.__lab.route.command({
        kind: "move",
        units: [c.id],
        gesture: 2702,
        goal: c.at,
        route: "shortest",
      }),
    { id: TANK, at: shadowed },
  );
  const inShade = (
    await until(
      page,
      (o) => {
        const t = o.own.find((u) => u.id === TANK);
        return (
          !!t && !t.goal && Math.hypot(t.position[0] - shadowed[0], t.position[1] - shadowed[1]) < 6
        );
      },
      1500,
      30,
    )
  )?.own.find((u) => u.id === TANK);
  if (inShade) await frameFrom(inShade.position, -Math.PI / 2);
  const shade = inShade ? await rimShown("in-shadow") : { shown: 0, light: Infinity, at: null };
  ctx.check(
    "a painted marker in a cast shadow is darker than in the sun, yet drawn",
    shade.shown >= 0.4 && shade.light < open.light * 0.99,
    JSON.stringify({ open, shade }),
  );
  await page.close();
}
