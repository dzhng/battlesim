// Slice 14: every displayed timer is the published one; completed timers
// vanish; one ring per weapon; ∞ for unlimited; guidance icon; no enemy
// readiness; the panel keeps details when zoomed out; commands and keys.
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, snapshot } from "./_lab.mjs";

/** The rings on screen, read back from the DOM. */
const rings = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("[data-testid=readouts] .ro-unit")].map((u) => ({
      unit: Number(u.dataset.unit),
      shown: u.style.display !== "none",
      mounts: [...u.querySelectorAll("svg[data-reason]")].map((s) => ({
        reason: s.dataset.reason,
        aim: s.dataset.aim === "" ? null : Number(s.dataset.aim),
        reload: s.dataset.reload === "" ? null : Number(s.dataset.reload),
        aimArc: !!s.querySelector(".ro-aim"),
        reloadArc: !!s.querySelector(".ro-reload"),
        text: s.querySelector(".ro-ammo")?.textContent,
        guide: !!s.querySelector(".ro-guide"),
      })),
      deploy: !!u.querySelector(".ro-deploy"),
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
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
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
  // Mid-engagement: aim, reload and deployment timers all running.
  let matched = true;
  let completedHidden = true;
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
        completedHidden &&= d.aimArc === (aim !== null) && d.reloadArc === (reload !== null);
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
  ctx.check("completed timers disappear", completedHidden);
  ctx.check("a guiding launcher shows the guidance icon", guided);
  ctx.check("an aim ring was captured", aimShot);
  const drawn = await rings(page);
  const tank = drawn.find((d) => d.unit === 0);
  const rifle = drawn.find((d) => d.unit === 2);
  ctx.check(
    "the cannon's AP and HE are one weapon ring, and unlimited reads ∞",
    tank.mounts.length === 2 &&
      /^(AP|HE)\d+$/.test(tank.mounts[0].text) &&
      rifle.mounts[0].text === "∞",
    JSON.stringify({
      tank: tank.mounts.map((m) => m.text),
      rifle: rifle.mounts.map((m) => m.text),
    }),
  );
  ctx.check(
    "the supply truck has a separate deployment readout",
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
  await page.setViewportSize({ width: 1280, height: 800 });
  await lab(page, () => window.__lab.route.select([0]));
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  await lab(page, () => window.__lab.setCamera({ ...window.__lab.camera(), distance: 900 }));
  await page.evaluate(() => window.__lab.frame());
  const far = await rings(page);
  const panel = await page.getByTestId("selection-panel").innerText();
  ctx.check(
    "zoomed out, rings stay only for the selection and the panel keeps the details",
    far
      .filter((d) => d.shown)
      .map((d) => d.unit)
      .join() === "0" &&
      /cannon/.test(panel) &&
      /HMG/.test(panel),
    JSON.stringify({ shown: far.filter((d) => d.shown).map((d) => d.unit), panel }),
  );
  await shots(ctx, page, "far-1280x800");
  await lab(page, () => window.__lab.reset());

  // Keys (CommandBindings): F toggles the fire policy; X arms attack-move,
  // R a reverse move (slice 39).
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
  const bar = await page.getByRole("toolbar", { name: "Commands" }).innerText();
  ctx.check(
    "the command bar names X for attack-move and R for reverse",
    /Attack-move \(X or Ctrl\+right-click\)/.test(bar) && /Reverse \(R,/.test(bar),
    bar,
  );

  await page.keyboard.press("Backspace");
  await page.waitForFunction(() => /^stop/.test(window.__lab.route.acks()[0].label));
  ctx.check("Backspace stops the selection", true);

  // T deploys the supply truck, or packs it once it is deployed or deploying.
  const truck = (await obs(page)).own.find((u) => u.kind === "supply");
  await lab(page, (id) => window.__lab.route.select([id]), truck.id);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const heading = truck.deployment.target;
  await page.keyboard.press("t");
  await page.waitForFunction(() => /^(deploy|pack) /.test(window.__lab.route.acks()[0].label));
  ack = await lastAck();
  ctx.check(
    "T deploys a packed truck and packs a deployed one",
    ack.label.startsWith(heading === "deployed" ? "pack " : "deploy ") && ack.ack.error === null,
    `${heading}: ${JSON.stringify(ack)}`,
  );
}
