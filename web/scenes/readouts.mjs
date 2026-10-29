// Slice 14: every displayed timer is the published one; completed timers
// vanish; one ring per weapon; ∞ for unlimited; guidance icon; no enemy
// readiness; the panel keeps details when zoomed out; commands and keys.
import { decode, pixel, writeCrop } from "./_png.mjs";
import { lab, obs, advance, snapshot, until, openBattle } from "./_lab.mjs";
import { paintOnly } from "./_overlays.mjs";
import { hull, unitType, weapons, visuals } from "./_units.mjs";

const escapeRegExp = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

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
  ctx.check(
    "clicking a floating panel selects its unit",
    (await lab(page, () => window.__lab.route.selected())).join(",") === "3",
  );
  await lab(page, () => window.__lab.route.select([]));
  await page.mouse.move(panelBox.x - 2, panelBox.y - 2);
  await page.mouse.down();
  await page.mouse.move(panelBox.x + panelBox.width + 2, panelBox.y + panelBox.height + 2, {
    steps: 4,
  });
  await page.mouse.up();
  await page.waitForFunction(() => window.__lab.route.selected().includes(3));
  ctx.check(
    "dragging a rectangle over a floating panel selects its unit",
    (await lab(page, () => window.__lab.route.selected())).includes(3),
  );
  await lab(page, () => window.__lab.route.select([]));
  await page.mouse.move(panelBox.x + 4, panelBox.y + 4);
  await page.mouse.down();
  await page.mouse.move(panelBox.x + panelBox.width + 2, panelBox.y + panelBox.height + 2, {
    steps: 4,
  });
  await page.mouse.up();
  await page.waitForFunction(() => window.__lab.route.selected().includes(3));
  ctx.check(
    "a selection drag can start inside a floating panel",
    (await lab(page, () => window.__lab.route.selected())).includes(3),
  );

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
    visuals.overlay.orders.area_draw_scale,
  );
  ctx.check(
    "the infantry picking probe is inside its circle and clear of bodies and panels",
    !!interior,
    JSON.stringify(interior),
  );
  if (interior) {
    await page.mouse.click(...interior);
    await page.waitForFunction(() => window.__lab.route.selected().join(",") === "2");
    ctx.check(
      "clicking empty ground inside the infantry circle selects the squad",
      (await lab(page, () => window.__lab.route.selected())).join(",") === "2",
    );
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
  // A mount of several kinds names each by its catalog label, then its count.
  const cannonKinds = new RegExp(
    `^${unitType(o.own.find((u) => u.id === 0).kind)
      .mounts[0].weapons.map((w) => `${escapeRegExp(weapons[w].name.toUpperCase())} \\d+`)
      .join(" · ")}$`,
  );
  ctx.check(
    "the cannon's AP and HE are one weapon row, and unlimited reads ∞",
    tank.mounts.length === 2 &&
      cannonKinds.test(tank.mounts[0].text) &&
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

  // A smaller viewport, then zoomed out: floating panels remain visible,
  // and the army deck's card keeps every detail.
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
      "div:has(> canvas):not([data-high-plane]) { background: rgb(100,100,100) !important; } canvas { visibility: hidden !important; }",
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
      .own.filter((u) => u.kind === "test_at" || u.kind === "test_tank")
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
  await lab(page, () => window.__lab.frame());
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
  await shots(ctx, page, "far-1280x800");
  // The army deck's card keeps the details at any zoom: hovering the tank's
  // card shows its whole panel, a row per weapon.
  await page.locator('.hud-army-card[data-unit="0"]').hover();
  const detail = await page.locator('.hud-army-detail[data-unit="0"]').evaluate((c) => ({
    name: c.querySelector(".ro-name-word")?.textContent,
    weapons: c.querySelectorAll(".ro-weapon").length,
  }));
  ctx.check(
    "zoomed out, compact panels remain visible and the tank's army card keeps the details",
    far.every((d) => d.shown) && (detail.name ?? "").endsWith("#0") && detail.weapons === 2,
    JSON.stringify({ shown: far.filter((d) => d.shown).map((d) => d.unit), detail }),
  );
  await shots(ctx, page, "far-card-detail-1280x800");

  // A card already selected is the camera's own focus: clicking it again
  // puts the orbit on the tank and keeps the angle and zoom N keeps.
  const cardTank = (await obs(page)).own.find((u) => u.id === 0);
  const beforeFocus = await lab(
    page,
    ([x, y]) => {
      const camera = { ...window.__lab.camera(), target: [x, y, 0] };
      window.__lab.setCamera(camera);
      return camera;
    },
    [cardTank.position[0] + 120, cardTank.position[1] + 120],
  );
  await page.locator('.hud-army-card[data-unit="0"]').click();
  await page.waitForFunction(
    ([x, y]) => {
      const target = window.__lab.camera().target;
      return Math.hypot(target[0] - x, target[1] - y) < 2;
    },
    [cardTank.position[0], cardTank.position[1]],
  );
  const focused = await lab(page, () => ({
    camera: window.__lab.camera(),
    asked: window.__lab.clearance().asked,
    selected: [...window.__lab.route.selected()],
  }));
  ctx.check(
    "clicking an already selected army card centres the camera on that unit, keeping the angle and zoom",
    Math.hypot(
      focused.camera.target[0] - cardTank.position[0],
      focused.camera.target[1] - cardTank.position[1],
    ) < 2 &&
      Math.abs(focused.asked.distance - beforeFocus.distance) < 1e-6 &&
      Math.abs(focused.asked.yaw - beforeFocus.yaw) < 1e-6 &&
      Math.abs(focused.asked.pitch - beforeFocus.pitch) < 1e-6 &&
      focused.selected.join() === "0",
    JSON.stringify({ before: beforeFocus, focused, tank: cardTank.position }),
  );

  // A second click is a new request, not an ignored repeat.
  await lab(
    page,
    ([x, y]) => {
      window.__lab.setCamera({ ...window.__lab.camera(), target: [x, y, 0] });
    },
    [cardTank.position[0] - 120, cardTank.position[1] + 60],
  );
  await page.locator('.hud-army-card[data-unit="0"]').click();
  await page.waitForFunction(
    ([x, y]) => {
      const target = window.__lab.camera().target;
      return Math.hypot(target[0] - x, target[1] - y) < 2;
    },
    [cardTank.position[0], cardTank.position[1]],
  );
  const again = await lab(page, () => window.__lab.camera());
  ctx.check(
    "clicking the same selected card again requests a fresh camera move",
    Math.hypot(again.target[0] - cardTank.position[0], again.target[1] - cardTank.position[1]) < 2,
    JSON.stringify({ again, tank: cardTank.position }),
  );

  // A double-click on an unselected card selects it on the first click and
  // centres on the second, so the same camera is reached from cold.
  await lab(page, () => window.__lab.route.select([]));
  const cardRifle = (await obs(page)).own.find((u) => u.id === 1);
  await lab(
    page,
    ([x, y]) => {
      window.__lab.setCamera({ ...window.__lab.camera(), target: [x, y, 0] });
    },
    [cardRifle.position[0] + 80, cardRifle.position[1] + 80],
  );
  await page.locator('.hud-army-card[data-unit="1"]').dblclick();
  await page.waitForFunction(
    ([x, y]) => {
      const target = window.__lab.camera().target;
      return Math.hypot(target[0] - x, target[1] - y) < 2;
    },
    [cardRifle.position[0], cardRifle.position[1]],
  );
  const doubleFocused = await lab(page, () => ({
    camera: window.__lab.camera(),
    selected: [...window.__lab.route.selected()],
  }));
  ctx.check(
    "double-clicking an unselected army card selects it and centres the camera on it",
    Math.hypot(
      doubleFocused.camera.target[0] - cardRifle.position[0],
      doubleFocused.camera.target[1] - cardRifle.position[1],
    ) < 2 && doubleFocused.selected.join() === "1",
    JSON.stringify({ doubleFocused, rifle: cardRifle.position }),
  );

  await page.mouse.move(20, 20);
  await lab(page, () => window.__lab.reset());

  // One real-keyboard smoke: a key produces one acknowledged order.
  const beforeKeys = await lab(page, () => window.__lab.route.acks().length);
  await page.keyboard.press("Backspace");
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, beforeKeys);
  const keyboardAck = await lab(page, () => window.__lab.route.acks()[0]);
  ctx.check(
    "one real key produces one acknowledged order",
    keyboardAck?.order.kind === "stop" && keyboardAck.ack.error === null,
    JSON.stringify(keyboardAck),
  );
  const bar = await page
    .getByRole("toolbar", { name: "Commands" })
    .evaluate((t) => [...t.querySelectorAll("button")].map((b) => b.ariaLabel).join("\n"));
  ctx.check(
    "the command bar names C for attack-move and R for reverse",
    /Attack-move \(C or Ctrl\+right-click\)/.test(bar) && /Reverse \(R,/.test(bar),
    bar,
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
    hull("test_tank").half_extents_m[0] + visuals.overlay.orders.vehicle_marker_margin_m;
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
