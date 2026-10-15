// The faction air matrix and the runtime models (slice 09). The panel lists,
// for each faction, the roster's short, medium and long air defence, its
// SEAD jets and its other fixed-wing aircraft, read from the cards as the
// catalog resolves them; every faction has all three AA tiers and a SEAD
// platform. The battle parks each of those types on the air field, and each
// is drawn; a contact sheet of them is left for review. High air is its own
// screen plane: a ground card laid under a jet's model never covers it, and
// crops of the line-up show the report's three cases (a jet over a SAM's
// card, two jets' cards, cards by the top edge).
import { gpuWarnings, aim, lab, hideHud, advance } from "./_lab.mjs";
import { encounter, openAir, shoot, drawnVehicles } from "./_air.mjs";
import { writeSheet } from "./_sheet.mjs";
import { decode, mostChanged, writeCrop } from "./_png.mjs";
import { layerOf } from "../../packages/scene-assets/src/units.ts";
import { unitType, weapons } from "./_units.mjs";
import { nodeCatalogSet } from "../src/battle/catalog/node.ts";
import { AIR_ROLES, airRole, FACTIONS } from "../../packages/scene-assets/src/units.ts";
const cards = (await nodeCatalogSet("test")).units.view.cards.filter((c) => !c.disabled_reason);
const matrix = Object.fromEntries(
  FACTIONS.map((f) => [
    f,
    Object.fromEntries(
      AIR_ROLES.map((r) => [
        r,
        cards
          .filter((c) => c.roster.factions.includes(f) && airRole(unitType(c.id)) === r)
          .map((c) => c.name),
      ]),
    ),
  ]),
);

/** The tick the line-up is shot at: past any tick the page pauses on. */
const LINEUP_TICK = 40;

export async function run(ctx) {
  await recordsPanel(ctx);
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const shown = await page.evaluate(() =>
    Object.fromEntries(
      [...document.querySelectorAll("[data-testid=air-matrix] [data-faction]")].map((s) => [
        s.getAttribute("data-faction"),
        Object.fromEntries(
          [...s.querySelectorAll("[data-role]")].map((r) => [
            r.getAttribute("data-role"),
            r.textContent.slice(r.textContent.indexOf(":") + 2),
          ]),
        ),
      ]),
    ),
  );
  for (const f of FACTIONS) {
    const want = Object.fromEntries(AIR_ROLES.map((r) => [r, matrix[f][r].join(", ") || "—"]));
    ctx.check(
      `${f}: the matrix lists the roster's air roles`,
      JSON.stringify(shown[f]) === JSON.stringify(want),
      JSON.stringify({ shown: shown[f], want }),
    );
    ctx.check(
      `${f}: short, medium and long air defence and a SEAD platform`,
      ["short", "medium", "long", "sead"].every((r) => matrix[f][r].length > 0),
      JSON.stringify(matrix[f]),
    );
  }

  // The line-up: one of each type in the matrix, every one drawn, at one
  // tick whenever the page paused, so its crops compare across runs.
  await advance(page, LINEUP_TICK - (await lab(page, () => window.__lab.route.tick())));
  const o = await lab(page, () => window.__lab.route.observation());
  const kinds = new Set(cards.filter((c) => airRole(unitType(c.id))).map((c) => c.id));
  ctx.check(
    "the battle parks each type of the matrix once",
    o.own.length === kinds.size && o.own.every((u) => kinds.has(u.kind)),
    `${o.own.length} units, ${kinds.size} types`,
  );
  const lineup = decode(await shoot(ctx, page, "lineup"));
  await reportCrops(ctx, page, o, lineup);
  await planeOrder(ctx, page, o);
  // The crops show the models alone: no HUD, cards or panel.
  await hideHud(page);
  await page.addStyleTag({ content: "[data-testid=air-matrix] { display: none !important; }" });
  const cells = [];
  const undrawn = [];
  for (const u of o.own) {
    await aim(
      page,
      u.position,
      { distance: 32, pitch: 0.45, yaw: -Math.PI / 2 - 0.6 },
      { onGround: false },
    );
    const drawn = await drawnVehicles(page);
    if (
      !drawn.some(
        (v) => Math.hypot(v.position[0] - u.position[0], v.position[1] - u.position[1]) < 2,
      )
    )
      undrawn.push(u.kind);
    await page.evaluate(() => window.__lab.frame());
    const png = await page.screenshot({ clip: { x: 340, y: 200, width: 600, height: 400 } });
    cells.push({ png, caption: `${unitType(u.kind).name} (${u.kind})` });
  }
  ctx.check("every parked type is drawn", undrawn.length === 0, undrawn.join(", "));
  await writeSheet(ctx, "contact-sheet.png", {
    title: "air-roster: the roster's air and air-defence types on their runtime models",
    columns: 5,
    cellWidth: 300,
    cells,
  });
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}

async function recordsPanel(ctx) {
  const page = await openAir(ctx, "records");
  const kinds = [...new Set(encounter("contracts", "air-roster").units.map((u) => u.kind))];
  // The panel: one record per fielded type, its fields the catalog's.
  const records = await page.evaluate(() =>
    [...document.querySelectorAll("[data-testid=air-records] [data-kind]")].map((s) => ({
      kind: s.getAttribute("data-kind"),
      fields: Object.fromEntries(
        [...s.querySelectorAll(".lab-row > span[data-field]")].map((f) => [
          f.getAttribute("data-field"),
          f.textContent.slice(f.getAttribute("data-field").length + 1),
        ]),
      ),
      weapons: Object.fromEntries(
        [...s.querySelectorAll("[data-weapon]")].map((w) => [
          w.getAttribute("data-weapon"),
          w.querySelector("[data-field=targets]").textContent,
        ]),
      ),
    })),
  );
  ctx.check(
    "the panel prints one record per fielded type",
    JSON.stringify(records.map((r) => r.kind).sort()) === JSON.stringify([...kinds].sort()),
    JSON.stringify(records.map((r) => r.kind)),
  );
  for (const kind of kinds) {
    const t = unitType(kind);
    const rec = records.find((r) => r.kind === kind);
    const caps = t.capabilities;
    const radar = caps.air?.radar ?? caps.radar;
    const want = {
      layer: layerOf(t),
      fuel: caps.air ? String(caps.air.fuel_capacity_s) : "—",
      aa_tier: caps.air_defense?.tier ?? "—",
      radar: radar ? `${radar.emits ? "emits" : "silent"}/${radar.detection_m}` : "—",
      sead: caps.air?.sead ? "yes" : "no",
    };
    const targets = Object.fromEntries(
      t.mounts
        .flatMap((m) => m.weapons)
        .map((w) => [w, Object.keys(weapons[w].targets ?? { ground: {} }).join("+")]),
    );
    ctx.check(
      `${kind}: its record reads the catalog's layer, fuel, tier, radar and weapon layers`,
      !!rec &&
        Object.entries(want).every(([k, v]) => rec.fields[k] === v) &&
        JSON.stringify(rec.weapons) === JSON.stringify(targets),
      JSON.stringify({ rec, want, targets }),
    );
  }

  await page.close();
}

const aloft = (u) => layerOf(unitType(u.kind)) === "high_air";
/** A unit's card anchor in page pixels: two metres over it. */
const anchorOf = (page, u) =>
  lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 2), u.position);

/** The line-up's crops of the report's three cases, chosen by where the
 *  units stand (not where their cards went), so a run before and after a
 *  change crops the same places. */
async function reportCrops(ctx, page, o, png) {
  const at = await Promise.all(o.own.map((u) => anchorOf(page, u)));
  const units = o.own.map((u, i) => ({ u, at: at[i] })).filter((p) => p.at);
  const jets = units.filter((p) => aloft(p.u));
  const ground = units.filter((p) => !aloft(p.u));
  const crop = (name, [x, y], hw, hh) =>
    writeCrop(png, ctx.evidencePath(`${name}.png`), x, y, hw, hh, 2);
  // A jet over a SAM's card: the jet nearest a ground unit's card spot, up
  // and to the side of that unit.
  let over = null;
  for (const j of jets)
    for (const g of ground) {
      const d = Math.hypot(j.at[0] - (g.at[0] + 70), j.at[1] - (g.at[1] - 40));
      if (!over || d < over.d) over = { d, at: j.at };
    }
  if (over) await crop("report-jet-over-card", over.at, 160, 80);
  // Two jets' cards on one spot: the two jets nearest each other.
  let pair = null;
  for (const [i, a] of jets.entries())
    for (const b of jets.slice(i + 1)) {
      const d = Math.hypot(a.at[0] - b.at[0], a.at[1] - b.at[1]);
      if (!pair || d < pair.d)
        pair = { d, at: [(a.at[0] + b.at[0]) / 2 + 60, (a.at[1] + b.at[1]) / 2 - 30] };
    }
  if (pair) await crop("report-two-jets", pair.at, 200, 80);
  // Cards at the top edge, beside the page's title plate.
  await crop("report-top-edge", [320, 70], 320, 70);
}

/** A ground card laid over a high-air model, in the ground and low-air
 *  plane: the model's pixels stay the model's, while the card shows where it
 *  is uncovered. */
async function planeOrder(ctx, page, o) {
  const jet = o.own.find(aloft);
  const ground = o.own.find((u) => layerOf(unitType(u.kind)) === "ground");
  await aim(page, jet.position, { distance: 240, pitch: 0.9 }, { onGround: false });
  await page.evaluate(() => window.__lab.frame());
  const model = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), jet.position);
  const plain = decode(await page.screenshot());
  // The ground unit's own card, twinned into its plane and centred on the jet.
  const corner = await lab(
    page,
    ({ id, at }) => {
      const card = document.querySelector(`[data-testid=readouts] .ro-unit[data-unit="${id}"]`);
      const twin = card.closest(".ro-callout").cloneNode(true);
      twin.dataset.twin = "";
      twin.style.zIndex = "999";
      twin.querySelector("path")?.setAttribute("d", "");
      const node = twin.querySelector(".ro-unit");
      node.style.display = "flex";
      const layer = document.querySelector("[data-testid=readouts]");
      (layer.querySelector(".ro-plane") ?? layer).appendChild(twin);
      const { width, height } = node.getBoundingClientRect();
      const [x, y] = [at[0] - width / 2, at[1] - height / 2];
      node.style.transform = `translate(${x}px, ${y}px)`;
      return [x + 4, y + 4];
    },
    { id: ground.id, at: model },
  );
  const covered = decode(await page.screenshot());
  await page.screenshot({ path: ctx.evidencePath("plane-order.png") });
  await page.evaluate(() => document.querySelector("[data-twin]")?.remove());
  const underModel = mostChanged(plain, covered, model, 2);
  const uncovered = mostChanged(plain, covered, corner, 3);
  ctx.check(
    "a ground card laid over a high-air model shows round it but never covers it",
    uncovered > 60 && underModel < 24,
    JSON.stringify({ model, underModel, corner, uncovered }),
  );
}
